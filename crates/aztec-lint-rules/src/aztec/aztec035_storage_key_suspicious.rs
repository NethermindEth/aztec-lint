use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::model::Span;
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::aztec::text_scan::{extract_double_at_keys, normalize_expression, scan_functions};
use crate::engine::context::RuleContext;

pub struct Aztec035StorageKeySuspiciousRule;

impl Rule for Aztec035StorageKeySuspiciousRule {
    fn id(&self) -> &'static str {
        "AZTEC035"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(_model) = ctx.aztec_model() else {
            return;
        };

        for function in scan_functions(ctx) {
            for line in &function.lines {
                let Some((first, second, second_start, second_end)) =
                    extract_double_at_keys(&line.text)
                else {
                    continue;
                };

                if first.is_empty() || second.is_empty() {
                    continue;
                }
                if normalize_expression(&first) != normalize_expression(&second) {
                    continue;
                }

                out.push(ctx.diagnostic(
                    self.id(),
                    CORRECTNESS,
                    format!(
                        "repeated nested storage key `{}` in `.at(...).at(...)` looks suspicious",
                        second.trim()
                    ),
                    span_slice(&line.span, second_start, second_end),
                ));
            }
        }
    }
}

fn span_slice(line_span: &Span, local_start: usize, local_end: usize) -> Span {
    let line_start = usize::try_from(line_span.start).unwrap_or(0);
    let start = line_start.saturating_add(local_start);
    let end = line_start.saturating_add(local_end.max(local_start));

    Span::new(
        line_span.file.clone(),
        u32::try_from(start).unwrap_or(line_span.start),
        u32::try_from(end).unwrap_or(line_span.end),
        line_span.line,
        line_span.col,
    )
}

#[cfg(test)]
mod tests {
    use aztec_lint_aztec::build_aztec_model;
    use aztec_lint_aztec::detect::SourceUnit;
    use aztec_lint_core::config::AztecConfig;
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Aztec035StorageKeySuspiciousRule;

    #[test]
    fn reports_repeated_nested_storage_key() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn update(from: Field) {
        let slot = self.storage.balances.at(from).at(from);
        emit(slot);
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
        Aztec035StorageKeySuspiciousRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_distinct_nested_storage_keys() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn update(from: Field, spender: Field) {
        let slot = self.storage.balances.at(from).at(spender);
        emit(slot);
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
        Aztec035StorageKeySuspiciousRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }
}
