use std::collections::BTreeSet;

use aztec_lint_aztec::SourceUnit;
use aztec_lint_aztec::taint::{
    TaintSinkKind, TaintSourceKind, analyze_intra_procedural, build_def_use_graph_with_semantic,
};
use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::PRIVACY;

use crate::Rule;
use crate::engine::context::RuleContext;

pub struct Aztec002SecretBranchingRule;

impl Rule for Aztec002SecretBranchingRule {
    fn id(&self) -> &'static str {
        "AZTEC002"
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
        let graph =
            build_def_use_graph_with_semantic(&sources, model, Some(ctx.semantic_model()), &config);
        let analysis = analyze_intra_procedural(&graph);

        let effectful_functions = graph
            .functions
            .iter()
            .filter(|function| {
                function.sinks.iter().any(|sink| {
                    matches!(
                        sink.kind,
                        TaintSinkKind::PublicOutput
                            | TaintSinkKind::PublicStorageWrite
                            | TaintSinkKind::EnqueuePublicCall
                            | TaintSinkKind::OracleArgument
                            | TaintSinkKind::LogEvent
                    )
                })
            })
            .map(|function| function.function_symbol_id.clone())
            .collect::<BTreeSet<_>>();

        for flow in analysis.flows {
            if flow.sink_kind != TaintSinkKind::BranchCondition {
                continue;
            }
            if !matches!(
                flow.source_kind,
                TaintSourceKind::NoteRead
                    | TaintSourceKind::PrivateEntrypointParam
                    | TaintSourceKind::SecretState
            ) {
                continue;
            }
            if !effectful_functions.contains(&flow.function_symbol_id) {
                continue;
            }

            out.push(ctx.diagnostic(
                self.id(),
                PRIVACY,
                format!(
                    "secret-derived value `{}` controls a branch that affects public behavior",
                    flow.variable
                ),
                flow.sink_span,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use aztec_lint_aztec::build_aztec_model;
    use aztec_lint_aztec::detect::SourceUnit;
    use aztec_lint_core::config::AztecConfig;
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Aztec002SecretBranchingRule;

    #[test]
    fn reports_secret_branch_when_public_effect_is_constant() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        if secret > 10 {
            emit(1);
        }
    }
}
"#;
        let project = ProjectModel::default();
        let mut context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        context.set_aztec_config(AztecConfig::default());
        let model = build_aztec_model(
            &[SourceUnit::new("src/main.nr", source)],
            &AztecConfig::default(),
        );
        context.set_aztec_model(model);

        let mut diagnostics = Vec::new();
        Aztec002SecretBranchingRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_secret_branch_without_public_effect() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        if secret > 10 {
            let x = secret + 1;
        }
    }
}
"#;
        let project = ProjectModel::default();
        let mut context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        context.set_aztec_config(AztecConfig::default());
        let model = build_aztec_model(
            &[SourceUnit::new("src/main.nr", source)],
            &AztecConfig::default(),
        );
        context.set_aztec_model(model);

        let mut diagnostics = Vec::new();
        Aztec002SecretBranchingRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }
}
