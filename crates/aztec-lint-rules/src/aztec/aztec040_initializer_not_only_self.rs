use std::collections::BTreeSet;

use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::model::EntrypointKind;
use aztec_lint_core::policy::PROTOCOL;

use crate::Rule;
use crate::engine::context::RuleContext;

pub struct Aztec040InitializerNotOnlySelfRule;

impl Rule for Aztec040InitializerNotOnlySelfRule {
    fn id(&self) -> &'static str {
        "AZTEC040"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(model) = ctx.aztec_model() else {
            return;
        };

        let mut visited = BTreeSet::<(String, String)>::new();
        for entry in &model.entrypoints {
            if entry.kind != EntrypointKind::Initializer {
                continue;
            }
            if is_constructor_style_initializer(&entry.function_symbol_id) {
                continue;
            }

            let key = (entry.contract_id.clone(), entry.function_symbol_id.clone());
            if !visited.insert(key) {
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

            out.push(ctx.diagnostic(
                self.id(),
                PROTOCOL,
                "initializer entrypoint missing #[only_self]",
                entry.span.clone(),
            ));
        }
    }
}

fn is_constructor_style_initializer(function_symbol_id: &str) -> bool {
    let Some((_, function_name)) = function_symbol_id.rsplit_once("::fn::") else {
        return false;
    };
    let normalized = function_name.to_ascii_lowercase();
    normalized == "constructor" || normalized.starts_with("constructor_")
}

#[cfg(test)]
mod tests {
    use aztec_lint_aztec::build_aztec_model;
    use aztec_lint_aztec::detect::SourceUnit;
    use aztec_lint_core::config::AztecConfig;
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Aztec040InitializerNotOnlySelfRule;

    #[test]
    fn reports_initializer_without_only_self() {
        let source = r#"
#[aztec]
pub contract C {
    #[initializer]
    #[external("public")]
    fn init(owner: Field) {
        emit(owner);
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
        Aztec040InitializerNotOnlySelfRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_initializer_with_only_self() {
        let source = r#"
#[aztec]
pub contract C {
    #[initializer]
    #[external("public")]
    #[only_self]
    fn init(owner: Field) {
        emit(owner);
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
        Aztec040InitializerNotOnlySelfRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_constructor_style_initializer_without_only_self() {
        let source = r#"
#[aztec]
pub contract C {
    #[initializer]
    #[external("public")]
    fn constructor_with_asset(owner: Field) {
        emit(owner);
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
        Aztec040InitializerNotOnlySelfRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }
}
