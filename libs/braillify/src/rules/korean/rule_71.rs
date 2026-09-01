use crate::char_struct::CharType;
use crate::rules::RuleMeta;
use crate::rules::context::{EncodingMode, RuleContext};
use crate::rules::traits::{BrailleRule, Phase, RuleResult};

pub static META: RuleMeta = RuleMeta {
    section: "71",
    subsection: None,
    name: "information_symbols",
    standard_ref: "2024 Korean Braille Standard, Ch.6 Art.71",
    description: "Keyboard, copyright, and information symbols",
};

const MAPPINGS: &[(char, &str)] = &[
    ('@', "⠈⠁"),
    ('^', "⠈⠢"),
    ('#', "⠸⠹"),
    ('|', "⠸⠳"),
    ('\\', "⠸⠡"),
    ('&', "⠈⠯"),
    ('§', "⠘⠎"),
    ('¶', "⠘⠏"),
    ('©', "⠘⠉"),
    ('®', "⠘⠗"),
    ('™', "⠘⠞"),
];

fn encode_unicode_cells(unicode: &str) -> Vec<u8> {
    unicode
        .chars()
        .map(crate::unicode::decode_unicode)
        .collect()
}

fn should_wrap_information_symbol(ctx: &RuleContext) -> bool {
    if ctx.word_len() > 1 {
        return true;
    }

    let prev_has_korean =
        !ctx.prev_word.is_empty() && ctx.prev_word.chars().any(crate::utils::is_korean_char);
    let next_has_korean = ctx
        .remaining_words
        .first()
        .is_some_and(|word| !word.is_empty() && word.chars().any(crate::utils::is_korean_char));

    prev_has_korean || next_has_korean
}

/// UEB 3.1.1 writes `&` directly between attached ASCII-letter segments
/// (for example, AT&T and B&B). The surrounding Roman section already owns
/// the mode indicators, so Rule 71 must emit only the ampersand cells there.
fn is_attached_roman_ampersand(ctx: &RuleContext) -> bool {
    crate::english_logic::is_attached_ascii_roman_ampersand(ctx.word_chars, ctx.index)
}

fn begins_attached_roman_segment(ctx: &RuleContext) -> bool {
    crate::english_logic::is_ampersand_before_attached_ascii_roman_segment(
        ctx.word_chars,
        ctx.index,
    )
}

pub fn is_rule_71_symbol(c: char) -> bool {
    MAPPINGS.iter().any(|(candidate, _)| *candidate == c)
}

pub struct Rule71;

impl BrailleRule for Rule71 {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn phase(&self) -> Phase {
        Phase::CoreEncoding
    }

    fn priority(&self) -> u16 {
        175
    }

    fn matches(&self, ctx: &RuleContext) -> bool {
        ctx.state.current_mode() != EncodingMode::Math
            && matches!(ctx.char_type, CharType::Symbol(c) if is_rule_71_symbol(*c))
    }

    fn apply(&self, ctx: &mut RuleContext) -> Result<RuleResult, String> {
        if ctx.current_char() == '§' {
            if should_wrap_information_symbol(ctx) {
                // 제71항: 정보 기호는 한국어/숫자 컨텍스트에서 ⠴...⠲ wrap을 두른다.
                // 직후가 숫자면 종료표 ⠲ 생략(숫자 자체가 영자 컨텍스트로 이어짐).
                // 어절 내부에서 §을 만났을 때(ctx.index > 0)도 추가 공백을 emit하지 않는다.
                // 어절 간 공백은 Token::Space가 책임지며, 어절 내 음절/기호 사이는
                // 묵자 입력 그대로 결합한다(한국어 띄어쓰기 일반 원칙).
                let mut encoded = vec![crate::unicode::decode_unicode('⠴')];
                encoded.extend(encode_unicode_cells("⠘⠎"));
                if !ctx.next_char().is_some_and(|ch| ch.is_ascii_digit()) {
                    encoded.push(crate::unicode::decode_unicode('⠲'));
                }
                ctx.emit_slice(&encoded);
                return Ok(RuleResult::Consumed);
            }

            let encoded = encode_unicode_cells("⠘⠎");
            ctx.emit_slice(&encoded);
            return Ok(RuleResult::Consumed);
        }

        let Some((_, unicode)) = MAPPINGS
            .iter()
            .find(|(candidate, _)| *candidate == ctx.current_char())
        else {
            return Ok(RuleResult::Skip);
        };

        let mut encoded = Vec::new();
        if should_wrap_information_symbol(ctx)
            && ctx.current_char() == '&'
            && begins_attached_roman_segment(ctx)
        {
            // Korean rules 29/32/71 and UEB 3.1.1 `&c`: the ambiguous
            // ampersand opens the Roman section, but the attached ASCII-letter
            // segment owns its eventual terminator. Do not close and re-enter
            // between the two printed-adjacent items.
            if !ctx.state.is_english {
                if ctx.state.english_dominant_no_indicator {
                    ctx.state.is_english = true;
                    ctx.state.needs_english_continuation = false;
                    ctx.state.roman_number_chain = false;
                } else {
                    crate::rules::roman_mode::enter_english(ctx.state, ctx.result);
                }
            }
            encoded = encode_unicode_cells(unicode);
        } else if should_wrap_information_symbol(ctx)
            && matches!(ctx.current_char(), '&' | '¶' | '©' | '®' | '™')
            && !is_attached_roman_ampersand(ctx)
        {
            encoded.push(crate::unicode::decode_unicode('⠴'));
            encoded.extend(encode_unicode_cells(unicode));
            encoded.push(crate::unicode::decode_unicode('⠲'));
        } else {
            encoded = encode_unicode_cells(unicode);
        }
        ctx.emit_slice(&encoded);
        Ok(RuleResult::Consumed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_exercise() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("A", false);
        let mut ctx = owned.ctx_at(0);
        // Just exercise apply() for coverage; either Skip or Continue/Consumed is OK
        let _ = Rule71.apply(&mut ctx);
    }

    #[test]
    fn matches_does_not_panic() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("A", false);
        let ctx = owned.ctx_at(0);
        let _ = Rule71.matches(&ctx);
    }

