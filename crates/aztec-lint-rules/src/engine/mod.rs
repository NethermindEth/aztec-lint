pub mod context;
pub mod registry;

use std::collections::BTreeMap;

use aztec_lint_core::config::RuleLevel;
use aztec_lint_core::diagnostics::{Diagnostic, Severity, sort_diagnostics};

use self::context::RuleContext;
use self::registry::{RuleRegistration, noir_core_registry};

pub trait Rule {
    fn id(&self) -> &'static str;
    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>);
}

#[derive(Clone, Debug, Default)]
pub struct RuleRunSettings {
    pub effective_levels: BTreeMap<String, RuleLevel>,
}

pub struct RuleEngine {
    registry: Vec<RuleRegistration>,
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            registry: noir_core_registry(),
        }
    }

    pub fn with_registry(registry: Vec<RuleRegistration>) -> Self {
        Self { registry }
    }

    pub fn run_with_settings(
        &self,
        ctx: &RuleContext<'_>,
        settings: &RuleRunSettings,
    ) -> Vec<Diagnostic> {
        self.run(ctx, &settings.effective_levels)
    }

    pub fn run(
        &self,
        ctx: &RuleContext<'_>,
        effective_levels: &BTreeMap<String, RuleLevel>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::<Diagnostic>::new();

        for registration in &self.registry {
            let level = effective_levels
                .get(registration.metadata.id)
                .copied()
                .unwrap_or(registration.metadata.default_level);
            if level == RuleLevel::Allow {
                continue;
            }

            let start = diagnostics.len();
            registration.rule.run(ctx, &mut diagnostics);

            for diagnostic in diagnostics.iter_mut().skip(start) {
                diagnostic.rule_id = registration.metadata.id.to_string();
                diagnostic.severity = level_to_severity(level);
                diagnostic.confidence = registration.metadata.confidence;
                diagnostic.policy = registration.metadata.policy.to_string();

                if let Some(reason) =
                    ctx.suppression_reason(registration.metadata.id, &diagnostic.primary_span)
                {
                    diagnostic.suppressed = true;
                    diagnostic.suppression_reason = Some(reason.to_string());
                }
            }
        }

        sort_diagnostics(&mut diagnostics);
        diagnostics
    }
}

fn level_to_severity(level: RuleLevel) -> Severity {
    match level {
        RuleLevel::Allow => Severity::Warning,
        RuleLevel::Warn => Severity::Warning,
        RuleLevel::Deny => Severity::Error,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use aztec_lint_core::config::RuleLevel;
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;
    use crate::engine::registry::{RuleMetadata, RuleRegistration};

    use super::RuleEngine;

    struct TestRule;

    impl Rule for TestRule {
        fn id(&self) -> &'static str {
            "NOIR100"
        }

        fn run(
            &self,
            ctx: &RuleContext<'_>,
            out: &mut Vec<aztec_lint_core::diagnostics::Diagnostic>,
        ) {
            let file = &ctx.files()[0];
            out.push(ctx.diagnostic(
                self.id(),
                aztec_lint_core::policy::MAINTAINABILITY,
                "magic number",
                file.span_for_range(45, 47),
            ));
        }
    }

    #[test]
    fn engine_applies_metadata_and_suppressions() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                r#"
#[allow(noir_core::NOIR100)]
fn main() {
    let x = 42;
}
"#
                .to_string(),
            )],
        );

        let engine = RuleEngine::with_registry(vec![RuleRegistration {
            metadata: RuleMetadata {
                id: "NOIR100",
                pack: "noir_core",
                policy: aztec_lint_core::policy::MAINTAINABILITY,
                default_level: RuleLevel::Warn,
                confidence: aztec_lint_core::diagnostics::Confidence::Low,
            },
            rule: Box::new(TestRule),
        }]);

        let diagnostics = engine.run(&context, &BTreeMap::new());
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].suppressed);
        assert_eq!(
            diagnostics[0].suppression_reason.as_deref(),
            Some("allow(NOIR100)")
        );
    }
}
