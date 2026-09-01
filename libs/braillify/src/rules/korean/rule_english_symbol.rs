//! English-context symbol handling.
//!
//! Handles symbol behavior that depends on English mode state:
//! - English symbol rendering for (, ), , when context requires
//! - Parenthesis stack push/pop for matching English parentheses
//! - Comma before Korean fallback preservation

use crate::char_struct::CharType;
use crate::english_logic;
use crate::rules::RuleMeta;
use crate::rules::context::RuleContext;
use crate::rules::traits::{BrailleRule, Phase, RuleResult};
use crate::symbol_shortcut;
use crate::utils;

pub static META: RuleMeta = RuleMeta {
    section: "49",
    subsection: Some("eng"),
    name: "english_symbol_context",
    standard_ref: "2024 Korean Braille Standard, Ch.4 Sec.10 + Ch.6 Sec.13",
    description: "English-context punctuation rendering with parenthesis tracking",
};

pub struct RuleEnglishSymbol;

impl BrailleRule for RuleEnglishSymbol {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn phase(&self) -> Phase {
        Phase::CoreEncoding
    }

    fn priority(&self) -> u16 {
        300
    }

    fn matches(&self, ctx: &RuleContext) -> bool {
        matches!(ctx.char_type, CharType::Symbol(_))
    }

    fn apply(&self, ctx: &mut RuleContext) -> Result<RuleResult, String> {
        let CharType::Symbol(sym) = ctx.char_type else {
            return Ok(RuleResult::Skip);
        };

        // 한글 점자 제43항·제48항: ASCII 숫자 사이의 마침표는 숫자 흐름의
        // 소수점이다. 같은 어절 뒤쪽에 로마자가 있다는 이유만으로 이 위치에서
        // 로마자 모드에 재진입하면 `⠴⠲`가 되어 수표 뒤의 올바른 `⠲` 앞에
        // 불필요한 로마자표가 붙는다. 이 기호는 아래의 일반 한글 문장부호
        // 규칙이 처리하도록 넘기고, 접미사 종류에는 관여하지 않는다.
        if *sym == '.'
            && ctx.prev_char().is_some_and(|ch| ch.is_ascii_digit())
            && ctx.next_char().is_some_and(|ch| ch.is_ascii_digit())
        {
            return Ok(RuleResult::Continue);
        }

        let mut use_english_symbol = english_logic::should_render_symbol_as_english(
            ctx.state.english_indicator,
            ctx.state.is_english,
            &ctx.state.parenthesis_stack,
            *sym,
            ctx.word_chars,
            ctx.index,
            ctx.remaining_words,
        );

        // 제39항 영-한 wrap context: 단어 끝의 영어 모드 유지 가능 기호(. , : ;)
        // 다음에 한글 어절(wrap 대상)이 이어지면 그 기호를 영어 점자로 처리한다.
        // 예) "(Korean:" 끝의 ':'은 다음 wrap된 "반찬" 직전이므로 영어 점자 ⠒.
        if !use_english_symbol
            && ctx.state.english_dominant_wrap_active
            && ctx.state.is_english
            && ctx.index == ctx.word_chars.len() - 1
            && matches!(*sym, '.' | ',' | ':' | ';')
            && let Some(next_word) = ctx.remaining_words.first()
            && next_word.chars().next().is_some_and(utils::is_korean_char)
        {
            use_english_symbol = true;
        }

        if *sym == '(' {
            ctx.state.parenthesis_stack.push(use_english_symbol);
        } else if *sym == ')' {
            use_english_symbol = ctx
                .state
                .parenthesis_stack
                .pop()
                .unwrap_or(use_english_symbol);
        }

        let has_ascii_alphabetic = ctx.word_chars.iter().any(|ch| ch.is_ascii_alphabetic());
        let can_use_english_symbol = ctx.state.is_english || has_ascii_alphabetic;

        if ctx.state.english_indicator && can_use_english_symbol && use_english_symbol {
            if !ctx.state.is_english && !ctx.state.needs_english_continuation {
                ctx.emit(52);
                ctx.state.is_english = true;
                ctx.state.needs_english_continuation = false;
            }
            if let Some(encoded) = symbol_shortcut::encode_english_char_symbol_shortcut(*sym) {
                ctx.emit_slice(&encoded);
                if *sym == '-' && ctx.state.is_english {
                    // UEB 5.7.2의 `CD-ROM`은 순수 대문자 segment 사이의 하이픈
                    // 뒤에서 대문자 단어표 앞에 1급 점자 기호표를 다시 적지
                    // 않는다. 숫자는 제35항 `D-100`처럼 수표가 나오므로 역시
                    // 로마자 연속표(⠰)가 불필요하다. 혼합 대소문자 prefix와 단일
                    // 대문자 suffix는 이 근거 범위 밖이므로 기존 경계를 보존한다.
                    let prefix_len = ctx.word_chars[..ctx.index]
                        .iter()
                        .rev()
                        .take_while(|c| c.is_ascii_alphabetic())
                        .count();
                    let prefix = &ctx.word_chars[ctx.index - prefix_len..ctx.index];
                    let suffix = &ctx.word_chars[ctx.index + 1..];
                    let suffix_len = suffix
                        .iter()
                        .take_while(|c| c.is_ascii_alphabetic())
                        .count();
                    let suffix_letters = &suffix[..suffix_len];
                    let next_has_own_indicator = suffix.first().is_some_and(char::is_ascii_digit)
                        || (!prefix.is_empty()
                            && prefix.iter().all(char::is_ascii_uppercase)
                            && suffix_letters.len() >= 2
                            && suffix_letters.iter().all(char::is_ascii_uppercase));
                    if !next_has_own_indicator {
                        ctx.emit(crate::rules::korean::rule_29::ENGLISH_CONTINUATION);
                    }
                }
                return Ok(RuleResult::Consumed);
            }
        }

        if *sym == ',' {
            let next_char = ctx
                .next_char()
                .or_else(|| ctx.remaining_words.first().and_then(|w| w.chars().next()));
            if next_char.is_some_and(utils::is_korean_char) {
                ctx.emit_slice(symbol_shortcut::encode_char_symbol_shortcut(*sym)?);
                return Ok(RuleResult::Consumed);
            }
        }

        Ok(RuleResult::Continue)
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
        let _ = RuleEnglishSymbol.apply(&mut ctx);
    }

