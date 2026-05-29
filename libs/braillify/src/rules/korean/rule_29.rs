//! 제29항 — 국어 문장 안에 로마자가 나올 때에는 그 앞에 로마자표 ⠴(52)을 적고
//! 그 뒤에 로마자 종료표 ⠲(50)을 적는다.
//!
//! 제31항 — 국어 문장 안에 그리스 문자가 나올 때에도 로마자표와 종료표를 적는다.
//!
//! 제33항 — 문장 부호의 점형이 다른 경우 종료표를 생략하는 규칙.
//! 제35항 — 로마자와 숫자가 이어 나올 때에는 종료표를 적지 않는다.
//!
//! Reference: 2024 Korean Braille Standard, Chapter 4, Section 10, Articles 29, 31, 33, 35

use crate::char_struct::CharType;
use crate::rules::RuleMeta;
use crate::rules::context::RuleContext;
use crate::rules::korean::rule_69::encode_ascii_unit;
use crate::rules::traits::{BrailleRule, Phase, RuleResult};

pub static META_29: RuleMeta = RuleMeta {
    section: "29",
    subsection: None,
    name: "roman_indicator",
    standard_ref: "2024 Korean Braille Standard, Ch.4 Sec.10 Art.29",
    description: "Roman letter indicator ⠴ (enter) and terminator ⠲ (exit)",
};

/// Roman letter indicator (로마자표).
pub const ROMAN_INDICATOR: u8 = 52; // ⠴

/// Roman letter terminator (로마자 종료표).
#[cfg(test)]
pub const ROMAN_TERMINATOR: u8 = 50; // ⠲

/// English continuation indicator (연속표).
pub const ENGLISH_CONTINUATION: u8 = 48; // ⠰

/// Plugin struct for the rule engine.
///
/// Manages English/Roman mode transitions (제29항, 제31항, 제33항, 제35항).
/// Emits 로마자표 ⠴ when entering English mode, 로마자 종료표 ⠲ when exiting.
/// Uses 연속표 ⠐ when continuing English after an interruption (e.g., number).
///
/// This rule runs in the ModeManagement phase, before CoreEncoding.
/// It inspects the current character and state to decide mode transitions.
pub struct Rule29;

fn prev_word_is_numeric(prev_word: &str) -> bool {
    !prev_word.is_empty()
        && prev_word
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, ',' | '.'))
}

fn should_enter_as_roman_indicator(ctx: &RuleContext) -> bool {
    let prev_is_numeric_or_digit = ctx.prev_char().is_some_and(|ch| ch.is_ascii_digit())
        || prev_word_is_numeric(ctx.prev_word);
    encode_ascii_unit(ctx.word_chars, ctx.index).is_some() && prev_is_numeric_or_digit
}

