use aztec_lint_aztec::patterns::is_nullifier_call_name;
use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::PROTOCOL;

use crate::Rule;
use crate::aztec::text_scan::{call_arguments, call_name, hash_like_arguments, scan_functions};
use crate::engine::context::RuleContext;

pub struct Aztec031DomainSepNullifierRule;

impl Rule for Aztec031DomainSepNullifierRule {
    fn id(&self) -> &'static str {
        "AZTEC031"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(_model) = ctx.aztec_model() else {
            return;
        };

        let config = ctx.aztec_config();

        for function in scan_functions(ctx) {
            for line in &function.lines {
                let Some(name) = call_name(&line.text) else {
                    continue;
                };
                if !is_nullifier_call_name(&name, &config) {
                    continue;
                }
                let Some(arguments) = call_arguments(&line.text, &name) else {
                    continue;
                };
                let hash_inputs = hash_like_arguments(&arguments);
                if hash_inputs.is_empty() {
                    continue;
                }

                let missing = hash_inputs.iter().find_map(|input| {
                    let missing = missing_domain_components(
                        input,
                        &config.domain_separation.nullifier_requires,
                    );
                    if missing.is_empty() {
                        None
                    } else {
                        Some(missing)
                    }
                });
                let Some(missing) = missing else {
                    continue;
                };

                out.push(ctx.diagnostic(
                    self.id(),
                    PROTOCOL,
                    format!(
                        "nullifier domain separation appears incomplete; missing: {}",
                        missing.join(", ")
                    ),
                    line.span.clone(),
                ));
            }
        }
    }
}

fn missing_domain_components(arguments: &str, required: &[String]) -> Vec<String> {
    required
        .iter()
        .filter_map(|component| {
            if requirement_is_satisfied(component, arguments) {
                None
            } else {
                Some(component.clone())
            }
        })
        .collect()
}

fn requirement_is_satisfied(component: &str, arguments: &str) -> bool {
    let normalized = arguments.to_ascii_lowercase();
    let requirement = component.trim().to_ascii_lowercase();

    match requirement.as_str() {
        "contract_address" => {
            normalized.contains("contract_address")
                || normalized.contains("this_address")
                || normalized.contains("context.this_address")
        }
        "nonce" => {
            normalized.contains("nonce")
                || normalized.contains("note_id")
                || normalized.contains("note_index")
        }
        "selector" => normalized.contains("selector") || normalized.contains("function_selector"),
        "chain" | "chain_id" => normalized.contains("chain") || normalized.contains("version"),
        _ => normalized.contains(&requirement),
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

    use super::Aztec031DomainSepNullifierRule;

    #[test]
    fn reports_missing_domain_components() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn consume(value: Field) {
        emit_nullifier(hash(value));
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
        Aztec031DomainSepNullifierRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_when_required_components_are_present() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn consume(value: Field, nonce: Field) {
        emit_nullifier(hash(value + self.context.this_address() + nonce));
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
        Aztec031DomainSepNullifierRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_when_component_is_outside_hash_preimage() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn consume(value: Field, nonce: Field) {
        emit_nullifier(hash(value), nonce);
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
        Aztec031DomainSepNullifierRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }
}
