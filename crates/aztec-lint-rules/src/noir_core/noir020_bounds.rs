use std::collections::BTreeSet;

use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::engine::context::RuleContext;
use crate::noir_core::util::{collect_identifiers, extract_identifiers};

pub struct Noir020BoundsRule;

impl Rule for Noir020BoundsRule {
    fn id(&self) -> &'static str {
        "NOIR020"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            let guarded_indices = collect_guarded_indices(file.text());
            let mut offset = 0usize;

            for line in file.text().lines() {
                for (index_name, column) in indexed_accesses(line) {
                    if guarded_indices.contains(&index_name) {
                        continue;
                    }
                    let start = offset + column;
                    out.push(ctx.diagnostic(
                        self.id(),
                        CORRECTNESS,
                        format!("index `{index_name}` is used without an obvious bounds assertion"),
                        file.span_for_range(start, start + index_name.len()),
                    ));
                }

                offset += line.len() + 1;
            }
        }
    }
}

fn collect_guarded_indices(source: &str) -> BTreeSet<String> {
    let mut guarded = BTreeSet::<String>::new();

    for line in source.lines() {
        if !(line.contains("assert(")
            && line.contains("len()")
            && (line.contains('<') || line.contains("<=")))
        {
            continue;
        }

        for identifier in collect_identifiers(line) {
            if matches!(identifier.as_str(), "assert" | "len") {
                continue;
            }
            guarded.insert(identifier);
        }
    }

    guarded
}

fn indexed_accesses(line: &str) -> Vec<(String, usize)> {
    let mut out = Vec::<(String, usize)>::new();
    let bytes = line.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] != b'[' {
            index += 1;
            continue;
        }
        let mut left = index;
        while left > 0 && bytes[left - 1].is_ascii_whitespace() {
            left -= 1;
        }
        if left == 0
            || !(bytes[left - 1].is_ascii_alphanumeric()
                || bytes[left - 1] == b'_'
                || bytes[left - 1] == b')')
        {
            index += 1;
            continue;
        }
        let Some(close_rel) = line[index + 1..].find(']') else {
            break;
        };
        let close = index + 1 + close_rel;
        let expr = line[index + 1..close].trim();

        if expr.is_empty() || expr.chars().all(|ch| ch.is_ascii_digit()) {
            index = close + 1;
            continue;
        }

        let identifiers = extract_identifiers(expr);
        if identifiers.len() == 1 {
            out.push((identifiers[0].0.clone(), index + 1 + identifiers[0].1));
        }

        index = close + 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir020BoundsRule;

    #[test]
    fn reports_unbounded_indexing() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main(arr: [Field; 4], idx: u32) { let x = arr[idx]; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir020BoundsRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_indexing_with_asserted_guard() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main(arr: [Field; 4], idx: u32) { assert(idx < arr.len()); let x = arr[idx]; }"
                    .to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir020BoundsRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }
}