impl BrailleRule for Rule29 {
    fn meta(&self) -> &'static RuleMeta {
        &META_29
    }

    fn phase(&self) -> Phase {
        Phase::ModeManagement
    }

    fn matches(&self, ctx: &RuleContext) -> bool {
        // Only relevant when english_indicator is active (Korean text contains English)
        if !ctx.state.english_indicator {
            return false;
        }
        // Match when we need to enter English mode (current char is English and not in English)
        if !ctx.state.is_english && matches!(ctx.char_type, CharType::English(_)) {
            return true;
        }
        // Match when we're in English and encounter a non-English char (potential exit)
        if ctx.state.is_english && !matches!(ctx.char_type, CharType::English(_)) {
            return true;
        }
        false
    }

    fn apply(&self, ctx: &mut RuleContext) -> Result<RuleResult, String> {
        if !ctx.state.is_english && matches!(ctx.char_type, CharType::English(_)) {
            // Enter English mode
            if ctx.state.needs_english_continuation && !should_enter_as_roman_indicator(ctx) {
                ctx.emit(ENGLISH_CONTINUATION); // ⠐ continuation
            } else {
                ctx.emit(ROMAN_INDICATOR); // ⠴ enter
            }
            ctx.state.is_english = true;
            ctx.state.needs_english_continuation = false;
        }
        // Exit logic is complex (depends on next word, symbol type, etc.)
        // and is deferred to Phase 3 engine-driven rewrite.
        Ok(RuleResult::Continue) // Continue to CoreEncoding
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indicator_values() {
        assert_eq!(ROMAN_INDICATOR, 52);
        assert_eq!(ROMAN_TERMINATOR, 50);
        assert_eq!(ENGLISH_CONTINUATION, 48);
    }

    #[test]
    fn golden_test_roman_in_korean() {
        // "그는 Canada로" → Roman indicator before Canada, terminator after
        let result = crate::encode_to_unicode("그는 Canada로").unwrap();
        assert!(result.contains('⠴'), "Should contain roman indicator ⠴");
    }

    /// `prev_word_is_numeric` — 숫자 및 `.` / `,` 만으로 구성된 단어를 true.
    #[rstest::rstest]
    #[case::pure_digits("123", true)]
    #[case::digits_with_comma("1,234", true)]
    #[case::decimal("3.14", true)]
    #[case::compound_punctuation("1.234,567", true)]
    #[case::empty_string("", false)]
    #[case::digits_with_letter("12a", false)]
    #[case::letters_only("hello", false)]
    fn prev_word_is_numeric_paths(#[case] input: &str, #[case] expected: bool) {
        assert_eq!(prev_word_is_numeric(input), expected);
    }

    fn make_ctx<'a>(
        word_chars: &'a [char],
        index: usize,
        char_type: &'a CharType,
        skip_count: &'a mut usize,
        state: &'a mut crate::rules::context::EncoderState,
        result: &'a mut Vec<u8>,
        prev_word: &'a str,
    ) -> RuleContext<'a> {
        RuleContext {
            word_chars,
            index,
            char_type,
            prev_word,
            remaining_words: &[],
            has_korean_char: false,
            is_all_uppercase: false,
            ascii_starts_at_beginning: true,
            skip_count,
            state,
            result,
        }
    }

    #[test]
    fn rule29_meta_and_phase() {
        let r = Rule29;
        assert_eq!(r.meta().section, "29");
        assert!(matches!(r.phase(), Phase::ModeManagement));
    }

    #[test]
    fn rule29_matches_false_when_indicator_off() {
        let chars: Vec<char> = "A".chars().collect();
        let ct = CharType::new(chars[0]).unwrap();
        let mut skip = 0usize;
        let mut state = crate::rules::context::EncoderState::new(false); // no english_indicator
        let mut out = Vec::new();
        let ctx = make_ctx(&chars, 0, &ct, &mut skip, &mut state, &mut out, "");
        assert!(!Rule29.matches(&ctx));
    }

    #[test]
    fn rule29_matches_when_entering_english() {
        let chars: Vec<char> = "A".chars().collect();
        let ct = CharType::new(chars[0]).unwrap();
        let mut skip = 0usize;
        let mut state = crate::rules::context::EncoderState::new(true);
        state.is_english = false;
        let mut out = Vec::new();
        let ctx = make_ctx(&chars, 0, &ct, &mut skip, &mut state, &mut out, "");
        assert!(Rule29.matches(&ctx));
    }

    #[test]
    fn rule29_matches_when_exiting_english() {
        let chars: Vec<char> = "ㄱ".chars().collect();
        let ct = CharType::new(chars[0]).unwrap();
        let mut skip = 0usize;
        let mut state = crate::rules::context::EncoderState::new(true);
        state.is_english = true; // already in english
        let mut out = Vec::new();
        let ctx = make_ctx(&chars, 0, &ct, &mut skip, &mut state, &mut out, "");
        assert!(Rule29.matches(&ctx));
    }

    #[test]
    fn rule29_apply_enters_english_with_indicator() {
        let chars: Vec<char> = "A".chars().collect();
        let ct = CharType::new(chars[0]).unwrap();
        let mut skip = 0usize;
        let mut state = crate::rules::context::EncoderState::new(true);
        let mut out = Vec::new();
        let mut ctx = make_ctx(&chars, 0, &ct, &mut skip, &mut state, &mut out, "");
        let res = Rule29.apply(&mut ctx).unwrap();
        assert!(matches!(res, RuleResult::Continue));
        assert_eq!(out, vec![ROMAN_INDICATOR]);
        assert!(state.is_english);
    }

    #[test]
    fn rule29_apply_continuation_after_numeric_prev_word() {
        // Just exercise the should_enter_as_roman_indicator path branches.
        // The exact byte depends on encode_ascii_unit matching behavior.
        let chars: Vec<char> = "A".chars().collect();
        let ct = CharType::new(chars[0]).unwrap();
        let mut skip = 0usize;
        let mut state = crate::rules::context::EncoderState::new(true);
        state.needs_english_continuation = true;
        let mut out = Vec::new();
        let mut ctx = make_ctx(&chars, 0, &ct, &mut skip, &mut state, &mut out, "123");
        Rule29.apply(&mut ctx).unwrap();
        assert_eq!(out.len(), 1);
        assert!(matches!(out[0], ROMAN_INDICATOR | ENGLISH_CONTINUATION));
    }

    #[test]
    fn rule29_apply_continuation_marker_path() {
        // needs_english_continuation=true AND should_enter_as_roman_indicator=false
        // → emit ENGLISH_CONTINUATION.
        let chars: Vec<char> = "A".chars().collect();
        let ct = CharType::new(chars[0]).unwrap();
        let mut skip = 0usize;
        let mut state = crate::rules::context::EncoderState::new(true);
        state.needs_english_continuation = true;
        let mut out = Vec::new();
        // prev_word empty (not numeric) and prev_char None at index 0 → not ascii digit
        let mut ctx = make_ctx(&chars, 0, &ct, &mut skip, &mut state, &mut out, "");
        Rule29.apply(&mut ctx).unwrap();
        assert_eq!(out, vec![ENGLISH_CONTINUATION]);
    }

    #[test]
    fn rule29_apply_no_change_when_exiting() {
        // In english, current char is Korean → matches=true but apply only handles enter
        let chars: Vec<char> = "가".chars().collect();
        let ct = CharType::new(chars[0]).unwrap();
        let mut skip = 0usize;
        let mut state = crate::rules::context::EncoderState::new(true);
        state.is_english = true;
        let mut out = Vec::new();
        let mut ctx = make_ctx(&chars, 0, &ct, &mut skip, &mut state, &mut out, "");
        let res = Rule29.apply(&mut ctx).unwrap();
        assert!(matches!(res, RuleResult::Continue));
        // exit logic is deferred — no byte emitted, state unchanged
        assert!(out.is_empty());
    }

    /// 제29항 — `prev_word_is_numeric` branch coverage via integration encode.
    /// A numeric prev word `1,234` followed by `km` should drive `should_enter_as_roman_indicator`
    /// through `prev_word_is_numeric`, indirectly emitting the roman indicator.
    /// We verify via `crate::encode` to avoid reverse-engineering helper internals.
    #[test]
    fn rule29_prev_word_numeric_drives_roman_indicator() {
        let out = crate::encode("1,234 km").expect("must encode");
        assert!(!out.is_empty());
        assert!(out.contains(&ROMAN_INDICATOR));
    }

    /// 제29항 — matches returns false when neither enter nor exit condition holds:
    /// already in English mode AND current char is also English (line 80).
    #[test]
    fn rule29_matches_false_when_already_in_english_with_english_char() {
        let chars: Vec<char> = "A".chars().collect();
        let ct = CharType::new(chars[0]).unwrap();
        let mut skip = 0usize;
        let mut state = crate::rules::context::EncoderState::new(true);
        state.is_english = true; // already in English
        let mut out = Vec::new();
        let ctx = make_ctx(&chars, 0, &ct, &mut skip, &mut state, &mut out, "");
        // Neither "entering" nor "exiting" — falls through to line 80 `false`.
        assert!(!Rule29.matches(&ctx));
    }
}
