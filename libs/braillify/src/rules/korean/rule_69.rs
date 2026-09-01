use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::char_struct::CharType;
use crate::rules::RuleMeta;
use crate::rules::context::RuleContext;
use crate::rules::english_ueb::span::encode_korean_word;
use crate::rules::korean::rule_29::{ENGLISH_CONTINUATION, ROMAN_INDICATOR};
use crate::rules::traits::{BrailleRule, Phase, RuleResult};
use unicode_normalization::UnicodeNormalization;

pub static META: RuleMeta = RuleMeta {
    section: "69",
    subsection: None,
    name: "measurement_symbols",
    standard_ref: "2024 Korean Braille Standard, Ch.6 Art.69",
    description: "Measurement and scientific unit symbols",
};

const SINGLE_MAPPINGS: &[(char, &str)] = &[
    ('Ω', "⠴⠠⠨⠺⠲"),
    ('%', "⠴⠏"),
    ('‰', "⠴⠏⠍"),
    ('°', "⠴⠙"),
    ('℃', "⠴⠙⠠⠉"),
    ('℉', "⠴⠙⠠⠋"),
    ('′', "⠴⠤"),
    ('″', "⠴⠤⠤"),
    ('Å', "⠴⠡"),
];

const ASCII_UNIT_MAPPINGS: &[(&str, &str)] = &[
    ("cm", "⠴⠉⠍⠲"),
    ("kg", "⠴⠅⠛⠲"),
    ("in", "⠴⠊⠝⠲"),
    ("mm", "⠴⠍⠍⠲"),
    ("min", "⠍⠔⠲"),
    ("cal", "⠴⠉⠁⠇⠲"),
    ("GB", "⠴⠠⠠⠛⠃⠲"),
    ("m", "⠴⠍⠲"),
    ("h", "⠴⠓⠲"),
];

const PERCENT_ABBREVIATION_MAPPINGS: &[(&str, &str)] = &[("%ile", "⠴⠏⠞"), ("%p", "⠴⠏⠏")];

const SEPARATED_SYMBOLS: &[char] = &['%', '‰', '°', '℃', '℉'];

fn encode_unicode_cells(unicode: &str) -> Vec<u8> {
    unicode
        .chars()
        .map(crate::unicode::decode_unicode)
        .collect()
}

/// Unicode's CJK compatibility block contains square presentation forms for
/// Roman unit symbols (`㎏` → `kg`, `㎓` → `GHz`, `㎥` → `m3`). Rules 68/69
/// define the transcription from the semantic Roman unit, so recognize the
/// whole Unicode family from its compatibility decomposition instead of adding
/// one input-specific mapping per glyph. Japanese square words and other CJK
/// compatibility characters are rejected by the component grammar.
fn compatibility_unit_decomposition(c: char) -> Option<Vec<char>> {
    // Unicode CJK Compatibility contains non-unit square abbreviations too
    // (`㏑` ln, `㏒` log, `㏚` PR). Keep the accepted ranges to scientific and
    // measurement symbols; the component grammar is an additional guard, not
    // the sole evidence that a square abbreviation is a unit.
    let is_unit_codepoint = matches!(
        c as u32,
        0x3371..=0x337a
            | 0x3380..=0x33c6
            | 0x33c8..=0x33cc
            | 0x33ce..=0x33d0
            | 0x33d3..=0x33d9
            | 0x33db..=0x33df
            | 0x33ff
    );
    if !is_unit_codepoint || super::rule_68::is_rule_68_symbol(c) {
        return None;
    }
    let parts = c.to_string().nfkc().collect::<Vec<_>>();
    (parts.iter().any(|part| part.is_ascii_alphabetic())
        && parts.iter().all(|part| {
            part.is_ascii_alphabetic()
                || matches!(part, '2' | '3' | '/' | '\u{2044}' | '\u{2215}' | 'μ')
        }))
    .then_some(parts)
}

/// Rule 69 delegates only to rule 37's multi-letter groupsigns. This is not
/// ordinary UEB word encoding: whole-word signs and shortforms are disabled,
/// and a lower groupsign cannot consume the whole entry run (`in` is spelled
/// `i`-`n`, while the same `in` may contract inside `min`).
fn encode_rule_69_unit_letters(letters: &[char]) -> Result<Vec<u8>, String> {
    encode_korean_word(letters, false, false, true, false).ok_or_else(|| {
        format!(
            "cannot encode rule 69 Roman unit letters: {}",
            letters.iter().collect::<String>()
        )
    })
}

