use aztec_lint_aztec::SourceUnit;
use aztec_lint_aztec::taint::{TaintSinkKind, build_def_use_graph};
use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::PRIVACY;

use crate::Rule;
use crate::engine::context::RuleContext;

pub struct Aztec003PrivateDebugLogRule;

impl Rule for Aztec003PrivateDebugLogRule {
    fn id(&self) -> &'static str {
        "AZTEC003"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(model) = ctx.aztec_model() else {
            return;
        };

        let config = ctx.aztec_config();
        let sources = ctx
            .files()
            .iter()
            .map(|file| SourceUnit::new(file.path().to_string(), file.text().to_string()))
            .collect::<Vec<_>>();
        let graph = build_def_use_graph(&sources, model, &config);

        for function in &graph.functions {
            if !function.is_private_entrypoint {
                continue;
            }
            for sink in &function.sinks {
                if sink.kind != TaintSinkKind::DebugLog {
                    continue;
                }
                out.push(ctx.diagnostic(
                    self.id(),
                    PRIVACY,
                    "private entrypoint contains debug logging",
                    sink.span.clone(),
                ));
            }
        }
    }
}
