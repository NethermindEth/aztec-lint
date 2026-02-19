use aztec_lint_aztec::SourceUnit;
use aztec_lint_aztec::taint::{
    TaintSinkKind, TaintSourceKind, analyze_intra_procedural, build_def_use_graph,
};
use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::SOUNDNESS;

use crate::Rule;
use crate::engine::context::RuleContext;

pub struct Aztec021RangeBeforeHashRule;

impl Rule for Aztec021RangeBeforeHashRule {
    fn id(&self) -> &'static str {
        "AZTEC021"
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
        let analysis = analyze_intra_procedural(&graph);

        for flow in analysis.flows {
            if flow.sink_kind != TaintSinkKind::HashOrSerialize {
                continue;
            }
            if !matches!(
                flow.source_kind,
                TaintSourceKind::NoteRead
                    | TaintSourceKind::PrivateEntrypointParam
                    | TaintSourceKind::SecretState
                    | TaintSourceKind::UnconstrainedCall
            ) {
                continue;
            }

            out.push(ctx.diagnostic(
                self.id(),
                SOUNDNESS,
                format!(
                    "missing range constraint before hashing/serialization of `{}`",
                    flow.variable
                ),
                flow.sink_span,
            ));
        }
    }
}