fn encode_compatibility_unit(
    parts: &[char],
    needs_roman_indicator: bool,
    needs_roman_terminator: bool,
) -> Result<Vec<u8>, String> {
    let mut encoded = Vec::new();
    if needs_roman_indicator {
        encoded.push(ROMAN_INDICATOR);
    }

    let mut index = 0usize;
    while index < parts.len() {
        match parts[index] {
            'μ' => {
                encoded.extend(encode_unicode_cells("⠨⠍"));
                index += 1;
            }
            '2' | '3' => {
                encoded.extend(encode_unicode_cells("⠘⠼"));
                encoded.push(crate::number::encode_number(parts[index])?);
                index += 1;
            }
            '/' | '\u{2044}' | '\u{2215}' => {
                encoded.extend(encode_unicode_cells("⠸⠌"));
                index += 1;
            }
            ch if ch.is_ascii_alphabetic() => {
                let end = index
                    + parts[index..]
                        .iter()
                        .take_while(|part| part.is_ascii_alphabetic())
                        .count();
                let letters = &parts[index..end];
                let unit = encode_rule_69_unit_letters(letters)?;
                encoded.extend(unit);
                index = end;
            }
            unsupported => {
                return Err(format!(
                    "unsupported compatibility unit component: U+{:04X}",
                    unsupported as u32
                ));
            }
        }
    }

    // Rule 68's superscript closes the compact unit without a Roman terminator
    // (`㎡` → `0m^#b`). Otherwise rule 69 terminates the Roman unit unless the
    // same Roman unit chain continues through a slash.
    if needs_roman_terminator && !matches!(parts.last(), Some('2' | '3')) {
        encoded.push(crate::unicode::decode_unicode('⠲'));
    }
    Ok(encoded)
}

fn is_roman_unit_component(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == 'μ' || compatibility_unit_decomposition(ch).is_some()
}

fn roman_unit_chain_continues_before(ctx: &RuleContext) -> bool {
    ctx.index >= 2
        && ctx.word_chars.get(ctx.index - 1) == Some(&'/')
        && ctx
            .word_chars
            .get(ctx.index - 2)
            .is_some_and(|previous| is_roman_unit_component(*previous))
}

fn roman_unit_chain_continues_after(ctx: &RuleContext) -> bool {
    ctx.word_chars.get(ctx.index + 1) == Some(&'/')
        && ctx
            .word_chars
            .get(ctx.index + 2)
            .is_some_and(|next| is_roman_unit_component(*next))
}

pub fn is_rule_69_symbol(c: char) -> bool {
    SINGLE_MAPPINGS.iter().any(|(candidate, _)| *candidate == c)
        || c == 'μ'
        || compatibility_unit_decomposition(c).is_some()
}

fn is_numeric_or_unit_context(ctx: &RuleContext) -> bool {
    ctx.prev_char()
        .is_some_and(|prev| prev.is_ascii_digit() || matches!(prev, '/' | 'μ'))
        || ctx.prev_word.chars().next().is_some()
            && ctx
                .prev_word
                .chars()
                .all(|ch| ch.is_ascii_digit() || matches!(ch, ',' | '.'))
        || ctx.prev_char() == Some('/')
}

/// 단어 자체가 단위 연쇄(cal/㎠/min 등)로 구성된 경우 첫 음절이 한국어 뒤에 와도
/// 단위로 해석한다. 단위 연쇄의 특징: 단어 내에 `/`가 있거나 제69항 단위 기호(㎠, ㎏ 등)가
/// 섞여 있다.
fn word_looks_like_unit_chain(word: &[char]) -> bool {
    let mut has_separator = false;
    let mut has_unit_symbol = false;
    for c in word {
        if *c == '/' {
            has_separator = true;
        } else if is_rule_69_symbol(*c) || *c == 'μ' {
            has_unit_symbol = true;
        }
    }
    let has_ascii_letter = word.iter().any(char::is_ascii_alphabetic);
    has_separator && (has_unit_symbol || has_ascii_letter)
}

fn is_symbol_measurement_context(ctx: &RuleContext, symbol: char) -> bool {
    match symbol {
        'μ' => {
            ctx.next_char().is_some_and(|ch| ch.is_ascii_alphabetic())
                || is_numeric_or_unit_context(ctx)
        }
        'Ω' => {
            ctx.next_char().is_some_and(crate::utils::is_korean_char)
                || is_numeric_or_unit_context(ctx)
        }
        _ => true,
    }
}

/// Check whether `tail` starts with the ASCII-only string `s` (char-by-char).
/// All entries in `ASCII_UNIT_MAPPINGS` are ASCII, so byte length and char count
/// coincide; we avoid materializing `tail` into a `String` on the hot path.
fn chars_start_with_ascii(tail: &[char], s: &str) -> bool {
    if tail.len() < s.len() {
        return false;
    }
    s.bytes().zip(tail.iter()).all(|(b, c)| (b as char) == *c)
}

