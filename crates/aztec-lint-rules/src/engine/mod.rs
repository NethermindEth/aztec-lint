pub mod context;
pub mod registry;

use std::collections::{BTreeMap, BTreeSet};

use aztec_lint_core::config::RuleLevel;
use aztec_lint_core::diagnostics::{Diagnostic, Severity, sort_diagnostics};
use aztec_lint_core::lints::{all_lints, find_lint};

use self::context::RuleContext;
use self::registry::{RuleRegistration, full_registry};

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
        let registry = full_registry();
        validate_registry_metadata(&registry);
        validate_registry_integrity_with_catalog(&registry);
        Self { registry }
    }

    pub fn with_registry(registry: Vec<RuleRegistration>) -> Self {
        validate_registry_metadata(&registry);
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
            let Some(level) = effective_levels.get(registration.lint.id).copied() else {
                continue;
            };
            if level == RuleLevel::Allow {
                continue;
            }

            // Run each rule against an isolated output buffer so a rule cannot
            // mutate diagnostics emitted by previously executed rules.
            let mut rule_diagnostics = Vec::<Diagnostic>::new();
            registration.rule.run(ctx, &mut rule_diagnostics);

            for diagnostic in &mut rule_diagnostics {
                diagnostic.rule_id = registration.lint.id.to_string();
                diagnostic.severity = level_to_severity(level);
                diagnostic.confidence = registration.lint.confidence;
                diagnostic.policy = registration.lint.policy.to_string();

                if let Some(reason) =
                    ctx.suppression_reason(registration.lint.id, &diagnostic.primary_span)
                {
                    diagnostic.suppressed = true;
                    diagnostic.suppression_reason = Some(reason.to_string());
                }
            }

            diagnostics.extend(rule_diagnostics);
        }

        sort_diagnostics(&mut diagnostics);
        diagnostics
    }
}

fn validate_registry_metadata(registry: &[RuleRegistration]) {
    let mut seen_rule_ids = BTreeSet::<&'static str>::new();

    for registration in registry {
        let lint = registration.lint;
        let normalized_rule_id = lint.id.trim().to_ascii_uppercase();
        assert!(
            !normalized_rule_id.is_empty(),
            "rule metadata id cannot be empty"
        );
        assert_eq!(
            lint.id, normalized_rule_id,
            "rule metadata id '{}' must be canonical uppercase",
            lint.id
        );
        assert!(
            normalized_rule_id
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_'),
            "rule metadata id '{}' contains unsupported characters",
            lint.id
        );
        assert!(
            seen_rule_ids.insert(lint.id),
            "duplicate rule metadata id '{}'",
            lint.id
        );

        assert!(
            is_pack_name_canonical(lint.pack),
            "rule metadata pack '{}' must be lowercase snake_case",
            lint.pack
        );
        assert!(
            aztec_lint_core::policy::is_supported_policy(lint.policy),
            "rule metadata policy '{}' is unsupported",
            lint.policy
        );
        assert_eq!(
            registration.rule.id(),
            lint.id,
            "rule implementation id '{}' does not match metadata id '{}'",
            registration.rule.id(),
            lint.id
        );
    }
}

fn validate_registry_integrity_with_catalog(registry: &[RuleRegistration]) {
    let mut seen_rule_ids = BTreeSet::<&'static str>::new();

    for registration in registry {
        let lint = registration.lint;
        seen_rule_ids.insert(lint.id);

        let canonical = find_lint(lint.id).unwrap_or_else(|| {
            panic!(
                "runtime rule '{}' is missing from canonical lint catalog",
                lint.id
            )
        });
        assert!(
            canonical.lifecycle.is_active(),
            "runtime rule '{}' maps to non-active canonical lint metadata",
            lint.id
        );
        assert_eq!(
            lint, canonical,
            "runtime rule '{}' metadata diverges from canonical lint metadata",
            lint.id
        );
    }

    for lint in all_lints().iter().filter(|lint| lint.lifecycle.is_active()) {
        assert!(
            seen_rule_ids.contains(lint.id),
            "canonical lint '{}' is missing from runtime rule registry",
            lint.id
        );
    }
}

