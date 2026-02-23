use aztec_lint_aztec::patterns::is_nullifier_call_name;
use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::SOUNDNESS;

use crate::Rule;
use crate::aztec::text_scan::{call_name, is_note_consume_call_name, scan_functions};
use crate::engine::context::RuleContext;

pub struct Aztec030NoteConsumedWithoutNullifierRule;

impl Rule for Aztec030NoteConsumedWithoutNullifierRule {
    fn id(&self) -> &'static str {
        "AZTEC030"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(_model) = ctx.aztec_model() else {
            return;
        };

        let config = ctx.aztec_config();
        for function in scan_functions(ctx) {
            let mut first_consumption_span = None;
            let mut has_nullifier_emit = false;

            for line in &function.lines {
                let Some(name) = call_name(&line.text) else {
                    continue;
                };

                if is_nullifier_call_name(&name, &config) {
                    has_nullifier_emit = true;
                }
                if is_note_consume_call_name(&name, &line.text) {
                    first_consumption_span.get_or_insert_with(|| line.span.clone());
                }
            }

            if let Some(span) = first_consumption_span
                && !has_nullifier_emit
            {
                out.push(ctx.diagnostic(
                    self.id(),
                    SOUNDNESS,
                    "note appears consumed without a matching nullifier emission in this function",
                    span,
                ));
            }
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

    use super::Aztec030NoteConsumedWithoutNullifierRule;

    #[test]
    fn reports_note_consumption_without_nullifier_emit() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn consume() {
        let note = self.notes.pop_note();
        emit(note);
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
        Aztec030NoteConsumedWithoutNullifierRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_when_nullifier_is_emitted() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn consume() {
        let note = self.notes.pop_note();
        emit_nullifier(note);
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
        Aztec030NoteConsumedWithoutNullifierRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }
}