/// ASCII spellings that are canonically exposed by the same Unicode
/// compatibility-unit family already accepted above. This derives the unit
/// lexicon from semantic unit code points instead of maintaining a second
/// corpus-shaped list (`㎞` -> `km`, `㎎` -> `mg`, `㎾` -> `kW`, ...).
fn compatibility_ascii_unit_candidate(glyph: char) -> Option<(String, Vec<u8>)> {
    let parts = glyph.to_string().nfkc().collect::<Vec<_>>();
    if !parts.iter().all(char::is_ascii_alphabetic) {
        return None;
    }
    let encoded = if compatibility_unit_decomposition(glyph).is_some() {
        encode_compatibility_unit(&parts, true, true).ok()?
    } else {
        super::rule_68::encode_rule_68_symbol(glyph)?
    };
    Some((parts.into_iter().collect(), encoded))
}

fn compatibility_ascii_unit_owners() -> BTreeMap<String, Vec<(char, Vec<u8>)>> {
    let mut by_spelling = BTreeMap::<String, Vec<(char, Vec<u8>)>>::new();
    for glyph in (0x3300..=0x33ff).filter_map(char::from_u32) {
        if let Some((spelling, encoded)) = compatibility_ascii_unit_candidate(glyph) {
            by_spelling
                .entry(spelling)
                .or_default()
                .push((glyph, encoded));
        }
    }
    by_spelling
}

fn retain_unambiguous_ascii_unit_spellings(
    owners_by_spelling: BTreeMap<String, Vec<(char, Vec<u8>)>>,
) -> Vec<(String, Vec<u8>)> {
    let mut spellings = owners_by_spelling
        .into_iter()
        .filter_map(|(spelling, owners)| {
            let first = &owners.first()?.1;
            owners
                .iter()
                .all(|(_, encoded)| encoded == first)
                .then(|| (spelling, first.clone()))
        })
        .collect::<Vec<_>>();
    spellings.sort_by(|left, right| {
        right
            .0
            .len()
            .cmp(&left.0.len())
            .then_with(|| left.0.cmp(&right.0))
    });
    spellings
}

fn compatibility_ascii_unit_spellings() -> &'static [(String, Vec<u8>)] {
    static SPELLINGS: OnceLock<Vec<(String, Vec<u8>)>> = OnceLock::new();
    SPELLINGS
        .get_or_init(|| retain_unambiguous_ascii_unit_spellings(compatibility_ascii_unit_owners()))
}

pub(crate) fn encode_ascii_unit(word: &[char], index: usize) -> Option<(Vec<u8>, usize)> {
    let tail = &word[index..];
    ASCII_UNIT_MAPPINGS
        .iter()
        .filter(|(unit, _)| chars_start_with_ascii(tail, unit))
        .max_by_key(|(unit, _)| unit.len())
        .map(|(unit, unicode)| (encode_unicode_cells(unicode), unit.len()))
}

/// Numeric-compact Rule 69 path. Compatibility-derived spellings are limited
/// to this measured boundary so an unrelated English word after a separated
/// number cannot become a unit merely because it starts with a unit spelling.
fn encode_numeric_ascii_unit(word: &[char], index: usize) -> Option<(Vec<u8>, usize)> {
    let tail = &word[index..];
    let explicit = encode_ascii_unit(word, index);
    let derived = compatibility_ascii_unit_spellings()
        .iter()
        .filter(|(unit, _)| chars_start_with_ascii(tail, unit))
        .max_by_key(|(unit, _)| unit.len());

    if let Some((encoded, consumed)) = explicit {
        match derived {
            Some((candidate, derived_encoded)) if consumed == candidate.len() => {
                return (encoded.as_slice() == derived_encoded.as_slice())
                    .then_some((encoded, consumed));
            }
            Some((candidate, _)) if consumed < candidate.len() => {}
            _ => return Some((encoded, consumed)),
        }
    }

    let (unit, encoded) = derived?;
    Some((encoded.clone(), unit.len()))
}

fn encode_percent_abbreviation(word: &[char], index: usize) -> Option<(Vec<u8>, usize)> {
    let tail = &word[index..];
    for (abbr, unicode) in PERCENT_ABBREVIATION_MAPPINGS {
        if !chars_start_with_ascii(tail, abbr) {
            continue;
        }
        if *abbr == "%p"
            && tail
                .get(abbr.len())
                .is_some_and(|ch| ch.is_ascii_alphabetic())
        {
            continue;
        }
        return Some((encode_unicode_cells(unicode), abbr.len()));
    }
    None
}

pub(crate) fn parse_numeric_ascii_unit_prefix(word: &[char]) -> Option<(String, Vec<u8>, usize)> {
    let numeric_len = word
        .iter()
        .take_while(|c| c.is_ascii_digit() || matches!(**c, ',' | '.'))
        .count();
    if numeric_len == 0 || numeric_len >= word.len() {
        return None;
    }

    let numeric = word[..numeric_len].iter().collect::<String>();
    let (unit, consumed) = encode_numeric_ascii_unit(word, numeric_len)?;
    if word
        .get(numeric_len + consumed)
        .is_some_and(|ch| ch.is_ascii_alphabetic())
    {
        return None;
    }
    Some((numeric, unit, numeric_len + consumed))
}

