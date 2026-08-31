//! Math symbol encoding with Korean spacing rules.
//!
//! Math symbols (＋, −, ×, ÷, etc.) need spacing around them when
//! adjacent to Korean text, unless the Korean is a grammatical particle (josa).

use crate::char_struct::CharType;
use crate::math_symbol_shortcut;
use crate::rules::RuleMeta;
use crate::rules::context::RuleContext;
use crate::rules::traits::{BrailleRule, Phase, RuleResult};
use crate::utils;

pub static META: RuleMeta = RuleMeta {
    section: "math",
    subsection: None,
    name: "math_symbol_encoding",
    standard_ref: "2024 Korean Braille Standard (math symbols)",
    description: "Math symbols with Korean spacing rules",
};

/// Korean particles (josa) that should NOT have spacing before them.
const JOSA: &[&str] = &["과", "와", "이다", "하고", "이랑", "와", "랑", "아니다"];

pub struct RuleMath;

impl BrailleRule for RuleMath {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn phase(&self) -> Phase {
        Phase::CoreEncoding
    }

    fn matches(&self, ctx: &RuleContext) -> bool {
        matches!(ctx.char_type, CharType::MathSymbol(_))
    }

    fn apply(&self, ctx: &mut RuleContext) -> Result<RuleResult, String> {
        let CharType::MathSymbol(c) = ctx.char_type else {
            return Ok(RuleResult::Skip);
        };

        // PDF 제46항 — 사칙연산 기호(+, −, ×, ÷, =) 띄어쓰기 규칙.
        // 좌·우가 모두 "한글이 포함된 식"일 때에만 기호 앞뒤를 한 칸씩 띄어 쓴다.
        //
        // 판정:
        //   - 좌측 segment: 단어 시작부터 현재 기호 직전까지의 chars. 한글 포함 여부.
        //   - 우측 segment: 현재 기호 직후부터 단어 끝까지의 chars 중 **선행 비한글을 건너뛴
        //     첫 한글 묶음**. (예: `3.14이다` → `이다`; `3개=2개` → `개`)
        //   - 우측 묶음이 비어 있거나 JOSA(조사: 과/와/이다/하고/이랑/랑/아니다 등)이면
        //     기호 양쪽을 띄어쓰지 않는다.
        //     예: `반지름×3.14이다` → `이다`는 JOSA → 띄어쓰지 않음.
        //     예: `5개−3개=2개` → `개`는 JOSA가 아님 → 띄어씀.
        let prev_has_korean = ctx.word_chars[..ctx.index]
            .iter()
            .any(|c| utils::is_korean_char(*c));

        let next_korean_is_non_josa = {
            let mut korean = Vec::new();
            for wc in &ctx.word_chars[ctx.index + 1..] {
                if utils::is_korean_char(*wc) {
                    korean.push(*wc);
                } else if !korean.is_empty() {
                    break;
                }
            }
            if korean.is_empty() {
                false
            } else {
                let s: String = korean.into_iter().collect();
                !JOSA.contains(&s.as_str())
            }
        };

        // PDF 한글 제49항 — 문장 부호의 띄어쓰기는 묵자를 따른다.
        // `한글(+)한글`처럼 연산 기호가 소괄호에 직접 둘러싸인 경우 기호는
        // 한글 사이에 직접 놓인 것이 아니므로 제46항의 양옆 공백을 삽입하지 않는다.
        // 과학 제21항의 `(-)`·`(+)` 예제도 괄호 안을 붙여 적는다.
        let immediately_parenthesized = ctx.index > 0
            && ctx.word_chars.get(ctx.index - 1) == Some(&'(')
            && ctx.word_chars.get(ctx.index + 1) == Some(&')');
        let pad_spaces = prev_has_korean && next_korean_is_non_josa && !immediately_parenthesized;

        if pad_spaces {
            ctx.emit(0);
        }

        let encoded = math_symbol_shortcut::encode_char_math_symbol_shortcut(*c)?;
        ctx.emit_slice(encoded);

        if pad_spaces {
            ctx.emit(0);
        }

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
        let _ = RuleMath.apply(&mut ctx);
    }

    #[test]
    fn matches_does_not_panic() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("A", false);
        let ctx = owned.ctx_at(0);
        let _ = RuleMath.matches(&ctx);
    }

    #[test]
    fn apply_pads_math_symbol_between_korean_quantity_words() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("5개=2개3", false);
        let mut ctx = owned.ctx_at(2);

        let outcome = RuleMath.apply(&mut ctx).expect("math rule should apply");

        assert!(matches!(outcome, RuleResult::Consumed));
        assert!(owned.result.starts_with(&[0]));
        assert!(owned.result.ends_with(&[0]));
    }

    #[rstest::rstest]
    #[case::plus('+')]
    #[case::times('×')]
    #[case::division('÷')]
    #[case::equals('=')]
    fn parenthesized_math_symbol_does_not_gain_inner_spaces(#[case] operator: char) {
        let input = format!("가({operator})나");
        let mut owned = crate::test_helpers::CtxOwned::for_text(&input, false);
        let mut ctx = owned.ctx_at(2);

        let outcome = RuleMath.apply(&mut ctx).expect("math rule should apply");

        assert!(matches!(outcome, RuleResult::Consumed));
        assert!(!owned.result.is_empty());
        assert_ne!(owned.result.first(), Some(&0));
        assert_ne!(owned.result.last(), Some(&0));
    }

    #[rstest::rstest]
    #[case::plus_math_symbol("양", "+", "극")]
    #[case::ascii_hyphen_minus_symbol("음", "-", "극")]
    fn full_encoder_preserves_tight_parenthesized_operator(
        #[case] left: &str,
        #[case] operator: &str,
        #[case] right: &str,
    ) {
        let input = format!("{left}({operator}){right}");
        let expected = [left, &format!("({operator})"), right]
            .into_iter()
            .map(|part| crate::encode_to_unicode(part).expect("component must encode"))
            .collect::<Vec<_>>()
            .concat();

        assert_eq!(
            crate::encode_to_unicode(&input).expect("full input must encode"),
            expected
        );
    }

    #[test]
    fn ascii_hyphen_minus_is_supported_by_punctuation_rule_49_path() {
        assert!(matches!(
            crate::char_struct::CharType::new('-').expect("hyphen-minus must classify"),
            crate::char_struct::CharType::Symbol('-')
        ));
        assert_eq!(
            crate::encode_to_unicode("음(-)극").expect("full input must encode"),
            ["음", "(-)", "극"]
                .into_iter()
                .map(|part| crate::encode_to_unicode(part).expect("component must encode"))
                .collect::<Vec<_>>()
                .concat()
        );
    }
}
