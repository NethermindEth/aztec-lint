use std::collections::BTreeSet;

use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::engine::context::RuleContext;
use crate::noir_core::util::{count_identifier_occurrences, find_let_bindings};

pub struct Noir001UnusedRule;

impl Rule for Noir001UnusedRule {
    fn id(&self) -> &'static str {
        "NOIR001"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            let source = file.text();
            let mut offset = 0usize;
            let mut seen = BTreeSet::<(String, usize)>::new();

            for line in source.lines() {
                for (name, column) in find_let_bindings(line) {
                    if name.starts_with('_') {
                        continue;
                    }
                    let declaration_offset = offset + column;
                    if !seen.insert((name.clone(), declaration_offset)) {
                        continue;
                    }
                    if count_identifier_occurrences(source, &name) <= 1 {
                        out.push(ctx.diagnostic(
                            self.id(),
                            CORRECTNESS,
                            format!("`{name}` is declared but never used"),
                            file.span_for_range(
                                declaration_offset,
                                declaration_offset + name.len(),
                            ),
                        ));
                    }
                }

                for (name, column) in import_bindings(line) {
                    if name.starts_with('_') {
                        continue;
                    }
                    let declaration_offset = offset + column;
                    if !seen.insert((name.clone(), declaration_offset)) {
                        continue;
                    }
                    if count_identifier_occurrences(source, &name) <= 1 {
                        out.push(ctx.diagnostic(
                            self.id(),
                            CORRECTNESS,
                            format!("import `{name}` is never used"),
                            file.span_for_range(
                                declaration_offset,
                                declaration_offset + name.len(),
                            ),
                        ));
                    }
                }

                offset += line.len() + 1;
            }
        }
    }
}

fn import_bindings(line: &str) -> Vec<(String, usize)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("use ") {
        return Vec::new();
    }

    let use_start = line.find("use ").unwrap_or(0) + "use ".len();
    let clause = &line[use_start..];
    let mut out = Vec::<(String, usize)>::new();

    let mut idx = 0usize;
    let bytes = clause.as_bytes();
    while idx < bytes.len() {
        if !(bytes[idx].is_ascii_alphabetic() || bytes[idx] == b'_') {
            idx += 1;
            continue;
        }

        let start = idx;
        idx += 1;
        while idx < bytes.len() && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == b'_') {
            idx += 1;
        }
        let token = &clause[start..idx];
        if matches!(token, "crate" | "super" | "self" | "as" | "pub") {
            continue;
        }
        out.push((token.to_string(), use_start + start));
    }

    out
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir001UnusedRule;

    #[test]
    fn reports_unused_local_binding() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let value = 7; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("never used"));
    }

    #[test]
    fn ignores_used_bindings() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let value = 7; assert(value == 7); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }
}