fn trim_recent_english_indicator(result: &mut Vec<u8>) {
    if result
        .last()
        .is_some_and(|cell| matches!(*cell, ENGLISH_CONTINUATION | ROMAN_INDICATOR))
    {
        result.pop();
    }
}

/// Rules 33/34 override rule 69's ordinary trailing Roman terminator when a
/// listed Korean punctuation mark or an enclosing mark closes the Roman run.
/// Unit encoders include their ordinary terminator so standalone/end/Korean
/// boundaries stay unchanged; this helper applies only at the actual following
/// input boundary.
fn omit_roman_terminator_before_boundary(
    encoded: &mut Vec<u8>,
    word: &[char],
    boundary_index: usize,
) {
    let skips_for_punctuation = word
        .get(boundary_index)
        .is_some_and(|symbol| crate::english_logic::should_skip_terminator_for_symbol(*symbol));
    let continues_through_slash = word.get(boundary_index) == Some(&'/')
        && word
            .get(boundary_index + 1)
            .is_some_and(|next| is_roman_unit_component(*next));
    if (skips_for_punctuation || continues_through_slash)
        && encoded.last() == Some(&crate::unicode::decode_unicode('⠲'))
    {
        encoded.pop();
    }
}

fn should_insert_separator_after_symbol(symbol: char, next: Option<char>) -> bool {
    SEPARATED_SYMBOLS.contains(&symbol) && next.is_some_and(crate::utils::is_korean_char)
}

pub struct Rule69;

impl BrailleRule for Rule69 {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn phase(&self) -> Phase {
        Phase::CoreEncoding
    }

    fn priority(&self) -> u16 {
        90
    }

    fn matches(&self, ctx: &RuleContext) -> bool {
        matches!(ctx.char_type, CharType::Symbol(c) if is_rule_69_symbol(*c) && is_symbol_measurement_context(ctx, *c))
            || matches!(ctx.char_type, CharType::Number(_)
                if ctx.index == 0 && parse_numeric_ascii_unit_prefix(ctx.word_chars).is_some())
            || matches!(ctx.char_type, CharType::English(_)
                if (is_numeric_or_unit_context(ctx)
                    || (ctx.index == 0 && word_looks_like_unit_chain(ctx.word_chars)))
                    && encode_ascii_unit(ctx.word_chars, ctx.index).is_some())
    }

