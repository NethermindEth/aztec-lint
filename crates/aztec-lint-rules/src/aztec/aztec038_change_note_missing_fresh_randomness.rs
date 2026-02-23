use aztec_lint_core::diagnostics::Diagnostic;

use crate::Rule;
use crate::engine::context::RuleContext;

pub struct Aztec038ChangeNoteMissingFreshRandomnessRule;

impl Rule for Aztec038ChangeNoteMissingFreshRandomnessRule {
    fn id(&self) -> &'static str {
        "AZTEC038"
    }

    fn run(&self, _ctx: &RuleContext<'_>, _out: &mut Vec<Diagnostic>) {}
}
