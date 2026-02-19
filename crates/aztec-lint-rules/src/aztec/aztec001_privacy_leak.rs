use std::collections::BTreeSet;

use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::PRIVACY;

use crate::Rule;
use crate::engine::context::RuleContext;

pub struct Aztec001PrivacyLeakRule;

impl Rule for Aztec001PrivacyLeakRule {
    fn id(&self) -> &'static str {
        "AZTEC001"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(model) = ctx.aztec_model() else {
            return;
        };

        let tainted_functions = model
            .note_read_sites
            .iter()
            .map(|site| site.function_symbol_id.clone())
            .collect::<BTreeSet<_>>();
        if tainted_functions.is_empty() {
            return;
        }

        for sink in &model.public_sinks {
            if !tainted_functions.contains(&sink.function_symbol_id) {
                continue;
            }
            out.push(ctx.diagnostic(
                self.id(),
                PRIVACY,
                "private note-derived data reaches a public sink in the same function",
                sink.span.clone(),
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

    use super::Aztec001PrivacyLeakRule;

    #[test]
    fn reports_note_flow_to_public_sink() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge() {
        let notes = self.notes.get_notes();
        emit(notes);
    }
}
"#;
        let project = ProjectModel::default();
        let mut context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let model = build_aztec_model(
            &[SourceUnit::new("src/main.nr", source)],
            &AztecConfig::default(),
        );
        context.set_aztec_model(model);

        let mut diagnostics = Vec::new();
        Aztec001PrivacyLeakRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_when_no_note_read() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge() {
        let value = 7;
        emit(value);
    }
}
"#;
        let project = ProjectModel::default();
        let mut context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let model = build_aztec_model(
            &[SourceUnit::new("src/main.nr", source)],
            &AztecConfig::default(),
        );
        context.set_aztec_model(model);

        let mut diagnostics = Vec::new();
        Aztec001PrivacyLeakRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }
}