    #[test]
    fn matches_does_not_panic() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("A", false);
        let ctx = owned.ctx_at(0);
        let _ = RuleEnglishSymbol.matches(&ctx);
    }

    #[test]
    fn opening_parenthesis_pushes_symbol_mode() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("(", false);
        let mut ctx = owned.ctx_at(0);

        let _ = RuleEnglishSymbol.apply(&mut ctx);

        assert!(!ctx.state.parenthesis_stack.is_empty());
    }

    #[test]
    fn closing_parenthesis_reuses_opening_parenthesis_symbol_mode() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("()", true);
        {
            let mut ctx = owned.ctx_at(0);
            ctx.state.is_english = true;

            let _ = RuleEnglishSymbol.apply(&mut ctx);
            assert_eq!(ctx.state.parenthesis_stack.len(), 1);
        }

        let mut ctx = owned.ctx_at(1);
        ctx.state.is_english = true;

        let _ = RuleEnglishSymbol.apply(&mut ctx);

        assert!(ctx.state.parenthesis_stack.is_empty());
    }

    #[test]
    fn decimal_point_between_digits_is_not_an_english_entry_symbol() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("3.5P", false);
        let mut ctx = owned.ctx_at(1);

        let outcome = RuleEnglishSymbol.apply(&mut ctx).unwrap();

        assert_eq!(outcome, RuleResult::Continue);
        assert!(owned.result.is_empty());
    }

    #[rstest::rstest]
    #[case::percent_with_later_roman("42.2%포인트(P)", "42.2")]
    #[case::korean_unit_with_annotation("34.3리터(L)", "34.3")]
    #[case::two_decimals_with_arrow("99.8→99.4", "99.8")]
    #[case::roman_identifier("GPT-3.5", "3.5")]
    fn decimal_subsequence_matches_the_standalone_rule_48_encoding(
        #[case] input: &str,
        #[case] decimal: &str,
    ) {
        let actual = crate::encode_to_unicode(input).expect("mixed decimal context must encode");
        let standalone =
            crate::encode_to_unicode(decimal).expect("standalone rule-48 decimal must encode");

        assert!(
            actual.contains(&standalone),
            "input={input}, decimal={decimal}, actual={actual}, standalone={standalone}"
        );
    }

    /// UEB 5.7.2 prints `CD-ROM` with one grade-1 indicator before the complete
    /// letters-sequence and no second grade-1 indicator after the hyphen. This
    /// full-encoder wrapper exercises the Korean rule-29 character route rather
    /// than the standalone-English token route used by the standard PDF case.
    #[test]
    fn korean_wrapper_keeps_pdf_cd_rom_as_one_grade1_letters_sequence() {
        let output = crate::encode("가(CD-ROM)나").expect("Korean wrapper must encode");
        let expected_ueb = "⠰⠠⠠⠉⠙⠤⠠⠠⠗⠕⠍"
            .chars()
            .map(crate::unicode::decode_unicode)
            .collect::<Vec<_>>();
        let roman_start = output
            .iter()
            .position(|cell| *cell == crate::rules::korean::rule_29::ROMAN_INDICATOR)
            .expect("Korean wrapper must enter one Roman section")
            + 1;

        assert_eq!(
            output.get(roman_start..roman_start + expected_ueb.len()),
            Some(expected_ueb.as_slice())
        );
    }
}
