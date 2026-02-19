use std::collections::BTreeSet;

use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::engine::context::RuleContext;
use crate::noir_core::util::{
    count_identifier_occurrences, extract_identifiers, find_let_bindings,
};

pub struct Noir030UnconstrainedInfluenceRule;

impl Rule for Noir030UnconstrainedInfluenceRule {
    fn id(&self) -> &'static str {
        "NOIR030"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            let unconstrained_fns = unconstrained_functions(file.text());
            let mut tainted = BTreeSet::<String>::new();
            let mut offset = 0usize;

            for line in file.text().lines() {
                for (name, column) in find_let_bindings(line) {
                    let Some(rhs) = assignment_rhs(line, &name, column) else {
                        continue;
                    };
                    let rhs_trimmed = rhs.trim();
                    let influenced = rhs_trimmed.contains("unconstrained")
                        || unconstrained_fns.iter().any(|function_name| {
                            rhs_trimmed.contains(&format!("{function_name}("))
                        });
                    if influenced {
                        tainted.insert(name);
                    }
                }

                if line.contains("assert(") || line.contains("constrain(") {
                    for variable in &tainted {
                        if count_identifier_occurrences(line, variable) > 0 {
                            out.push(ctx.diagnostic(
                                self.id(),
                                CORRECTNESS,
                                format!(
                                    "unconstrained value `{variable}` influences constrained logic"
                                ),
                                file.span_for_range(offset, offset + line.len()),
                            ));
                        }
                    }
                }

                offset += line.len() + 1;
            }
        }
    }
}

fn unconstrained_functions(source: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();

    for line in source.lines() {
        let Some(marker) = line.find("unconstrained fn ") else {
            continue;
        };
        let tail = &line[marker + "unconstrained fn ".len()..];
        if let Some((name, _)) = extract_identifiers(tail).into_iter().next() {
            out.insert(name);
        }
    }

    out
}

fn assignment_rhs<'a>(line: &'a str, name: &str, name_column: usize) -> Option<&'a str> {
    let tail = &line[name_column + name.len()..];
    let equals = tail.find('=')?;
    Some(tail[equals + 1..].trim())
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir030UnconstrainedInfluenceRule;

    #[test]
    fn reports_unconstrained_influence() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                r#"
unconstrained fn read_secret() -> Field { 7 }
fn main() {
    let secret = read_secret();
    assert(secret == 7);
}
"#
                .to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir030UnconstrainedInfluenceRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_constrained_values() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let value = 7; assert(value == 7); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir030UnconstrainedInfluenceRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }
}
