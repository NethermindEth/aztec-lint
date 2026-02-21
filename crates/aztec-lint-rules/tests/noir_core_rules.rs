use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use aztec_lint_core::config::RuleLevel;
use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::fix::{FixApplicationMode, SkippedFixReason, apply_fixes};
use aztec_lint_core::model::ProjectModel;
use aztec_lint_core::output::text::{CheckTextReport, render_check_report};
use aztec_lint_rules::Rule;
use aztec_lint_rules::RuleEngine;
use aztec_lint_rules::engine::context::RuleContext;
use aztec_lint_rules::noir_core::noir001_unused::Noir001UnusedRule;
use aztec_lint_rules::noir_core::noir002_shadowing::Noir002ShadowingRule;
use aztec_lint_rules::noir_core::noir010_bool_not_asserted::Noir010BoolNotAssertedRule;
use aztec_lint_rules::noir_core::noir020_bounds::Noir020BoundsRule;
use aztec_lint_rules::noir_core::noir030_unconstrained_influence::Noir030UnconstrainedInfluenceRule;
use aztec_lint_rules::noir_core::noir100_magic_numbers::Noir100MagicNumbersRule;
use aztec_lint_rules::noir_core::noir110_complexity::Noir110ComplexityRule;
use aztec_lint_rules::noir_core::noir120_nesting::Noir120NestingRule;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/noir_core/rule_cases")
        .join(name)
}

fn fixture_source(name: &str) -> String {
    fs::read_to_string(fixture_path(name)).expect("fixture source must load")
}

fn run_rule(rule: &dyn Rule, source: &str) -> Vec<Diagnostic> {
    let project = ProjectModel::default();
    let context = RuleContext::from_sources(
        &project,
        vec![("src/main.nr".to_string(), source.to_string())],
    );
    let mut diagnostics = Vec::new();
    rule.run(&context, &mut diagnostics);
    diagnostics
}

fn run_rule_with_engine(rule_id: &str, source: &str) -> Vec<Diagnostic> {
    let project = ProjectModel::default();
    let context = RuleContext::from_sources(
        &project,
        vec![("src/main.nr".to_string(), source.to_string())],
    );
    let engine = RuleEngine::new();
    engine
        .run(
            &context,
            &BTreeMap::from([(rule_id.to_string(), RuleLevel::Deny)]),
        )
        .expect("engine run should succeed")
}

#[test]
fn noir001_fixture_pair() {
    let rule = Noir001UnusedRule;
    assert!(!run_rule(&rule, &fixture_source("noir001_positive.nr")).is_empty());
    assert!(run_rule(&rule, &fixture_source("noir001_negative.nr")).is_empty());
}

#[test]
fn noir002_fixture_pair() {
    let rule = Noir002ShadowingRule;
    assert!(!run_rule(&rule, &fixture_source("noir002_positive.nr")).is_empty());
    assert!(run_rule(&rule, &fixture_source("noir002_negative.nr")).is_empty());
}

#[test]
fn noir010_fixture_pair() {
    let rule = Noir010BoolNotAssertedRule;
    assert!(!run_rule(&rule, &fixture_source("noir010_positive.nr")).is_empty());
    assert!(run_rule(&rule, &fixture_source("noir010_negative.nr")).is_empty());
}

#[test]
fn noir020_fixture_pair() {
    let rule = Noir020BoundsRule;
    assert!(!run_rule(&rule, &fixture_source("noir020_positive.nr")).is_empty());
    assert!(run_rule(&rule, &fixture_source("noir020_negative.nr")).is_empty());
}

#[test]
fn noir030_fixture_pair() {
    let rule = Noir030UnconstrainedInfluenceRule;
    assert!(!run_rule(&rule, &fixture_source("noir030_positive.nr")).is_empty());
    assert!(run_rule(&rule, &fixture_source("noir030_negative.nr")).is_empty());
}

#[test]
fn noir100_fixture_pair() {
    let rule = Noir100MagicNumbersRule;
    assert!(!run_rule(&rule, &fixture_source("noir100_positive.nr")).is_empty());
    assert!(run_rule(&rule, &fixture_source("noir100_negative.nr")).is_empty());
}

#[test]
fn noir110_fixture_pair() {
    let rule = Noir110ComplexityRule;
    assert!(!run_rule(&rule, &fixture_source("noir110_positive.nr")).is_empty());
    assert!(run_rule(&rule, &fixture_source("noir110_negative.nr")).is_empty());
}

