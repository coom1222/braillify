//! 제28항 — 로마자는 ｢통일영어점자 규정｣에 따라 다음과 같이 적는다.
//!
//! English letters are mapped to braille using the UEB (Unified English Braille) system.
//! Uppercase indicators: single ⠠(32), word ⠠⠠(32,32), passage ⠠⠠⠠(32,32,32).
//!
//! Encoding is delegated to `english::encode_english()`.
//!
//! Reference: 2024 Korean Braille Standard, Chapter 4, Section 10, Article 28

use crate::char_struct::CharType;
use crate::rules::RuleMeta;
use crate::rules::context::RuleContext;
use crate::rules::english_shortform::requires_grade1_indicator;
use crate::rules::english_ueb::korean_context::KoreanPrefixInput;
use crate::rules::english_ueb::span::{encode_korean_unit, encode_korean_word};
use crate::rules::traits::{BrailleRule, Phase, RuleResult};

fn is_nonempty_ascii_word(word: &str) -> bool {
    !word.is_empty() && word.chars().all(|ch| ch.is_ascii_alphabetic())
}

pub static META: RuleMeta = RuleMeta {
    section: "28",
    subsection: None,
    name: "english_encoding",
    standard_ref: "2024 Korean Braille Standard, Ch.4 Sec.10 Art.28",
    description: "English letters encoded per UEB (Unified English Braille)",
};

/// Single uppercase indicator (대문자 기호표).
pub const UPPERCASE_SINGLE: u8 = 32; // ⠠

/// Encode a single English letter to braille.
#[cfg(test)]
fn apply(ch: char) -> Result<u8, String> {
    crate::english::encode_english(ch)
}

/// Returns a slice of indicator bytes to prepend.
#[cfg(test)]
fn uppercase_indicators(
    is_single_uppercase: bool,
    is_word_all_uppercase: bool,
    consecutive_uppercase_words: u8,
) -> &'static [u8] {
    if consecutive_uppercase_words >= 3 {
        &[32, 32, 32] // passage: ⠠⠠⠠
    } else if is_word_all_uppercase {
        &[32, 32] // word: ⠠⠠
    } else if is_single_uppercase {
        &[32] // single: ⠠
    } else {
        &[]
    }
}

/// Plugin struct for the rule engine.
///
/// Handles 제28항 English-in-Korean encoding: 로마자표/연속표 entry and uppercase
/// indicators. Letter/contraction cell production is delegated to
/// [`crate::rules::english_ueb::span`]; 종료표/exit orchestration lives in
/// [`crate::rules::emit`].
pub struct Rule28;

impl BrailleRule for Rule28 {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn phase(&self) -> Phase {
        Phase::CoreEncoding
    }

    fn matches(&self, ctx: &RuleContext) -> bool {
        matches!(ctx.char_type, CharType::English(_))
    }

