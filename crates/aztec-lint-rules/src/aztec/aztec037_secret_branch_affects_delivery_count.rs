use aztec_lint_core::diagnostics::Diagnostic;

use crate::Rule;
use crate::engine::context::RuleContext;

pub struct Aztec037SecretBranchAffectsDeliveryCountRule;

impl Rule for Aztec037SecretBranchAffectsDeliveryCountRule {
    fn id(&self) -> &'static str {
        "AZTEC037"
    }

    fn run(&self, _ctx: &RuleContext<'_>, _out: &mut Vec<Diagnostic>) {}
}