fn is_pack_name_canonical(pack: &str) -> bool {
    !pack.is_empty()
        && pack
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
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
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use aztec_lint_core::config::RuleLevel;
    use aztec_lint_core::diagnostics::Confidence;
    use aztec_lint_core::lints::{LintCategory, LintDocs, LintLifecycleState, LintSpec, find_lint};
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;
    use crate::engine::registry::{RuleRegistration, full_registry};

    use super::{RuleEngine, validate_registry_integrity_with_catalog};

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

        let lint = find_lint("NOIR100").expect("NOIR100 should be in canonical catalog");
        let engine = RuleEngine::with_registry(vec![RuleRegistration {
            lint,
            rule: Box::new(TestRule),
        }]);

        let diagnostics = engine.run(
            &context,
            &BTreeMap::from([("NOIR100".to_string(), RuleLevel::Warn)]),
        );
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].suppressed);
        assert_eq!(
            diagnostics[0].suppression_reason.as_deref(),
            Some("allow(NOIR100)")
        );
    }

    struct StaticRule {
        id: &'static str,
    }

    impl Rule for StaticRule {
        fn id(&self) -> &'static str {
            self.id
        }

        fn run(
            &self,
            _ctx: &RuleContext<'_>,
            _out: &mut Vec<aztec_lint_core::diagnostics::Diagnostic>,
        ) {
        }
    }

    fn leak_test_lint(
        id: &'static str,
        pack: &'static str,
        policy: &'static str,
    ) -> &'static LintSpec {
        Box::leak(Box::new(LintSpec {
            id,
            pack,
            policy,
            category: LintCategory::Correctness,
            introduced_in: "0.1.0",
            default_level: RuleLevel::Warn,
            confidence: Confidence::Medium,
            lifecycle: LintLifecycleState::Active,
            docs: test_docs(),
        }))
    }

    fn test_registration(lint: &'static LintSpec, impl_id: &'static str) -> RuleRegistration {
        RuleRegistration {
            lint,
            rule: Box::new(StaticRule { id: impl_id }),
        }
    }

    #[test]
    fn engine_rejects_non_canonical_rule_id_metadata() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let lint = leak_test_lint("noir100", "noir_core", aztec_lint_core::policy::CORRECTNESS);
            RuleEngine::with_registry(vec![test_registration(lint, "noir100")]);
        }));
        assert!(result.is_err());
    }

    #[test]
    fn engine_rejects_unsupported_policy_metadata() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let lint = leak_test_lint("NOIR100", "noir_core", "non_deterministic");
            RuleEngine::with_registry(vec![test_registration(lint, "NOIR100")]);
        }));
        assert!(result.is_err());
    }

    #[test]
    fn engine_rejects_non_canonical_pack_metadata() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let lint = leak_test_lint("NOIR100", "NoirCore", aztec_lint_core::policy::CORRECTNESS);
            RuleEngine::with_registry(vec![test_registration(lint, "NOIR100")]);
        }));
        assert!(result.is_err());
    }

    #[test]
    fn engine_rejects_rule_and_metadata_id_mismatch() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let lint = leak_test_lint("NOIR100", "noir_core", aztec_lint_core::policy::CORRECTNESS);
            RuleEngine::with_registry(vec![test_registration(lint, "NOIR101")]);
        }));
        assert!(result.is_err());
    }

    #[test]
    fn engine_rejects_duplicate_rule_ids() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let lint_a =
                leak_test_lint("NOIR100", "noir_core", aztec_lint_core::policy::CORRECTNESS);
            let lint_b =
                leak_test_lint("NOIR100", "noir_core", aztec_lint_core::policy::CORRECTNESS);
            RuleEngine::with_registry(vec![
                test_registration(lint_a, "NOIR100"),
                test_registration(lint_b, "NOIR100"),
            ]);
        }));
        assert!(result.is_err());
    }

    #[test]
    fn full_registry_matches_canonical_lint_catalog() {
        validate_registry_integrity_with_catalog(&full_registry());
    }

    #[test]
    fn integrity_check_rejects_rule_missing_from_catalog() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let lint = leak_test_lint("NOIR999", "noir_core", aztec_lint_core::policy::CORRECTNESS);
            validate_registry_integrity_with_catalog(&[test_registration(lint, "NOIR999")]);
        }));
        assert!(result.is_err());
    }

    struct MutatingRule {
        id: &'static str,
    }

    impl Rule for MutatingRule {
        fn id(&self) -> &'static str {
            self.id
        }

        fn run(
            &self,
            ctx: &RuleContext<'_>,
            out: &mut Vec<aztec_lint_core::diagnostics::Diagnostic>,
        ) {
            // This intentionally attempts to mutate prior diagnostics. The engine
            // should isolate rule output so this has no effect outside this rule.
            out.clear();
            let file = &ctx.files()[0];
            out.push(ctx.diagnostic(
                self.id(),
                aztec_lint_core::policy::CORRECTNESS,
                "mutating rule diagnostic",
                file.span_for_range(0, 1),
            ));
        }
    }

    #[test]
    fn engine_isolates_rule_outputs_between_rules() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), "fn main() {}\n".to_string())],
        );

        let lint_noir100 = find_lint("NOIR100").expect("NOIR100 should be in canonical catalog");
        let lint_noir101 =
            leak_test_lint("NOIR101", "noir_core", aztec_lint_core::policy::CORRECTNESS);
        let registry = vec![
            RuleRegistration {
                lint: lint_noir100,
                rule: Box::new(TestRule),
            },
            RuleRegistration {
                lint: lint_noir101,
                rule: Box::new(MutatingRule { id: "NOIR101" }),
            },
        ];
        let engine = RuleEngine::with_registry(registry);

        let diagnostics = engine.run(
            &context,
            &BTreeMap::from([
                ("NOIR100".to_string(), RuleLevel::Warn),
                ("NOIR101".to_string(), RuleLevel::Warn),
            ]),
        );

        let mut ids = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_id.as_str())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        assert_eq!(ids, vec!["NOIR100", "NOIR101"]);
    }

    const fn test_docs() -> LintDocs {
        LintDocs {
            summary: "summary",
            what_it_does: "what it does",
            why_this_matters: "why this matters",
            known_limitations: "known limitations",
            how_to_fix: "how to fix",
            examples: &["example"],
            references: &["docs/reference.md"],
        }
    }
}
