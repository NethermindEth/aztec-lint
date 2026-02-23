use aztec_lint_core::diagnostics::Diagnostic;

use crate::Rule;
use crate::engine::context::RuleContext;

pub struct Aztec040InitializerNotOnlySelfRule;

impl Rule for Aztec040InitializerNotOnlySelfRule {
    fn id(&self) -> &'static str {
        "AZTEC040"
    }

    fn run(&self, _ctx: &RuleContext<'_>, _out: &mut Vec<Diagnostic>) {}
}