    fn apply(&self, ctx: &mut RuleContext) -> Result<RuleResult, String> {
        if matches!(ctx.char_type, CharType::Number(_))
            && ctx.index == 0
            && let Some((numeric, mut unit, consumed)) =
                parse_numeric_ascii_unit_prefix(ctx.word_chars)
        {
            omit_roman_terminator_before_boundary(&mut unit, ctx.word_chars, consumed);
            let mut encoded = crate::encode(&numeric)?;
            encoded.extend(unit);
            ctx.emit_slice(&encoded);
            ctx.state.is_english = false;
            ctx.state.needs_english_continuation = false;
            *ctx.skip_count = consumed.saturating_sub(1);
            return Ok(RuleResult::Consumed);
        }

        if matches!(ctx.char_type, CharType::English(_))
            && (is_numeric_or_unit_context(ctx)
                || (ctx.index == 0 && word_looks_like_unit_chain(ctx.word_chars)))
            && let Some((mut encoded, consumed)) = encode_ascii_unit(ctx.word_chars, ctx.index)
        {
            omit_roman_terminator_before_boundary(
                &mut encoded,
                ctx.word_chars,
                ctx.index + consumed,
            );
            trim_recent_english_indicator(ctx.result);
            ctx.emit_slice(&encoded);
            ctx.state.is_english = false;
            ctx.state.needs_english_continuation = false;
            *ctx.skip_count = consumed.saturating_sub(1);
            return Ok(RuleResult::Consumed);
        }

        if ctx.current_char() == '%'
            && let Some((encoded, consumed)) =
                encode_percent_abbreviation(ctx.word_chars, ctx.index)
        {
            ctx.emit_slice(&encoded);
            *ctx.skip_count = consumed.saturating_sub(1);
            if ctx
                .word_chars
                .get(ctx.index + consumed)
                .is_some_and(|ch| crate::utils::is_korean_char(*ch))
            {
                ctx.emit(0);
            }
            return Ok(RuleResult::Consumed);
        }

        if ctx.current_char() == 'μ' {
            trim_recent_english_indicator(ctx.result);
            let mut encoded = encode_unicode_cells("⠴⠨⠍");
            let mut consumed = 1usize;

            if let Some((unit_encoded, unit_len)) = encode_ascii_unit(ctx.word_chars, ctx.index + 1)
            {
                let mut unit_without_prefix = unit_encoded;
                if unit_without_prefix.first() == Some(&crate::unicode::decode_unicode('⠴')) {
                    unit_without_prefix.remove(0);
                }
                encoded.extend(unit_without_prefix);
                consumed += unit_len;
            } else {
                encoded.extend(encode_unicode_cells("⠍"));
            }

            omit_roman_terminator_before_boundary(
                &mut encoded,
                ctx.word_chars,
                ctx.index + consumed,
            );

            ctx.emit_slice(&encoded);
            ctx.state.is_english = false;
            ctx.state.needs_english_continuation = false;
            *ctx.skip_count = consumed.saturating_sub(1);
            return Ok(RuleResult::Consumed);
        }

        if let Some(parts) = compatibility_unit_decomposition(ctx.current_char()) {
            let continues_from_previous = roman_unit_chain_continues_before(ctx);
            let continues_after = roman_unit_chain_continues_after(ctx);
            let mut encoded =
                encode_compatibility_unit(&parts, !continues_from_previous, !continues_after)?;
            omit_roman_terminator_before_boundary(&mut encoded, ctx.word_chars, ctx.index + 1);
            ctx.emit_slice(&encoded);
            ctx.state.is_english = false;
            ctx.state.needs_english_continuation = false;
            return Ok(RuleResult::Consumed);
        }

        // `matches()` guard `is_rule_69_symbol(c)` is a `SINGLE_MAPPINGS` lookup,
        // so reaching here without the prior μ/ASCII-unit/`%`-shortcut paths
        // means the char is guaranteed to be in `SINGLE_MAPPINGS`.
        let (_, unicode) = SINGLE_MAPPINGS
            .iter()
            .find(|(candidate, _)| *candidate == ctx.current_char())
            .expect("matches() guarantees the char is in SINGLE_MAPPINGS");
        let mut encoded = encode_unicode_cells(unicode);
        omit_roman_terminator_before_boundary(&mut encoded, ctx.word_chars, ctx.index + 1);
        ctx.emit_slice(&encoded);
        if should_insert_separator_after_symbol(ctx.current_char(), ctx.next_char()) {
            ctx.emit(0);
        }
        Ok(RuleResult::Consumed)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Rule69, compatibility_ascii_unit_owners, compatibility_unit_decomposition,
        encode_ascii_unit, encode_compatibility_unit, encode_numeric_ascii_unit,
        encode_percent_abbreviation, encode_rule_69_unit_letters, encode_unicode_cells,
        omit_roman_terminator_before_boundary, parse_numeric_ascii_unit_prefix,
        retain_unambiguous_ascii_unit_spellings, word_looks_like_unit_chain,
    };

    #[rstest::rstest]
    #[case::slash_with_ascii_unit("cal/min", true)]
    #[case::slash_with_unit_symbol("kg/㎠", true)]
    #[case::slash_without_unit_component("//", false)]
    #[case::unit_symbol_without_slash("㎠", false)]
    fn detects_unit_chain_words(#[case] input: &str, #[case] expected: bool) {
        let chars: Vec<char> = input.chars().collect();

        assert_eq!(word_looks_like_unit_chain(&chars), expected);
    }

    #[rstest::rstest]
    #[case::kilogram('㎏', "kg")]
    #[case::gigahertz('㎓', "GHz")]
    #[case::cubic_metre('㎥', "m3")]
    #[case::metres_per_second('㎧', "m∕s")]
    #[case::milliwatt('㎽', "mW")]
    #[case::kilowatt('㎾', "kW")]
    #[case::sievert('㏜', "Sv")]
    fn decomposes_compatibility_unit_symbols(#[case] input: char, #[case] expected: &str) {
        assert_eq!(
            compatibility_unit_decomposition(input),
            Some(expected.chars().collect())
        );
    }

    /// Unicode CJK Compatibility names distinguish the accepted SQUARE IU
    /// (U+337A) from non-unit square abbreviations LN, LOG, and PR. In
    /// particular, U+33DA is SQUARE PR, not SQUARE IU.
    #[rstest::rstest]
    #[case::international_unit('㍺', Some("IU"))]
    #[case::natural_logarithm('㏑', None)]
    #[case::logarithm('㏒', None)]
    #[case::public_relations('㏚', None)]
    fn accepts_only_unit_semantics(#[case] input: char, #[case] expected: Option<&str>) {
        assert_eq!(
            compatibility_unit_decomposition(input),
            expected.map(|text| text.chars().collect())
        );
    }

