//! 제3항 — 기본 자음자 14개가 받침으로 쓰일 때에는 다음과 같이 적는다.
//! 제4항 — 쌍받침 'ㄲ'은 ⠁⠁으로 적고, 쌍받침 'ㅆ'은 약자인 ⠌으로 적는다.
//! 제5항 — 겹받침은 각 받침 글자를 어울러 다음과 같이 적는다.
//!
//! Maps 28 final consonants (jongseong) to braille dot patterns.
//! Includes single, double, and compound final consonants.
//!
//! Encoding is delegated to `jauem::jongseong::encode_jongseong()` which uses a PHF map.
//!
//! Reference: 2024 Korean Braille Standard, Chapter 1, Section 2, Articles 3-5

use crate::jauem::jongseong;
use crate::rules::RuleMeta;
use crate::rules::context::RuleContext;
use crate::rules::traits::{BrailleRule, Phase, RuleResult};

pub static META_3: RuleMeta = RuleMeta {
    section: "3",
    subsection: None,
    name: "basic_jongseong",
    standard_ref: "2024 Korean Braille Standard, Ch.1 Sec.2 Art.3",
    description: "Encode 14 basic final consonants (jongseong) to braille",
};

/// Encode a jongseong character to its braille representation.
/// Re-exports `jauem::jongseong::encode_jongseong`.
#[cfg(test)]
fn apply(jong: char) -> Result<&'static [u8], String> {
    jongseong::encode_jongseong(jong)
}

/// Plugin struct for the rule engine.
///
/// Sub-component rule: encodes the final consonant (jongseong) of a Korean syllable.
/// Covers 제3항 (basic), 제4항 (double: ㄲ→⠁⠁, ㅆ→⠌), and 제5항 (compound).
/// Returns Continue since this is a sub-component of syllable encoding.
pub struct Rule3;