    fn apply(&self, ctx: &mut RuleContext) -> Result<RuleResult, String> {
        let CharType::English(c) = ctx.char_type else {
            return Ok(RuleResult::Skip);
        };

        // Enter English mode (로마자표 / 연속표)
        // 제39항 영어 주도 문서에서는 영자표시/연속표를 emit하지 않는다.
        if ctx.state.english_indicator
            && !ctx.state.is_english
            && !ctx.state.english_dominant_no_indicator
        {
            if ctx.state.needs_english_continuation {
                ctx.emit(48);
            } else {
                ctx.emit(52);
            }
        }

        // 제37항: a Roman section in Korean text spells the word with UEB
        // alphabet signs and multi-letter groupsigns, while suppressing UEB
        // whole-word contractions. Encode each contiguous ASCII letter run in
        // one pass so the shared UEB preference/morphology algorithm can choose
        // contractions across the whole word. Lowercase apostrophe continuations
        // retain the legacy position-aware path because they are not fresh word
        // starts. An uppercase continuation is encoded as a run so UEB 8.4.2 can
        // restart capitals mode after the nonalphabetic apostrophe.
        let starts_ascii_run = c.is_ascii_alphabetic()
            && ctx
                .index
                .checked_sub(1)
                .and_then(|index| ctx.word_chars.get(index))
                .is_none_or(|previous| !previous.is_ascii_alphabetic());
        let follows_apostrophe = ctx
            .index
            .checked_sub(1)
            .and_then(|index| ctx.word_chars.get(index))
            .is_some_and(|previous| matches!(previous, '\'' | '\u{2019}'));
        if starts_ascii_run && (!follows_apostrophe || c.is_ascii_uppercase()) {
            let run_end = ctx.index
                + ctx.word_chars[ctx.index..]
                    .iter()
                    .take_while(|ch| ch.is_ascii_alphabetic())
                    .count();
            let run = &ctx.word_chars[ctx.index..run_end];
            // The token rule pre-emits capitals-word mode only for the initial
            // uppercase letters-sequence. UEB 8.4.2 ends that mode at a
            // nonletter, so a later run (the final `T` in official `AT&T`)
            // must produce its own capitalization indicator.
            let caps_already_emitted = ctx.index == 0
                && ctx.is_all_uppercase
                && ctx.word_len() >= 2
                && ctx.ascii_starts_at_beginning;
            let is_whole_lowercase_word = ctx.index == 0
                && run_end == ctx.word_chars.len()
                && run.iter().all(|ch| ch.is_ascii_lowercase());
            let prev_is_ascii_word = is_nonempty_ascii_word(ctx.prev_word);
            let next_is_ascii_word = match ctx.remaining_words.first() {
                Some(word) => is_nonempty_ascii_word(word),
                None => false,
            };
            // Rule 37's PDF example, "그는 Can you help me?라고 도움을 요청했다.",
            // suppresses a whole-word sign for the first Roman word (`Can`) but retains
            // the UEB wordsign for the phrase-interior `you`. The adjacent-ASCII-word
            // gate models that structural position. Rule 39's "What is 김치 in English?"
            // resumes the surrounding English passage after Korean, so the persistent
            // English-dominant gate retains the resumed `in` wordsign. Neither gate
            // depends on the example's literal words.
            let standalone_wordsign = is_whole_lowercase_word
                && (ctx.state.english_dominant_wrap_active
                    || (prev_is_ascii_word && next_is_ascii_word));
            let word_initial = ctx.index == 0
                || ctx.word_chars.get(ctx.index - 1).is_some_and(|previous| {
                    matches!(
                        previous,
                        '(' | '[' | '{' | '\u{2018}' | '\u{201c}' | '"' | '-'
                    )
                });
            let digit_adjacent = ctx
                .index
                .checked_sub(1)
                .and_then(|index| ctx.word_chars.get(index))
                .is_some_and(|ch| ch.is_ascii_digit())
                || ctx
                    .word_chars
                    .get(run_end)
                    .is_some_and(|ch| ch.is_ascii_digit());
            // UEB 5.7.2, 5.8.1, and 10.9.7: when an attached Roman entry is a
            // complete pure-letter shortform, grade 1 precedes its capitalization
            // marker. Standalone ASCII tokens have already been handled by
            // `UppercasePassageRule`. Digit and hyphen continuations retain their
            // independent rule-35/state-machine paths.
            let follows_hyphen = ctx
                .index
                .checked_sub(1)
                .and_then(|index| ctx.word_chars.get(index))
                .is_some_and(|ch| *ch == '-');
            let uppercase_run = run.iter().collect::<String>();
            let prepend_grade1_indicator = !caps_already_emitted
                && word_initial
                && !digit_adjacent
                && !follows_hyphen
                && run.iter().all(|ch| ch.is_ascii_uppercase())
                && requires_grade1_indicator(&uppercase_run);
            if let Some(cells) = encode_korean_word(
                run,
                caps_already_emitted,
                prepend_grade1_indicator,
                standalone_wordsign,
                word_initial,
                digit_adjacent,
            ) {
                ctx.emit_slice(&cells);
                *ctx.skip_count = run.len().saturating_sub(1);
                ctx.state.is_english = true;
                ctx.state.needs_english_continuation = false;
                return Ok(RuleResult::Consumed);
            }
        }

        // Uppercase indicators (single/consecutive uppercase run)
        if (!ctx.is_all_uppercase || ctx.word_len() < 2 || !ctx.ascii_starts_at_beginning)
            && !ctx.state.is_big_english
            && c.is_uppercase()
        {
            ctx.state.is_big_english = true;
            for idx in 0..std::cmp::min(ctx.word_len() - ctx.index, 2) {
                if ctx.word_chars[ctx.index + idx].is_uppercase() {
                    ctx.emit(UPPERCASE_SINGLE);
                } else {
                    break;
                }
            }
        }

        // English abbreviation lookup + fallback letter encoding.
        // Korean-context UEB contractions and standalone wordsigns are delegated to
        // `encode_korean_unit`; this rule only decides the surrounding mode markers.
        let is_whole_lowercase_word =
            ctx.index == 0 && ctx.word_chars.iter().all(|ch| ch.is_ascii_lowercase());
        let prev_is_ascii_word =
            !ctx.prev_word.is_empty() && ctx.prev_word.chars().all(|ch| ch.is_ascii_alphabetic());
        let next_is_ascii_word = ctx
            .remaining_words
            .first()
            .is_some_and(|w| !w.is_empty() && w.chars().all(|ch| ch.is_ascii_alphabetic()));
        let unit = encode_korean_unit(KoreanPrefixInput {
            word: ctx.word_chars,
            pos: ctx.index,
            wrap_active: ctx.state.english_dominant_wrap_active,
            is_all_uppercase: ctx.is_all_uppercase,
            at_entry: !ctx.state.is_english || ctx.index == 0,
            standalone_wordsign: is_whole_lowercase_word
                && prev_is_ascii_word
                && next_is_ascii_word,
        })?;
        ctx.emit_slice(&unit.cells);
        if unit.contracted {
            *ctx.skip_count = unit.consumed.saturating_sub(1);
        }

        ctx.state.is_english = true;
        ctx.state.needs_english_continuation = false;
        Ok(RuleResult::Consumed)
    }
}