    const ACCEPTED_GLYPHS: &str = "㍱㍲㍳㍴㍵㍶㍷㍸㍹㍺㎀㎁㎂㎃㎄㎅㎆㎇㎈㎉㎊㎋㎌㎍㎎㎏㎐㎑㎒㎓㎔㎕㎖㎗㎘㎙㎚㎛㎜㎝㎞㎟㎠㎢㎣㎤㎥㎦㎧㎨㎩㎪㎫㎬㎭㎮㎯㎰㎱㎲㎳㎴㎵㎶㎷㎸㎹㎺㎻㎼㎽㎾㎿㏃㏄㏅㏆㏈㏉㏋㏌㏎㏏㏐㏓㏔㏕㏖㏗㏙㏛㏜㏝㏞㏟㏿";

    #[test]
    fn accepted_compatibility_unit_set_is_stable() {
        let actual = (0x3300..=0x33ff)
            .filter_map(char::from_u32)
            .filter(|ch| compatibility_unit_decomposition(*ch).is_some())
            .collect::<String>();

        assert_eq!(actual, ACCEPTED_GLYPHS);
    }

    #[test]
    fn every_accepted_compatibility_unit_encodes_without_panicking() {
        // Generated property check: the set identity is asserted separately,
        // while this loop only proves that every accepted decomposition and
        // each of its ASCII letter runs reaches the fallible Rule 69 encoder.
        for glyph in ACCEPTED_GLYPHS.chars() {
            let parts = compatibility_unit_decomposition(glyph).unwrap();
            let mut index = 0usize;
            while index < parts.len() {
                if !parts[index].is_ascii_alphabetic() {
                    index += 1;
                    continue;
                }
                let end = index
                    + parts[index..]
                        .iter()
                        .take_while(|part| part.is_ascii_alphabetic())
                        .count();
                encode_rule_69_unit_letters(&parts[index..end]).unwrap();
                index = end;
            }
            encode_compatibility_unit(&parts, true, true).unwrap();
        }
    }

    #[test]
    fn every_rule_68_or_69_ascii_derivation_matches_every_owner_glyph() {
        for (spelling, owners) in compatibility_ascii_unit_owners() {
            let first = &owners[0].1;
            for (glyph, owner_encoding) in &owners {
                assert_eq!(
                    owner_encoding, first,
                    "conflicting owner cells for NFKC spelling {spelling:?}: U+{:04X}",
                    *glyph as u32
                );

                let chars = spelling.chars().collect::<Vec<_>>();
                let (derived, consumed) = encode_numeric_ascii_unit(&chars, 0)
                    .unwrap_or_else(|| {
                        panic!(
                            "unambiguous ASCII compatibility-unit spelling {spelling:?} from U+{:04X} must be recognized",
                            *glyph as u32
                        )
                    });
                assert_eq!(consumed, spelling.len(), "partial match for {spelling}");
                assert_eq!(
                    &derived, owner_encoding,
                    "derived cells differ from owner U+{:04X} for {spelling}",
                    *glyph as u32
                );

                let ascii_input = format!("값은 1{spelling}이다");
                let glyph_input = format!("값은 1{glyph}이다");
                assert_eq!(
                    crate::encode_to_unicode(&ascii_input).unwrap(),
                    crate::encode_to_unicode(&glyph_input).unwrap(),
                    "full encoder differs for {spelling} and owner U+{:04X}",
                    *glyph as u32
                );
            }
        }
    }

    #[test]
    fn conflicting_nfkc_owner_cells_are_excluded_instead_of_first_wins() {
        let owners = std::collections::BTreeMap::from([
            (
                "safe".to_string(),
                vec![('A', vec![1, 2]), ('B', vec![1, 2])],
            ),
            ("conflict".to_string(), vec![('C', vec![3]), ('D', vec![4])]),
        ]);

        let resolved = retain_unambiguous_ascii_unit_spellings(owners);

        assert!(resolved.iter().any(|(spelling, _)| spelling == "safe"));
        assert!(resolved.iter().all(|(spelling, _)| spelling != "conflict"));
    }

    #[rstest::rstest]
    #[case::kilometre("80km", "80㎞")]
    #[case::pdf_milligram("160mg", "160㎎")]
    #[case::numeric_invariance_milligram("240mg", "240㎎")]
    #[case::kilowatt("30kW", "30㎾")]
    #[case::megahertz("96.7MHz", "96.7㎒")]
    #[case::hectare("15.2ha", "15.2㏊")]
    fn compact_ascii_units_match_supported_compatibility_forms(
        #[case] ascii: &str,
        #[case] compatibility: &str,
    ) {
        let ascii = format!("값은 {ascii}이다");
        let compatibility = format!("값은 {compatibility}이다");
        assert_eq!(
            crate::encode_to_unicode(&ascii).unwrap(),
            crate::encode_to_unicode(&compatibility).unwrap()
        );
    }

    #[rstest::rstest]
    #[case::letter_after_digit("3m", "⠼⠉⠍")]
    #[case::letter_after_decimal_punctuation("4.m", "⠼⠙⠲⠍")]
    fn pure_english_ambiguous_suffixes_remain_on_ueb_path(
        #[case] input: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(crate::encode_to_unicode(input).unwrap(), expected);
    }

