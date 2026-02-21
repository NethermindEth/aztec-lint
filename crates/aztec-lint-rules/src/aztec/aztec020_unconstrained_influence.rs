use aztec_lint_aztec::SourceUnit;
use aztec_lint_aztec::taint::{
    TaintSinkKind, TaintSourceKind, analyze_intra_procedural, build_def_use_graph_with_semantic,
};
use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::SOUNDNESS;

use crate::Rule;
use crate::engine::context::RuleContext;

pub struct Aztec020UnconstrainedInfluenceRule;

impl Rule for Aztec020UnconstrainedInfluenceRule {
    fn id(&self) -> &'static str {
        "AZTEC020"
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

        for flow in analysis.flows {
            if flow.source_kind != TaintSourceKind::UnconstrainedCall {
                continue;
            }
            if !matches!(
                flow.sink_kind,
                TaintSinkKind::NullifierOrCommitment | TaintSinkKind::PublicStorageWrite
            ) {
                continue;
            }
            out.push(ctx.diagnostic(
                self.id(),
                SOUNDNESS,
                format!(
                    "unconstrained value `{}` influences nullifier/commitment or storage write",
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

    use super::Aztec020UnconstrainedInfluenceRule;

    #[test]
    fn reports_unconstrained_flow_to_nullifier() {
        let source = r#"
#[aztec]
pub contract C {
    unconstrained fn read_secret() -> Field { 7 }

    #[external("private")]
    fn bridge() {
        let secret = read_secret();
        emit_nullifier(secret);
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
        Aztec020UnconstrainedInfluenceRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_when_sink_uses_constrained_value() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge() {
        let value = 7;
        emit_nullifier(value);
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
        Aztec020UnconstrainedInfluenceRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }
}
