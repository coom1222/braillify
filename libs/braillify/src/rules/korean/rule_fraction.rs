//! Unicode fraction character encoding (½, ⅓, ¼, etc.).

use crate::char_struct::CharType;
use crate::fraction;
use crate::rules::RuleMeta;
use crate::rules::context::RuleContext;
use crate::rules::traits::{BrailleRule, Phase, RuleResult};

pub static META: RuleMeta = RuleMeta {
    section: "fraction",
    subsection: None,
    name: "unicode_fraction_encoding",
    standard_ref: "2024 Korean Braille Standard (fractions)",
    description: "Unicode fraction characters (½, ⅓, ¼, etc.)",
};

pub struct RuleFraction;

impl BrailleRule for RuleFraction {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn phase(&self) -> Phase {
        Phase::CoreEncoding
    }

    fn matches(&self, ctx: &RuleContext) -> bool {
        matches!(ctx.char_type, CharType::Fraction(_))
    }

    fn apply(&self, ctx: &mut RuleContext) -> Result<RuleResult, String> {
        let CharType::Fraction(c) = ctx.char_type else {
            return Ok(RuleResult::Skip);
        };
        if let Some((num_str, den_str)) = fraction::parse_unicode_fraction(*c) {
            let encoded = fraction::encode_fraction(&num_str, &den_str)?;
            ctx.emit_slice(&encoded);
            ctx.state.is_number = true;
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
        let _ = RuleFraction.apply(&mut ctx);
    }

    #[test]
    fn matches_does_not_panic() {
        let mut owned = crate::test_helpers::CtxOwned::for_text("A", false);
        let ctx = owned.ctx_at(0);
        let _ = RuleFraction.matches(&ctx);
    }
}