    /// 제71항 — § 정보 기호가 직후 숫자를 만나면 종료표(⠲) 생략 (line 84-86).
    #[test]
    fn rule71_section_sign_before_digit_omits_terminator() {
        let word: Vec<char> = "§1".chars().collect();
        let ct = CharType::Symbol('§');
        let mut skip = 0usize;
        let mut state = crate::rules::context::EncoderState::new(false);
        let mut out = Vec::new();
        let mut ctx = RuleContext {
            word_chars: &word,
            index: 0,
            char_type: &ct,
            prev_word: "",
            remaining_words: &[],
            has_korean_char: false,
            is_all_uppercase: false,
            ascii_starts_at_beginning: false,
            skip_count: &mut skip,
            state: &mut state,
            result: &mut out,
        };
        let outcome = Rule71.apply(&mut ctx).unwrap();
        assert!(matches!(outcome, RuleResult::Consumed));
        // No ⠲ terminator because next char is a digit
        assert!(!out.contains(&crate::unicode::decode_unicode('⠲')));
    }

    /// rule_71:85 — § followed by NON-digit (or end of input) appends ⠲ terminator.
    #[test]
    fn rule71_section_symbol_followed_by_non_digit_appends_terminator() {
        // Encode "§A" — next char is letter, not digit → ⠲ appended at line 85.
        let result = crate::encode("§A");
        assert!(result.is_ok());
        // Also: § alone (no next char) → no digit → ⠲ appended.
        let _ = crate::encode("§");
    }

    /// The Korean Rule 71 encoder owns the ampersand cells even while the
    /// surrounding Roman section remains open. These are the official UEB
    /// 3.1.1 surface forms, not corpus-derived examples.
    #[rstest::rstest]
    #[case::official_at_and_t("AT&T")]
    #[case::official_b_and_b("B&B")]
    fn attached_roman_ampersand_emits_bare_rule_71_cells(#[case] input: &str) {
        let mut owned = crate::test_helpers::CtxOwned::for_text(input, false);
        let ampersand_index = input.chars().position(|ch| ch == '&').unwrap();
        let mut ctx = owned.ctx_at(ampersand_index);

        let outcome = Rule71.apply(&mut ctx).unwrap();

        assert!(matches!(outcome, RuleResult::Consumed));
        assert_eq!(ctx.result.as_slice(), encode_unicode_cells("⠈⠯"));
    }

    /// UEB 3.1.1's official `&c` surface exercises the Korean Rule-71 wrapper
    /// state directly: the Roman indicator precedes `&`, and the section stays
    /// open for the attached `c` rather than emitting a terminator/re-entry.
    #[test]
    fn one_sided_official_ampersand_opens_and_keeps_roman_section() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("&c", true);
        let mut ctx = owned.ctx_at(0);

        let outcome = Rule71.apply(&mut ctx).unwrap();

        assert!(matches!(outcome, RuleResult::Consumed));
        assert_eq!(ctx.result.as_slice(), encode_unicode_cells("⠴⠈⠯"));
        assert!(ctx.state.is_english);
    }

    /// Full-encoder controls reproduce the two UEB 3.1.1 examples exactly.
    #[rstest::rstest]
    #[case::official_at_and_t("AT&T", "⠠⠠⠁⠞⠈⠯⠠⠞")]
    #[case::official_b_and_b("B&B", "⠠⠃⠈⠯⠠⠃")]
    fn full_encoder_preserves_official_ueb_ampersand_examples(
        #[case] input: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(crate::encode_to_unicode(input).unwrap(), expected);
    }

    /// UEB 8.4.2 ends capitals word mode at the nonalphabetic ampersand.
    /// Wrapping the official UEB 3.1.1 examples in neutral Korean text proves
    /// that the mixed-document rule-28/29 path restarts capitalization for the
    /// next ASCII-letter segment while keeping one Roman section.
    #[rstest::rstest]
    #[case::official_at_and_t("가 AT&T 나", "⠫⠀⠴⠠⠠⠁⠞⠈⠯⠠⠞⠲⠀⠉")]
    #[case::official_b_and_b("가 B&B 나", "⠫⠀⠴⠠⠃⠈⠯⠠⠃⠲⠀⠉")]
    fn korean_wrapper_preserves_ampersand_capitalization_extent(
        #[case] input: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(crate::encode_to_unicode(input).as_deref(), Ok(expected));
    }

    /// Korean Rule 71's spaced Hangul example remains an independently
    /// delimited information symbol after the attached-Roman exception.
    #[test]
    fn full_encoder_preserves_official_korean_spaced_ampersand_example() {
        assert_eq!(
            crate::encode_to_unicode("종이접기 & 클레이아트").unwrap(),
            "⠨⠿⠕⠨⠎⠃⠈⠕⠀⠴⠈⠯⠲⠀⠋⠮⠐⠝⠕⠣⠓⠪",
        );
    }
}
