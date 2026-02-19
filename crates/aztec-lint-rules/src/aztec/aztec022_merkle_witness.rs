use std::collections::BTreeSet;

use aztec_lint_aztec::SourceUnit;
use aztec_lint_aztec::taint::{TaintSinkKind, analyze_intra_procedural, build_def_use_graph};
use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::SOUNDNESS;

use crate::Rule;
use crate::engine::context::RuleContext;

pub struct Aztec022MerkleWitnessRule;

impl Rule for Aztec022MerkleWitnessRule {
    fn id(&self) -> &'static str {
        "AZTEC022"
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

        let tainted_merkle_functions = analysis
            .flows
            .iter()
            .filter(|flow| flow.sink_kind == TaintSinkKind::MerkleWitness)
            .map(|flow| flow.function_symbol_id.clone())
            .collect::<BTreeSet<_>>();

        for function in &graph.functions {
            let has_merkle_sink = function
                .sinks
                .iter()
                .any(|sink| sink.kind == TaintSinkKind::MerkleWitness);
            if !has_merkle_sink {
                continue;
            }

            let has_verification = function.lines.iter().any(|line| {
                line.text.contains("verify_merkle")
                    || line.text.contains("check_membership")
                    || (line.text.contains("assert(")
                        && (line.text.contains("root")
                            || line.text.contains("path")
                            || line.text.contains("witness")))
            });
            if has_verification {
                continue;
            }

            if !tainted_merkle_functions.contains(&function.function_symbol_id)
                && !function.is_private_entrypoint
            {
                continue;
            }

            for sink in &function.sinks {
                if sink.kind != TaintSinkKind::MerkleWitness {
                    continue;
                }
                out.push(ctx.diagnostic(
                    self.id(),
                    SOUNDNESS,
                    "Merkle witness usage appears unchecked or weakly verified",
                    sink.span.clone(),
                ));
            }
        }
    }
}