impl BrailleRule for Rule3 {
    fn meta(&self) -> &'static RuleMeta {
        &META_3
    }

    fn phase(&self) -> Phase {
        Phase::CoreEncoding
    }

    fn priority(&self) -> u16 {
        210 // Sub-component — runs after choseong (200) and jungseong
    }

    fn matches(&self, ctx: &RuleContext) -> bool {
        ctx.as_korean().is_some_and(|k| k.jong.is_some())
    }

    fn apply(&self, ctx: &mut RuleContext) -> Result<RuleResult, String> {
        let Some(korean) = ctx.as_korean() else {
            return Ok(RuleResult::Skip);
        };
        if let Some(jong) = korean.jong {
            let encoded = jongseong::encode_jongseong(jong)?;
            ctx.emit_slice(encoded);
        }
        Ok(RuleResult::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unicode::decode_unicode;

    // ── 제3항: basic 14 jongseong ──────────────────────

    #[rstest::rstest]
    #[case::giyeok('ㄱ', '⠁')]
    #[case::nieun('ㄴ', '⠒')]
    #[case::digeut('ㄷ', '⠔')]
    #[case::rieul('ㄹ', '⠂')]
    #[case::mieum('ㅁ', '⠢')]
    #[case::bieup('ㅂ', '⠃')]
    #[case::siot('ㅅ', '⠄')]
    #[case::ieung('ㅇ', '⠶')]
    #[case::jieut('ㅈ', '⠅')]
    #[case::chieut('ㅊ', '⠆')]
    #[case::kieuk('ㅋ', '⠖')]
    #[case::tieut('ㅌ', '⠦')]
    #[case::pieup('ㅍ', '⠲')]
    #[case::hieut('ㅎ', '⠴')]
    fn encodes_basic_jongseong(#[case] jong: char, #[case] expected: char) {
        let result = apply(jong).unwrap();
        assert_eq!(
            result,
            &[decode_unicode(expected)],
            "Failed for jongseong: {jong}"
        );
    }

    // ── 제4항: double jongseong (ㄲ, ㅆ) ──────────────

    #[test]
    fn encodes_double_jongseong_gg() {
        let result = apply('ㄲ').unwrap();
        assert_eq!(result, &[decode_unicode('⠁'), decode_unicode('⠁')]);
    }

    #[test]
    fn encodes_double_jongseong_ss() {
        // ㅆ is abbreviated to ⠌
        let result = apply('ㅆ').unwrap();
        assert_eq!(result, &[decode_unicode('⠌')]);
    }

    // ── 제5항: compound jongseong ──────────────────────

    #[rstest::rstest]
    #[case::giyeok_siot('ㄳ', '⠁', '⠄')]
    #[case::nieun_jieut('ㄵ', '⠒', '⠅')]
    #[case::nieun_hieut('ㄶ', '⠒', '⠴')]
    #[case::rieul_giyeok('ㄺ', '⠂', '⠁')]
    #[case::bieup_siot('ㅄ', '⠃', '⠄')]
    fn encodes_compound_jongseong(#[case] jong: char, #[case] first: char, #[case] second: char) {
        let result = apply(jong).unwrap();
        assert_eq!(
            result,
            &[decode_unicode(first), decode_unicode(second)],
            "Failed for compound jongseong: {jong}"
        );
    }

    #[rstest::rstest]
    #[case::ascii_letter('A')]
    #[case::syllable('가')]
    fn invalid_returns_error(#[case] ch: char) {
        assert!(apply(ch).is_err());
    }

    #[test]
    fn golden_test_alignment() {
        let cases = vec![("국보", "⠈⠍⠁⠘⠥"), ("놋그릇", "⠉⠥⠄⠈⠪⠐⠪⠄")];
        for (input, expected) in cases {
            let result = crate::encode_to_unicode(input).unwrap();
            assert_eq!(result, expected, "Rule 3 golden test failed for: {}", input);
        }
    }

    use rstest::rstest;

    /// matches() must be true only for syllables that have a jongseong (받침).
    #[rstest]
    #[case("국", true)] // 국 has 받침 ㄱ
    #[case("강", true)] // 강 has 받침 ㅇ
    #[case("가", false)] // 가 has no 받침
    #[case("A", false)]
    fn rule3_matches_only_for_syllable_with_jongseong(#[case] input: &str, #[case] expected: bool) {
        let mut owned = crate::test_helpers::CtxOwned::for_text(input, false);
        let ctx = owned.ctx_at(0);
        assert_eq!(Rule3.matches(&ctx), expected, "input={input}");
    }

    #[rstest]
    #[case("국")]
    #[case("강")]
    #[case("님")]
    #[case("닿")]
    fn rule3_apply_emits_jongseong(#[case] input: &str) {
        let mut owned = crate::test_helpers::CtxOwned::for_text(input, false);
        let mut ctx = owned.ctx_at(0);
        let outcome = Rule3.apply(&mut ctx).unwrap();
        assert!(matches!(outcome, RuleResult::Continue));
        assert!(!owned.result.is_empty());
    }

    #[test]
    fn rule3_apply_no_emit_for_syllable_without_jongseong() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("가", false);
        let mut ctx = owned.ctx_at(0);
        let outcome = Rule3.apply(&mut ctx).unwrap();
        assert!(matches!(outcome, RuleResult::Continue));
        assert!(owned.result.is_empty());
    }

    #[test]
    fn rule3_apply_skips_non_korean() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("A", false);
        let mut ctx = owned.ctx_at(0);
        let outcome = Rule3.apply(&mut ctx).unwrap();
        assert!(matches!(outcome, RuleResult::Skip));
    }

    #[test]
    fn rule3_meta_phase_priority() {
        assert_eq!(Rule3.meta().section, "3");
        assert!(matches!(Rule3.phase(), Phase::CoreEncoding));
        assert_eq!(Rule3.priority(), 210);
    }
}
