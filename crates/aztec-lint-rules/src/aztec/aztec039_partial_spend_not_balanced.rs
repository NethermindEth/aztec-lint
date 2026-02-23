use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::aztec::text_scan::scan_functions;
use crate::engine::context::RuleContext;

pub struct Aztec039PartialSpendNotBalancedRule;

impl Rule for Aztec039PartialSpendNotBalancedRule {
    fn id(&self) -> &'static str {
        "AZTEC039"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(_model) = ctx.aztec_model() else {
            return;
        };

        for function in scan_functions(ctx) {
            let has_reconciliation_assert = function
                .lines
                .iter()
                .any(|line| is_reconciliation_assert(&line.text));
            let has_spend_terms = function.lines.iter().any(|line| {
                let lower = line.text.to_ascii_lowercase();
                lower.contains("spent")
                    || lower.contains("transferred")
                    || lower.contains("total_transferred")
            });

            let mut has_guard = false;
            let mut change_line_span = None;
            let mut reported = false;

            for line in &function.lines {
                if is_guard_for_note_subtraction(&line.text) {
                    has_guard = true;
                    continue;
                }

                if is_change_assignment(&line.text) {
                    change_line_span = Some(line.span.clone());
                }

                if !has_guard
                    && (is_unchecked_remaining_subtraction(&line.text)
                        || is_unchecked_change_subtraction(&line.text))
                {
                    out.push(ctx.diagnostic(
                        self.id(),
                        CORRECTNESS,
                        "partial spend arithmetic appears unguarded and may be unbalanced",
                        line.span.clone(),
                    ));
                    reported = true;
                    break;
                }
            }

            if reported {
                continue;
            }

            if has_spend_terms && !has_reconciliation_assert {
                let Some(change_span) = change_line_span else {
                    continue;
                };
                out.push(ctx.diagnostic(
                    self.id(),
                    CORRECTNESS,
                    "partial spend flow appears to miss a reconciliation assertion",
                    change_span,
                ));
            }
        }
    }
}

fn is_guard_for_note_subtraction(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if !lower.contains("assert(") {
        return false;
    }
    let compact = lower
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect::<String>();
    compact.contains("remaining>=note.amount") || compact.contains("note.amount<=remaining")
}

fn is_unchecked_remaining_subtraction(line: &str) -> bool {
    let compact = line
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect::<String>();
    compact.contains("remaining-=note.amount")
        || compact.contains("remaining=remaining-note.amount")
}

fn is_unchecked_change_subtraction(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    (lower.contains("change =") || lower.contains("let change ="))
        && lower.contains("note.amount")
        && lower.contains("- remaining")
}

fn is_change_assignment(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    (lower.contains("change =") || lower.contains("let change =")) && lower.contains("note.amount")
}

fn is_reconciliation_assert(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("assert(")
        && lower.contains("change")
        && lower.contains("note.amount")
        && (lower.contains("spent")
            || lower.contains("transferred")
            || lower.contains("total")
            || lower.contains('+'))
}

#[cfg(test)]
mod tests {
    use aztec_lint_aztec::build_aztec_model;
    use aztec_lint_aztec::detect::SourceUnit;
    use aztec_lint_core::config::AztecConfig;
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Aztec039PartialSpendNotBalancedRule;

    #[test]
    fn reports_unguarded_remaining_subtraction() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn spend(note: Field, mut remaining: Field) {
        remaining -= note.amount;
        emit(remaining);
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
        Aztec039PartialSpendNotBalancedRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_guarded_and_reconciled_partial_spend() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn spend(note: Field, mut remaining: Field, spent: Field) {
        assert(remaining >= note.amount);
        remaining -= note.amount;
        let change = note.amount - remaining;
        assert(spent + change == note.amount);
        emit(change);
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
        Aztec039PartialSpendNotBalancedRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_remaining_subtraction_when_guard_direction_is_wrong() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn spend(note: Field, mut remaining: Field) {
        assert(remaining <= note.amount);
        remaining -= note.amount;
        emit(remaining);
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
        Aztec039PartialSpendNotBalancedRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn reports_equivalent_remaining_assignment_subtraction_without_guard() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn spend(note: Field, mut remaining: Field) {
        remaining = remaining - note.amount;
        emit(remaining);
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
        Aztec039PartialSpendNotBalancedRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }
}
