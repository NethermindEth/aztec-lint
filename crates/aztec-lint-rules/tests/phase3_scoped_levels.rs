use std::collections::BTreeMap;

use aztec_lint_core::config::RuleLevel;
use aztec_lint_core::diagnostics::{Diagnostic, Severity};
use aztec_lint_core::lints::find_lint;
use aztec_lint_core::model::ProjectModel;
use aztec_lint_rules::engine::context::RuleContext;
use aztec_lint_rules::engine::registry::RuleRegistration;
use aztec_lint_rules::{Rule, RuleEngine};

struct MarkerRule;

impl Rule for MarkerRule {
    fn id(&self) -> &'static str {
        "NOIR100"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
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

fn run_marker_engine(source: &str, baseline: RuleLevel) -> Vec<Diagnostic> {
    let project = ProjectModel::default();
    let context = RuleContext::from_sources(
        &project,
        vec![("src/main.nr".to_string(), source.to_string())],
    );
    let lint = find_lint("NOIR100").expect("NOIR100 should exist in catalog");
    let engine = RuleEngine::with_registry(vec![RuleRegistration {
        lint,
        rule: Box::new(MarkerRule),
    }]);

    engine
        .run(
            &context,
            &BTreeMap::from([("NOIR100".to_string(), baseline)]),
        )
        .expect("engine run should succeed")
}

#[test]
fn scoped_level_precedence_and_suppression_visibility_are_preserved() {
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

    let diagnostics = run_marker_engine(source, RuleLevel::Warn);
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
fn scoped_non_allow_overrides_global_allow() {
    let source = r#"
#[deny(NOIR100)]
fn item_scope() {
    let item_value = 7;
}

fn baseline_scope() {
    let baseline_value = 9;
}
"#;

    let diagnostics = run_marker_engine(source, RuleLevel::Allow);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message, "item_value");
    assert!(!diagnostics[0].suppressed);
    assert_eq!(diagnostics[0].severity, Severity::Error);
}
