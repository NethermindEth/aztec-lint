pub mod context;
pub mod query;
pub mod registry;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

use aztec_lint_core::config::RuleLevel;
use aztec_lint_core::diagnostics::{
    Diagnostic, DiagnosticViolation, Severity, sort_diagnostics, validate_diagnostics,
};
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleEngineError {
    InvalidDiagnostics {
        violations: Vec<DiagnosticViolation>,
    },
}

impl Display for RuleEngineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDiagnostics { violations } => {
                if let Some(first) = violations.first() {
                    write!(
                        f,
                        "diagnostic validation failed with {} violation(s); first violation: {}",
                        violations.len(),
                        first
                    )
                } else {
                    write!(f, "diagnostic validation failed with zero violations")
                }
            }
        }
    }
}

impl Error for RuleEngineError {}

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
    ) -> Result<Vec<Diagnostic>, RuleEngineError> {
        self.run(ctx, &settings.effective_levels)
    }

    pub fn run(
        &self,
        ctx: &RuleContext<'_>,
        effective_levels: &BTreeMap<String, RuleLevel>,
    ) -> Result<Vec<Diagnostic>, RuleEngineError> {
        let mut diagnostics = Vec::<Diagnostic>::new();

        for registration in &self.registry {
            let Some(level) = effective_levels.get(registration.lint.id).copied() else {
                continue;
            };
            if level == RuleLevel::Allow
                && !ctx.has_non_allow_scoped_directive(registration.lint.id)
            {
                continue;
            }

            // Run each rule against an isolated output buffer so a rule cannot
            // mutate diagnostics emitted by previously executed rules.
            let mut rule_diagnostics = Vec::<Diagnostic>::new();
            registration.rule.run(ctx, &mut rule_diagnostics);

            for diagnostic in &mut rule_diagnostics {
                diagnostic.rule_id = registration.lint.id.to_string();
                diagnostic.confidence = registration.lint.confidence;
                diagnostic.policy = registration.lint.policy.to_string();
            }

            let mut resolved_diagnostics = Vec::<Diagnostic>::new();
            for mut diagnostic in rule_diagnostics {
                let resolved_level =
                    ctx.resolve_rule_level(registration.lint.id, &diagnostic.primary_span, level);

                if resolved_level.level == RuleLevel::Allow {
                    if !resolved_level.from_scoped_directive {
                        continue;
                    }
                    let reason = ctx
                        .suppression_reason(registration.lint.id, &diagnostic.primary_span)
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("allow({})", registration.lint.id));
                    diagnostic.suppressed = true;
                    diagnostic.suppression_reason = Some(reason);
                    diagnostic.severity = Severity::Warning;
                } else {
                    diagnostic.severity = level_to_severity(resolved_level.level);
                }
                resolved_diagnostics.push(diagnostic);
            }

            diagnostics.extend(resolved_diagnostics);
        }

        sort_diagnostics(&mut diagnostics);

        let violations = validate_diagnostics(&diagnostics);
        if !violations.is_empty() {
            return Err(RuleEngineError::InvalidDiagnostics { violations });
        }

        Ok(diagnostics)
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
    use aztec_lint_core::diagnostics::{Confidence, Diagnostic, DiagnosticViolationKind, Severity};
    use aztec_lint_core::lints::{
        LintCategory, LintDocs, LintLifecycleState, LintMaturityTier, LintSpec, find_lint,
    };
    use aztec_lint_core::model::ProjectModel;
    use aztec_lint_core::model::Span;

    use crate::Rule;
    use crate::engine::context::RuleContext;
    use crate::engine::registry::{RuleRegistration, full_registry};

    use super::{RuleEngine, RuleEngineError, validate_registry_integrity_with_catalog};

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

    struct MarkerRule;

    impl Rule for MarkerRule {
        fn id(&self) -> &'static str {
            "NOIR100"
        }

        fn run(
            &self,
            ctx: &RuleContext<'_>,
            out: &mut Vec<aztec_lint_core::diagnostics::Diagnostic>,
        ) {
            let file = &ctx.files()[0];
            for marker in ["module_value", "item_value", "file_value", "baseline_value"] {
                let Some(offset) = file.text().find(marker) else {
                    continue;
                };
                out.push(ctx.diagnostic(
                    self.id(),
                    aztec_lint_core::policy::MAINTAINABILITY,
                    marker,
                    file.span_for_range(offset, offset + marker.len()),
                ));
            }
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

        let diagnostics = engine
            .run(
                &context,
                &BTreeMap::from([("NOIR100".to_string(), RuleLevel::Warn)]),
            )
            .expect("engine run should succeed");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].suppressed);
        assert_eq!(
            diagnostics[0].suppression_reason.as_deref(),
            Some("allow(NOIR100)")
        );
    }

    #[test]
    fn engine_applies_scoped_allow_warn_and_deny_levels() {
        let project = ProjectModel::default();
        let source = r#"
#[allow(NOIR100)]
use dep::foo;

#[warn(NOIR100)]
mod scoped {
    fn module_scope() {
        let module_value = 42;
    }

    #[deny(NOIR100)]
    fn item_scope() {
        let item_value = 7;
    }
}

fn file_scope() {
    let file_value = 3;
}
"#;
        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let lint = find_lint("NOIR100").expect("NOIR100 should be in canonical catalog");
        let engine = RuleEngine::with_registry(vec![RuleRegistration {
            lint,
            rule: Box::new(MarkerRule),
        }]);

        let diagnostics = engine
            .run(
                &context,
                &BTreeMap::from([("NOIR100".to_string(), RuleLevel::Warn)]),
            )
            .expect("engine run should succeed");
        assert_eq!(diagnostics.len(), 3);

        let module = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.message == "module_value")
            .expect("module diagnostic should exist");
        let item = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.message == "item_value")
            .expect("item diagnostic should exist");
        let file = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.message == "file_value")
            .expect("file diagnostic should exist");

        assert!(!module.suppressed);
        assert_eq!(module.severity, Severity::Warning);
        assert!(!item.suppressed);
        assert_eq!(item.severity, Severity::Error);
        assert!(file.suppressed);
        assert_eq!(file.suppression_reason.as_deref(), Some("allow(NOIR100)"));
    }

    #[test]
    fn engine_keeps_scoped_non_allow_diagnostics_when_global_is_allow() {
        let project = ProjectModel::default();
        let source = r#"
#[deny(NOIR100)]
fn item_scope() {
    let item_value = 7;
}

fn baseline_scope() {
    let baseline_value = 9;
}
"#;
        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let lint = find_lint("NOIR100").expect("NOIR100 should be in canonical catalog");
        let engine = RuleEngine::with_registry(vec![RuleRegistration {
            lint,
            rule: Box::new(MarkerRule),
        }]);

        let diagnostics = engine
            .run(
                &context,
                &BTreeMap::from([("NOIR100".to_string(), RuleLevel::Allow)]),
            )
            .expect("engine run should succeed");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "item_value");
        assert!(!diagnostics[0].suppressed);
        assert_eq!(diagnostics[0].severity, Severity::Error);
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
            maturity: LintMaturityTier::Stable,
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

        let diagnostics = engine
            .run(
                &context,
                &BTreeMap::from([
                    ("NOIR100".to_string(), RuleLevel::Warn),
                    ("NOIR101".to_string(), RuleLevel::Warn),
                ]),
            )
            .expect("engine run should succeed");

        let mut ids = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_id.as_str())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        assert_eq!(ids, vec!["NOIR100", "NOIR101"]);
    }

    struct InvalidDiagnosticRule;

    impl Rule for InvalidDiagnosticRule {
        fn id(&self) -> &'static str {
            "NOIR100"
        }

        fn run(
            &self,
            _ctx: &RuleContext<'_>,
            out: &mut Vec<aztec_lint_core::diagnostics::Diagnostic>,
        ) {
            out.push(Diagnostic {
                rule_id: String::new(),
                severity: Severity::Warning,
                confidence: Confidence::Low,
                policy: String::new(),
                message: String::new(),
                primary_span: Span::new("src/main.nr", 10, 2, 1, 1),
                secondary_spans: Vec::new(),
                suggestions: Vec::new(),
                notes: Vec::new(),
                helps: Vec::new(),
                structured_suggestions: Vec::new(),
                suggestion_groups: Vec::new(),
                fixes: Vec::new(),
                suppressed: true,
                suppression_reason: None,
            });
        }
    }

    #[test]
    fn engine_returns_validation_error_for_invalid_diagnostics() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), "fn main() {}\n".to_string())],
        );
        let lint = find_lint("NOIR100").expect("NOIR100 should be in canonical catalog");
        let engine = RuleEngine::with_registry(vec![RuleRegistration {
            lint,
            rule: Box::new(InvalidDiagnosticRule),
        }]);

        let error = engine
            .run(
                &context,
                &BTreeMap::from([("NOIR100".to_string(), RuleLevel::Warn)]),
            )
            .expect_err("invalid diagnostic should fail validation");

        let violations = match error {
            RuleEngineError::InvalidDiagnostics { violations } => violations,
        };
        assert!(
            violations
                .iter()
                .any(|violation| violation.kind == DiagnosticViolationKind::EmptyMessage)
        );
        assert!(violations.iter().any(|violation| {
            violation.kind == DiagnosticViolationKind::InvalidPrimarySpan { start: 10, end: 2 }
        }));
        assert!(violations.iter().any(|violation| {
            violation.kind == DiagnosticViolationKind::MissingSuppressionReason
        }));
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
