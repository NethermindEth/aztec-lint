use std::collections::{BTreeMap, BTreeSet};

use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::model::EntrypointKind;
use aztec_lint_core::policy::PROTOCOL;

use crate::Rule;
use crate::aztec::text_scan::{call_name, is_note_consume_call_name, scan_functions};
use crate::engine::context::RuleContext;

pub struct Aztec033PublicMutatesPrivateWithoutOnlySelfRule;

impl Rule for Aztec033PublicMutatesPrivateWithoutOnlySelfRule {
    fn id(&self) -> &'static str {
        "AZTEC033"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(model) = ctx.aztec_model() else {
            return;
        };

        let scanned_by_symbol = scan_functions(ctx)
            .into_iter()
            .map(|function| (function.function_symbol_id.clone(), function))
            .collect::<BTreeMap<_, _>>();

        let mut visited = BTreeSet::<String>::new();
        for entry in &model.entrypoints {
            if entry.kind != EntrypointKind::Public {
                continue;
            }
            if !visited.insert(entry.function_symbol_id.clone()) {
                continue;
            }

            let has_only_self = model.entrypoints.iter().any(|candidate| {
                candidate.contract_id == entry.contract_id
                    && candidate.function_symbol_id == entry.function_symbol_id
                    && candidate.kind == EntrypointKind::OnlySelf
            });
            if has_only_self {
                continue;
            }

            let note_write_span = model
                .note_write_sites
                .iter()
                .find(|site| site.function_symbol_id == entry.function_symbol_id)
                .map(|site| site.span.clone());
            let consume_span =
                scanned_by_symbol
                    .get(&entry.function_symbol_id)
                    .and_then(|function| {
                        function.lines.iter().find_map(|line| {
                            let name = call_name(&line.text)?;
                            if is_note_consume_call_name(&name, &line.text)
                                || looks_like_private_state_transition(&line.text)
                            {
                                Some(line.span.clone())
                            } else {
                                None
                            }
                        })
                    });

            let Some(span) = note_write_span.or(consume_span) else {
                continue;
            };

            out.push(ctx.diagnostic(
                self.id(),
                PROTOCOL,
                "public entrypoint mutates private state without #[only_self]",
                span,
            ));
        }
    }
}

fn looks_like_private_state_transition(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    (lower.contains(".insert(") && lower.contains("deliver("))
        || lower.contains("self.storage") && (lower.contains(".write(") || lower.contains(".set("))
}

#[cfg(test)]
mod tests {
    use aztec_lint_aztec::build_aztec_model;
    use aztec_lint_aztec::detect::SourceUnit;
    use aztec_lint_core::config::AztecConfig;
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Aztec033PublicMutatesPrivateWithoutOnlySelfRule;

    #[test]
    fn reports_public_mutation_without_only_self() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("public")]
    fn rotate() {
        self.storage.notes.insert(1).deliver(0);
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
        Aztec033PublicMutatesPrivateWithoutOnlySelfRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_public_mutation_with_only_self() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("public")]
    #[only_self]
    fn rotate() {
        self.storage.notes.insert(1).deliver(0);
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
        Aztec033PublicMutatesPrivateWithoutOnlySelfRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }
}
