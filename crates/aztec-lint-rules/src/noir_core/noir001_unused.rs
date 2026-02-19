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
    let clause = line[use_start..]
        .split_once(';')
        .map_or(&line[use_start..], |(prefix, _)| prefix);
    let mut out = Vec::<(String, usize)>::new();
    let mut search_from = 0usize;

    for binding in parse_use_clause_bindings(clause) {
        let Some(relative) = clause[search_from..].find(&binding) else {
            continue;
        };
        let absolute_relative = search_from + relative;
        out.push((binding.clone(), use_start + absolute_relative));
        search_from = absolute_relative + binding.len();
    }

    out
}

fn parse_use_clause_bindings(clause: &str) -> Vec<String> {
    let trimmed = clause.trim();
    let mut out = Vec::<String>::new();

    if let (Some(open), Some(close)) = (trimmed.find('{'), trimmed.rfind('}'))
        && open < close
    {
        let inner = &trimmed[open + 1..close];
        for part in inner.split(',') {
            if let Some(binding) = parse_single_import_binding(part) {
                out.push(binding);
            }
        }
        return out;
    }

    for part in trimmed.split(',') {
        if let Some(binding) = parse_single_import_binding(part) {
            out.push(binding);
        }
    }

    out
}

fn parse_single_import_binding(part: &str) -> Option<String> {
    let trimmed = part.trim();
    if trimmed.is_empty() || trimmed == "*" {
        return None;
    }

    let candidate = trimmed
        .rsplit_once(" as ")
        .map(|(_, alias)| alias.trim())
        .unwrap_or_else(|| trimmed.rsplit("::").next().unwrap_or(trimmed).trim());
    if candidate.is_empty() {
        return None;
    }

    let candidate = candidate.trim_matches('{').trim_matches('}');
    if candidate.is_empty() || matches!(candidate, "crate" | "super" | "self" | "pub" | "*") {
        return None;
    }
    Some(candidate.to_string())
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

    #[test]
    fn ignores_module_prefixes_in_use_paths() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "use math::add;\nfn main() { let x = add(1, 2); assert(x == 3); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }
}
