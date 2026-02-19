use std::collections::BTreeSet;

use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::SOUNDNESS;

use crate::Rule;
use crate::engine::context::RuleContext;
use crate::noir_core::util::{
    count_identifier_occurrences, extract_identifiers, find_let_bindings,
};

pub struct Aztec020UnconstrainedInfluenceRule;

impl Rule for Aztec020UnconstrainedInfluenceRule {
    fn id(&self) -> &'static str {
        "AZTEC020"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(model) = ctx.aztec_model() else {
            return;
        };

        let sink_functions = model
            .nullifier_emit_sites
            .iter()
            .chain(model.note_write_sites.iter())
            .map(|site| site.function_symbol_id.clone())
            .collect::<BTreeSet<_>>();
        if sink_functions.is_empty() {
            return;
        }

        for file in ctx.files() {
            let unconstrained_fns = unconstrained_functions(file.text());
            let mut tainted = BTreeSet::<String>::new();
            let mut current_function = String::new();
            let mut current_contract = String::new();
            let mut offset = 0usize;

            for line in file.text().lines() {
                if let Some(contract_name) = contract_name(line) {
                    current_contract = format!("{}::{}", file.path(), contract_name);
                }
                if let Some(function_name) = function_name(line) {
                    if current_contract.is_empty() {
                        current_function.clear();
                    } else {
                        current_function = format!("{current_contract}::fn::{function_name}");
                    }
                    tainted.clear();
                }

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

                if sink_functions.contains(&current_function)
                    && (line.contains("emit_nullifier(")
                        || line.contains("nullify(")
                        || line.contains(".insert("))
                {
                    for value in &tainted {
                        if count_identifier_occurrences(line, value) == 0 {
                            continue;
                        }
                        out.push(ctx.diagnostic(
                            self.id(),
                            SOUNDNESS,
                            format!(
                                "unconstrained value `{value}` influences nullifier/commitment operation"
                            ),
                            file.span_for_range(offset, offset + line.len()),
                        ));
                    }
                }

                offset += line.len() + 1;
            }
        }
    }
}

fn unconstrained_functions(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(function_name)
        .filter(|name| source.contains(&format!("unconstrained fn {name}")))
        .collect()
}

fn function_name(line: &str) -> Option<String> {
    let marker = line.find("fn ")?;
    extract_identifiers(&line[marker + "fn ".len()..])
        .into_iter()
        .next()
        .map(|(name, _)| name)
}

fn contract_name(line: &str) -> Option<String> {
    let marker = line.find("contract ")?;
    extract_identifiers(&line[marker + "contract ".len()..])
        .into_iter()
        .next()
        .map(|(name, _)| name)
}

fn assignment_rhs<'a>(line: &'a str, name: &str, name_column: usize) -> Option<&'a str> {
    let tail = &line[name_column + name.len()..];
    let equals = tail.find('=')?;
    Some(tail[equals + 1..].trim())
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