    #[rstest::rstest]
    #[case::longest_derived("30mW", 4)]
    #[case::hectare_derived("15.2ha", 6)]
    #[case::reject_partial_suffix("30kWh", 0)]
    fn parses_only_complete_compatibility_derived_units(
        #[case] input: &str,
        #[case] expected_consumed: usize,
    ) {
        let chars = input.chars().collect::<Vec<_>>();
        assert_eq!(
            parse_numeric_ascii_unit_prefix(&chars).map_or(0, |(_, _, consumed)| consumed),
            expected_consumed
        );
    }

    #[rstest::rstest]
    #[case::inch('㏌', "in")]
    #[case::centimetre('㎝', "cm")]
    #[case::millimetre('㎜', "mm")]
    #[case::gigabyte('㎇', "GB")]
    fn compatibility_units_match_existing_ascii_unit_spelling(
        #[case] glyph: char,
        #[case] ascii: &str,
    ) {
        let ascii_chars = ascii.chars().collect::<Vec<_>>();
        let expected = encode_ascii_unit(&ascii_chars, 0)
            .expect("existing rule 69 ASCII unit")
            .0;
        let decomposition = compatibility_unit_decomposition(glyph).unwrap();
        let actual = encode_compatibility_unit(&decomposition, true, true).unwrap();
        assert_eq!(actual, expected);
    }

    /// Rules 68/69: a compatibility presentation form follows the same general
    /// Roman-unit and superscript algorithm as its Unicode decomposition.
    #[rstest::rstest]
    #[case::kilogram("㎏", "⠴⠅⠛⠲")]
    #[case::gigahertz("㎓", "⠴⠠⠛⠠⠓⠵⠲")]
    #[case::cubic_metre("㎥", "⠴⠍⠘⠼⠉")]
    #[case::milliwatt("㎽", "⠴⠍⠠⠺⠲")]
    #[case::kilowatt("㎾", "⠴⠅⠠⠺⠲")]
    #[case::sievert("㏜", "⠴⠠⠎⠧⠲")]
    fn encodes_compatibility_unit_symbols(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(crate::encode_to_unicode(input).unwrap(), expected);
    }

    #[test]
    fn slash_after_korean_starts_a_new_roman_unit_chain() {
        let encoded = crate::encode_to_unicode("시/㎏").unwrap();
        assert!(
            encoded.ends_with("⠸⠌⠴⠅⠛⠲"),
            "the Roman indicator must not be suppressed after a Korean component: {encoded}"
        );
    }

    /// Exact PDF examples exercise both Roman-unit continuation through `/`
    /// and termination before a slash followed by a Korean unit.
    #[rstest::rstest]
    #[case::milligram_per_decilitre("160㎎/㎗", "⠼⠁⠋⠚⠴⠍⠛⠸⠌⠙⠇⠲")]
    #[case::calorie_per_square_centimetre_per_minute("cal/㎠/min", "⠴⠉⠁⠇⠸⠌⠉⠍⠘⠼⠃⠸⠌⠍⠔⠲")]
    #[case::megahertz("96.7 ㎒", "⠼⠊⠋⠲⠛⠀⠴⠠⠍⠠⠓⠵⠲")]
    #[case::kilometres_per_hour("80 ㎞/시", "⠼⠓⠚⠀⠴⠅⠍⠲⠸⠌⠠⠕")]
    fn preserves_pdf_unit_examples(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(crate::encode_to_unicode(input).unwrap(), expected);
    }

    /// Rules 33/34/69: the ordinary unit terminator is omitted only when the
    /// actual following boundary is one of the standard's punctuation/enclosing
    /// marks. These are full-encoder checks, including numeric-prefix routing.
    #[rstest::rstest]
    #[case::kilogram_in_parentheses("상자(20kg)당", "⠼⠃⠚⠴⠅⠛⠠⠴", "⠴⠅⠛⠲⠠⠴")]
    #[case::centimetre_before_korean_comma("키는 173cm, 몸무게는", "⠼⠁⠛⠉⠴⠉⠍⠐", "⠴⠉⠍⠲⠐")]
    #[case::centimetre_before_next_measurement("키 173cm, 68kg", "⠼⠁⠛⠉⠴⠉⠍⠂", "⠴⠉⠍⠲⠂")]
    #[case::metre_before_sentence_period("비거리 130m.", "⠼⠁⠉⠚⠴⠍⠲", "⠴⠍⠲⠲")]
    #[case::compatibility_kilogram_in_parentheses("상자(20㎏)당", "⠼⠃⠚⠴⠅⠛⠠⠴", "⠴⠅⠛⠲⠠⠴")]
    fn omits_unit_terminator_at_rule_33_or_34_boundary(
        #[case] input: &str,
        #[case] expected_segment: &str,
        #[case] forbidden_segment: &str,
    ) {
        let actual = crate::encode_to_unicode(input).unwrap();
        assert!(
            actual.contains(expected_segment),
            "missing rule-33/34 unit boundary {expected_segment:?} in {actual:?}"
        );
        assert!(
            !actual.contains(forbidden_segment),
            "unexpected rule-69 terminator at rule-33/34 boundary {forbidden_segment:?} in {actual:?}"
        );
    }