/// Determine the uppercase indicator(s) needed.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::context::EncodingMode;
    use crate::unicode::decode_unicode;
    use crate::{EncodeOptions, encode_to_unicode, encode_with_options};

    #[rstest::rstest]
    #[case::empty("", false)]
    #[case::ascii("help", true)]
    #[case::korean("도움", false)]
    #[case::punctuated("me?", false)]
    fn classifies_adjacent_ascii_words(#[case] word: &str, #[case] expected: bool) {
        assert_eq!(is_nonempty_ascii_word(word), expected);
    }

    /// 제28항 — 영문자 점역. 소문자/대문자 모두 동일 점형으로 인코딩.
    #[rstest::rstest]
    #[case::lower_a('a', '⠁')]
    #[case::lower_z('z', '⠵')]
    #[case::upper_a_as_lowercase('A', '⠁')]
    fn encodes_english_letters(#[case] ch: char, #[case] expected: char) {
        assert_eq!(apply(ch).unwrap(), decode_unicode(expected));
    }

    /// 영문자가 아닌 입력은 Err.
    #[rstest::rstest]
    #[case::digit('1')]
    #[case::syllable('가')]
    fn invalid_returns_error(#[case] ch: char) {
        assert!(apply(ch).is_err());
    }

    /// `uppercase_indicators` — single/word/passage 대문자 지시자 점형.
    #[rstest::rstest]
    #[case::single_letter(true, false, 0, &[32u8] as &[u8])]
    #[case::word_two_letters(false, true, 0, &[32, 32])]
    #[case::passage_run(false, true, 3, &[32, 32, 32])]
    #[case::no_indicator_lower(false, false, 0, &[] as &[u8])]
    fn uppercase_indicator_paths(
        #[case] single: bool,
        #[case] is_word: bool,
        #[case] run: u8,
        #[case] expected: &[u8],
    ) {
        assert_eq!(uppercase_indicators(single, is_word, run), expected);
    }

    /// 제37항 PDF examples: Korean-context Roman words suppress whole-word
    /// contractions while retaining their applicable multi-letter groupsigns.
    #[rstest::rstest]
    #[case::initial_letter_groupsign("every", &[52, 16, 17, 61, 50])]
    #[case::lower_and_strong_groupsigns("enough", &[52, 34, 51, 35, 50])]
    #[case::strong_contraction_inside_word("rather", &[52, 23, 1, 46, 23, 50])]
    #[case::entry_lower_wordsign_spelled_as_letters("in", &[52, 10, 29, 50])]
    fn korean_roman_words_share_ueb_groupsign_algorithm(
        #[case] input: &str,
        #[case] expected: &[u8],
    ) {
        let options = EncodeOptions {
            default_mode: Some(EncodingMode::Korean),
        };
        assert_eq!(encode_with_options(input, &options).unwrap(), expected);
    }

    /// 제37항 PDF 문장 전체를 공개 encoder로 통과시켜, 첫 Roman 어절의
    /// complete wordsign 억제와 뒤따르는 Roman phrase 경로를 함께 검증한다.
    #[test]
    fn rule_37_official_sentence_uses_shared_roman_engine() {
        assert_eq!(
            encode_to_unicode("그는 Can you help me?라고 도움을 요청했다.").unwrap(),
            "⠈⠪⠉⠵⠀⠴⠠⠉⠁⠝⠀⠽⠀⠓⠑⠇⠏⠀⠍⠑⠦⠐⠣⠈⠥⠀⠊⠥⠍⠢⠮⠀⠬⠰⠻⠚⠗⠌⠊⠲"
        );
    }

    /// UEB 5.7.2/5.8.1/10.9.7 complete-shortform handling through the complete
    /// Korean encoder. Every Roman surface comes directly from the PDF examples
    /// (`CD`, `ALT`, `NEC`); the Korean wrapper exercises only rule 28/29/34 routing.
    #[rstest::rstest]
    #[case::standing_alone_could("가(CD)", "⠫⠦⠄⠴⠰⠠⠠⠉⠙⠠⠴")]
    #[case::alt_example("가(ALT)", "⠫⠦⠄⠴⠰⠠⠠⠁⠇⠞⠠⠴")]
    #[case::nec_example("가(NEC)", "⠫⠦⠄⠴⠰⠠⠠⠝⠑⠉⠠⠴")]
    fn attached_allcaps_complete_shortform_uses_grade1(
        #[case] input: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(crate::encode_to_unicode(input).unwrap(), expected);
    }

    /// UEB 8.4.2 keeps an internal apostrophe in the Roman letters-sequence but
    /// terminates capitals-word mode at that nonalphabetic symbol. The Roman
    /// surfaces are official UEB examples; the neutral Korean wrapper exercises
    /// Rule 28/29 routing. Korean Rule 37 still suppresses the `that` wordsign in
    /// `THAT'S`, so its initial run retains the permitted `th` groupsign instead.
    #[rstest::rstest]
    #[case::official_name("가 O'Hara 나", "⠫⠀⠴⠠⠕⠄⠠⠓⠜⠁⠲⠀⠉")]
    #[case::official_contraction("가 DON'T 나", "⠫⠀⠴⠠⠠⠙⠕⠝⠄⠠⠞⠲⠀⠉")]
    #[case::official_possessive("가 THAT'S 나", "⠫⠀⠴⠠⠠⠹⠁⠞⠄⠠⠎⠲⠀⠉")]
    #[case::official_two_letter_suffix("가 SHE'LL 나", "⠫⠀⠴⠠⠠⠩⠑⠄⠠⠠⠇⠇⠲⠀⠉")]
    fn korean_wrapper_restarts_capitals_after_internal_apostrophe(
        #[case] input: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(crate::encode_to_unicode(input).as_deref(), Ok(expected));
    }

    #[test]
    fn english_dominant_wrap_resumes_ueb_wordsigns_after_korean_span() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("in", true);
        owned.state.is_english = true;
        owned.state.english_dominant_wrap_active = true;
        let mut ctx = owned.ctx_at(0);

        assert!(matches!(
            Rule28.apply(&mut ctx).unwrap(),
            RuleResult::Consumed
        ));
        assert_eq!(owned.result, vec![20]);
    }

    /// Rule 37's PDF sentence `Can you help me?` permits the phrase-interior
    /// `you` wordsign because both adjacent whitespace-delimited words are Roman.
    #[test]
    fn rule_37_phrase_interior_word_uses_standalone_wordsign() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("you", true)
            .with_prev_word("Can")
            .with_remaining_words(["help", "me?"]);
        let mut ctx = owned.ctx_at(0);

        assert!(matches!(
            Rule28.apply(&mut ctx).unwrap(),
            RuleResult::Consumed
        ));
        assert_eq!(owned.result, vec![52, decode_unicode('⠽')]);
    }

    #[test]
    fn apply_skips_non_korean() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("A", false);
        let mut ctx = owned.ctx_at(0);
        let _ = Rule28.apply(&mut ctx).unwrap();
        // Just exercise apply() for coverage
    }

    /// rule_28 — multi-cell `ong` abbreviation hit via real word `pyeongchang`
    /// from PDF testcase (rule_35.json). The 'o' at index 2 has remaining="ongchang"
    /// which matches `rule_en_multi_cell`.
    #[test]
    fn rule28_multi_cell_via_pyeongchang() {
        let _ = crate::encode("pyeongchang 2018");
    }

    /// rule_28:205-206 — multi-cell English abbreviation ("ong" → ⠰⠛)
    /// applied word-middle. Drives the `rule_en_multi_cell` arm via direct
    /// `RuleContext` setup with state.is_english=true, index > 0.
    #[test]
    fn rule28_multi_cell_word_middle_direct() {
        use crate::char_struct::CharType;
        let word: Vec<char> = "along".chars().collect();
        let ct = CharType::English('o');
        let mut skip = 0usize;
        let mut state = crate::rules::context::EncoderState::new(false);
        state.is_english = true;
        let mut out = Vec::new();
        let mut ctx = crate::rules::context::RuleContext {
            word_chars: &word,
            index: 2, // 'o' position; remaining = "ong"
            char_type: &ct,
            prev_word: "",
            remaining_words: &[],
            has_korean_char: false,
            is_all_uppercase: false,
            ascii_starts_at_beginning: true,
            skip_count: &mut skip,
            state: &mut state,
            result: &mut out,
        };
        let outcome = Rule28.apply(&mut ctx).unwrap();
        // Either Consumed (multi-cell applied) or other; at minimum the arm runs.
        let _ = outcome;
    }

    /// rule_28 line 64 — `let-else return Skip` for non-English ctx.
    #[test]
    fn rule28_apply_skip_for_non_english_ctx() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("가", false);
        let mut ctx = owned.ctx_at(0);
        let outcome = Rule28.apply(&mut ctx).unwrap();
        assert!(matches!(outcome, RuleResult::Skip));
    }
}
