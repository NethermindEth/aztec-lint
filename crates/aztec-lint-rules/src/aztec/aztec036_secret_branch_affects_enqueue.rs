use aztec_lint_core::diagnostics::Diagnostic;

use crate::Rule;
use crate::engine::context::RuleContext;

pub struct Aztec036SecretBranchAffectsEnqueueRule;

impl Rule for Aztec036SecretBranchAffectsEnqueueRule {
    fn id(&self) -> &'static str {
        "AZTEC036"
    }

    fn run(&self, _ctx: &RuleContext<'_>, _out: &mut Vec<Diagnostic>) {}
}