    /// Rule 69 remains the default outside the rule-33/34 override. End of
    /// input, a following Korean syllable, and forced slash boundaries retain
    /// the ordinary Roman terminator.
    #[rstest::rstest]
    #[case::end_of_input("180cm", "⠴⠉⠍⠲")]
    #[case::calorie_at_end("열량은 3cal", "⠴⠉⠁⠇⠲")]
    #[case::before_korean("1m는", "⠴⠍⠲")]
    #[case::before_forced_slash("3m/시", "⠴⠍⠲⠸⠌")]
    fn retains_unit_terminator_at_ordinary_rule_69_boundary(
        #[case] input: &str,
        #[case] expected_segment: &str,
    ) {
        let actual = crate::encode_to_unicode(input).unwrap();
        assert!(
            actual.contains(expected_segment),
            "missing ordinary rule-69 unit boundary {expected_segment:?} in {actual:?}"
        );
    }

    #[test]
    fn boundary_helper_does_not_remove_non_terminator_cells() {
        let word = "kg)".chars().collect::<Vec<_>>();
        let mut encoded = encode_unicode_cells("⠴⠅⠛");
        omit_roman_terminator_before_boundary(&mut encoded, &word, 2);
        assert_eq!(encoded, encode_unicode_cells("⠴⠅⠛"));
    }

    #[test]
    fn parses_compact_number_unit_word() {
        let chars: Vec<char> = "180cm".chars().collect();
        let parsed = parse_numeric_ascii_unit_prefix(&chars).expect("should parse 180cm");
        assert_eq!(parsed.0, "180");
        assert_eq!(parsed.2, chars.len());
    }

    #[test]
    fn parses_decimal_number_unit_word() {
        let chars: Vec<char> = "1,234.5kg".chars().collect();
        let parsed = parse_numeric_ascii_unit_prefix(&chars).expect("should parse decimal kg");

        assert_eq!(parsed.0, "1,234.5");
        assert_eq!(parsed.2, chars.len());
    }

    #[test]
    fn parses_leading_decimal_numeric_unit_word() {
        let chars: Vec<char> = ".5kg".chars().collect();
        let parsed = parse_numeric_ascii_unit_prefix(&chars).expect("should parse .5kg");

        assert_eq!(parsed.0, ".5");
        assert_eq!(parsed.2, chars.len());
    }

    /// 제69항 — percent-derived measurement abbreviations are data-driven, and
    /// `%p` only contracts at an abbreviation boundary.
    #[rstest::rstest]
    #[case::percentile("%ile", 4)]
    #[case::percentage_point("%p는", 2)]
    fn encodes_percent_abbreviation(#[case] input: &str, #[case] consumed: usize) {
        let chars: Vec<char> = input.chars().collect();
        let (encoded, actual_consumed) = encode_percent_abbreviation(&chars, 0).expect("abbr");
        assert!(!encoded.is_empty());
        assert_eq!(actual_consumed, consumed);
    }

    #[test]
    fn percent_p_does_not_match_inside_ascii_word() {
        let chars: Vec<char> = "%point".chars().collect();
        assert!(encode_percent_abbreviation(&chars, 0).is_none());
    }

    #[test]
    fn ascii_unit_scan_continues_past_non_matching_candidates() {
        let chars: Vec<char> = "zzz".chars().collect();

        assert!(encode_ascii_unit(&chars, 0).is_none());
    }

    #[test]
    fn rule69_metadata_is_stable() {
        use crate::rules::traits::BrailleRule;

        assert_eq!(Rule69.meta().name, "measurement_symbols");
        assert_eq!(Rule69.phase(), crate::rules::traits::Phase::CoreEncoding);
        assert_eq!(Rule69.priority(), 90);
    }

    /// rule_69:255 — `μ` (mu) alone or followed by non-unit chars triggers the
    /// else branch where `encode_unicode_cells("⠍")` is appended.
    #[test]
    fn rule69_mu_alone_without_unit() {
        // μ followed by Korean (no ASCII unit) → encode_ascii_unit returns None →
        // else branch at line 255 fires.
        let result = crate::encode_to_unicode("3μ가");
        assert!(result.is_ok());
        // μ at end with no following text.
        let result = crate::encode_to_unicode("3μ");
        assert!(result.is_ok());
    }
}
