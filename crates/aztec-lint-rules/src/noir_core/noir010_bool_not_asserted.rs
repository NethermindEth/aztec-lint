use std::collections::BTreeSet;

use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::engine::context::RuleContext;
use crate::noir_core::util::{count_identifier_occurrences, find_let_bindings};

pub struct Noir010BoolNotAssertedRule;

impl Rule for Noir010BoolNotAssertedRule {
    fn id(&self) -> &'static str {
        "NOIR010"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            let mut bool_bindings = Vec::<(String, usize)>::new();
            let mut offset = 0usize;

            for line in file.text().lines() {
                for (name, column) in find_let_bindings(line) {
                    let Some(rhs) = assignment_rhs(line, &name, column) else {
                        continue;
                    };
                    if looks_boolean_expression(rhs) {
                        bool_bindings.push((name, offset + column));
                    }
                }

                offset += line.len() + 1;
            }

            let mut asserted = BTreeSet::<String>::new();
            for line in file.text().lines() {
                if !(line.contains("assert(") || line.contains("assert_eq(")) {
                    continue;
                }
                for (name, _) in &bool_bindings {
                    if count_identifier_occurrences(line, name) > 0 {
                        asserted.insert(name.clone());
                    }
                }
            }

            for (name, declaration_offset) in bool_bindings {
                if asserted.contains(&name) {
                    continue;
                }
                out.push(ctx.diagnostic(
                    self.id(),
                    CORRECTNESS,
                    format!("boolean `{name}` is computed but never asserted"),
                    file.span_for_range(declaration_offset, declaration_offset + name.len()),
                ));
            }
        }
    }
}

fn assignment_rhs<'a>(line: &'a str, name: &str, name_column: usize) -> Option<&'a str> {
    let tail = &line[name_column + name.len()..];
    let equals = tail.find('=')?;
    Some(tail[equals + 1..].trim())
}

fn looks_boolean_expression(rhs: &str) -> bool {
    rhs.contains("==")
        || rhs.contains("!=")
        || rhs.contains(">=")
        || rhs.contains("<=")
        || rhs.contains(" > ")
        || rhs.contains(" < ")
        || rhs.contains("true")
        || rhs.contains("false")
        || rhs.trim_start().starts_with('!')
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir010BoolNotAssertedRule;

    #[test]
    fn reports_bool_value_that_is_not_asserted() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let is_valid = 1 == 2; let x = 5; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir010BoolNotAssertedRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_asserted_bool_value() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let is_valid = 1 == 1; assert(is_valid); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir010BoolNotAssertedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }
}
