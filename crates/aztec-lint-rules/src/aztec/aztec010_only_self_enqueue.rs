use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::model::EntrypointKind;
use aztec_lint_core::policy::PROTOCOL;

use crate::Rule;
use crate::engine::context::RuleContext;

pub struct Aztec010OnlySelfEnqueueRule;

impl Rule for Aztec010OnlySelfEnqueueRule {
    fn id(&self) -> &'static str {
        "AZTEC010"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(model) = ctx.aztec_model() else {
            return;
        };

        for site in &model.enqueue_sites {
            let Some(target_contract_id) = site.target_contract_id.as_ref() else {
                continue;
            };
            if target_contract_id != &site.source_contract_id {
                continue;
            }
            let source_is_private = model.entrypoints.iter().any(|entry| {
                entry.contract_id == site.source_contract_id
                    && entry.function_symbol_id == site.source_function_symbol_id
                    && entry.kind == EntrypointKind::Private
            });
            if !source_is_private {
                continue;
            }
            if site.target_function_name.is_empty() {
                continue;
            }

            let target_symbol = format!(
                "{}::fn::{}",
                site.source_contract_id, site.target_function_name
            );
            let is_public = model.entrypoints.iter().any(|entry| {
                entry.contract_id == site.source_contract_id
                    && entry.function_symbol_id == target_symbol
                    && entry.kind == EntrypointKind::Public
            });
            if !is_public {
                continue;
            }
            let has_only_self = model.entrypoints.iter().any(|entry| {
                entry.contract_id == site.source_contract_id
                    && entry.function_symbol_id == target_symbol
                    && entry.kind == EntrypointKind::OnlySelf
            });
            if has_only_self {
                continue;
            }

            out.push(ctx.diagnostic(
                self.id(),
                PROTOCOL,
                format!(
                    "same-contract enqueue target `{}` must declare #[only_self]",
                    site.target_function_name
                ),
                site.span.clone(),
            ));
        }
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

    use super::Aztec010OnlySelfEnqueueRule;

    #[test]
    fn reports_same_contract_public_target_without_only_self() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge() {
        self.enqueue(Contract::at(self.context.this_address()).mint_public(1));
    }

    #[external("public")]
    fn mint_public(value: Field) {
        emit(value);
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
        Aztec010OnlySelfEnqueueRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_target_with_only_self() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge() {
        self.enqueue(Contract::at(self.context.this_address()).mint_public(1));
    }

    #[external("public")]
    #[only_self]
    fn mint_public(value: Field) {
        emit(value);
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
        Aztec010OnlySelfEnqueueRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_when_source_is_not_private_bridge() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("public")]
    fn trigger() {
        self.enqueue(Contract::at(self.context.this_address()).mint_public(1));
    }

    #[external("public")]
    fn mint_public(value: Field) {
        emit(value);
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
        Aztec010OnlySelfEnqueueRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }
}