#[test]
fn noir120_fixture_pair() {
    let rule = Noir120NestingRule;
    assert!(!run_rule(&rule, &fixture_source("noir120_positive.nr")).is_empty());
    assert!(run_rule(&rule, &fixture_source("noir120_negative.nr")).is_empty());
}

#[test]
fn noir001_alias_import_edge_case_is_covered() {
    let rule = Noir001UnusedRule;
    assert!(run_rule(&rule, &fixture_source("noir001_alias_import_negative.nr")).is_empty());
}

#[test]
fn noir002_nested_scope_edge_case_is_covered() {
    let rule = Noir002ShadowingRule;
    assert!(!run_rule(&rule, &fixture_source("noir002_nested_scope_positive.nr")).is_empty());
}

#[test]
fn noir020_range_guard_edge_cases_are_covered() {
    let rule = Noir020BoundsRule;
    assert!(
        run_rule(
            &rule,
            &fixture_source("noir020_guard_after_access_negative.nr")
        )
        .is_empty()
    );
    assert!(run_rule(&rule, &fixture_source("noir020_branch_guard_negative.nr")).is_empty());
}

#[test]
fn noir_core_phase2_rules_support_suppression() {
    let cases = [
        ("NOIR001", "noir001_suppressed.nr"),
        ("NOIR002", "noir002_suppressed.nr"),
        ("NOIR010", "noir010_suppressed.nr"),
        ("NOIR020", "noir020_suppressed.nr"),
        ("NOIR030", "noir030_suppressed.nr"),
        ("NOIR100", "noir100_suppressed.nr"),
        ("NOIR110", "noir110_suppressed.nr"),
        ("NOIR120", "noir120_suppressed.nr"),
    ];

    for (rule_id, fixture) in cases {
        let diagnostics = run_rule_with_engine(rule_id, &fixture_source(fixture));
        let expected_reason = format!("allow({rule_id})");
        assert!(
            !diagnostics.is_empty(),
            "expected diagnostics for suppressed fixture {rule_id}"
        );
        assert!(
            diagnostics.iter().all(|diagnostic| diagnostic.suppressed),
            "expected all diagnostics to be suppressed for {rule_id}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.suppression_reason.as_deref()
                    == Some(expected_reason.as_str())),
            "expected suppression reason to match for {rule_id}"
        );
    }
}

#[test]
fn noir100_maybe_incorrect_suggestions_are_rendered_but_not_auto_applied() {
    let rule = Noir100MagicNumbersRule;
    let source = "fn main() { let fee = 42; }\n";
    let diagnostics = run_rule(&rule, source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].structured_suggestions.len(), 1);
    assert_eq!(
        diagnostics[0].structured_suggestions[0].applicability,
        aztec_lint_core::diagnostics::Applicability::MaybeIncorrect
    );

    let temp_root = temp_test_root("noir100_text_and_fix");
    let source_path = temp_root.join("src/main.nr");
    fs::create_dir_all(source_path.parent().expect("source parent should exist"))
        .expect("source directory should exist");
    fs::write(&source_path, source).expect("source file should be written");

    let rendered = render_check_report(CheckTextReport {
        path: temp_root.as_path(),
        source_root: temp_root.as_path(),
        show_run_header: false,
        profile: "default",
        changed_only: false,
        active_rules: 1,
        diagnostics: &[&diagnostics[0]],
    });
    assert!(
        rendered.contains("replace with `NAMED_CONSTANT`"),
        "text output should include structured suggestion: {rendered}"
    );

    let report = apply_fixes(temp_root.as_path(), &diagnostics, FixApplicationMode::Apply)
        .expect("fix application should succeed");
    assert_eq!(report.total_candidates, 1);
    assert!(report.selected.is_empty());
    assert_eq!(report.skipped.len(), 1);
    assert_eq!(report.skipped[0].reason, SkippedFixReason::UnsafeFix);
    assert_eq!(
        fs::read_to_string(&source_path).expect("source should remain readable"),
        source
    );

    let _ = fs::remove_dir_all(temp_root);
}

fn temp_test_root(prefix: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("aztec_lint_{prefix}_{timestamp}"));
    fs::create_dir_all(&path).expect("temp root should be created");
    path
}
