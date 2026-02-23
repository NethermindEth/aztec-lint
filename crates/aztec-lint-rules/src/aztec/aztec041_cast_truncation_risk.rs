use aztec_lint_core::diagnostics::Diagnostic;

use crate::Rule;
use crate::engine::context::RuleContext;

pub struct Aztec041CastTruncationRiskRule;

impl Rule for Aztec041CastTruncationRiskRule {
    fn id(&self) -> &'static str {
        "AZTEC041"
    }

    fn run(&self, _ctx: &RuleContext<'_>, _out: &mut Vec<Diagnostic>) {}
}
