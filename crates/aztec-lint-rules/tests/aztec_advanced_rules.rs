use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use aztec_lint_aztec::{SourceUnit, build_aztec_model};
use aztec_lint_core::config::{AztecConfig, RuleLevel};
use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::model::ProjectModel;
use aztec_lint_rules::RuleEngine;
use aztec_lint_rules::engine::context::RuleContext;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/aztec/rule_cases")
        .join(name)
}

fn fixture_source(name: &str) -> String {
    fs::read_to_string(fixture_path(name)).expect("fixture source must load")
}

fn run_rule(rule_id: &str, source: &str) -> Vec<Diagnostic> {
    let project = ProjectModel::default();
    let config = AztecConfig::default();
    let mut context = RuleContext::from_sources(
        &project,
        vec![("src/main.nr".to_string(), source.to_string())],
    );
    context.set_aztec_config(config.clone());
    let sources = vec![SourceUnit::new("src/main.nr", source)];
    let model = build_aztec_model(&sources, &config);
    context.set_aztec_model(model);

    let engine = RuleEngine::new();
    engine
        .run(
            &context,
            &BTreeMap::from([(rule_id.to_string(), RuleLevel::Deny)]),
        )
        .expect("engine run should succeed")
}

fn assert_single_suppressed_with_reason(diagnostics: &[Diagnostic], expected_rule: &str) {
    let expected_reason = format!("allow({expected_rule})");
    assert_eq!(
        diagnostics.len(),
        1,
        "expected exactly one diagnostic for {expected_rule}"
    );
    assert!(
        diagnostics[0].suppressed,
        "expected diagnostic to be suppressed for {expected_rule}"
    );
    assert_eq!(
        diagnostics[0].suppression_reason.as_deref(),
        Some(expected_reason.as_str()),
        "expected canonical suppression reason for {expected_rule}"
    );
}

#[test]
fn aztec002_fixture_pair() {
    assert!(!run_rule("AZTEC002", &fixture_source("aztec002_positive.nr")).is_empty());
    assert!(run_rule("AZTEC002", &fixture_source("aztec002_negative.nr")).is_empty());

    assert!(
        !run_rule(
            "AZTEC002",
            &fixture_source("aztec002_effect_coupling_positive.nr")
        )
        .is_empty()
    );

    let suppressed = run_rule("AZTEC002", &fixture_source("aztec002_suppressed.nr"));
    assert_single_suppressed_with_reason(&suppressed, "AZTEC002");
}

#[test]
fn aztec003_fixture_pair_and_suppression() {
    assert!(!run_rule("AZTEC003", &fixture_source("aztec003_positive.nr")).is_empty());
    assert!(run_rule("AZTEC003", &fixture_source("aztec003_negative.nr")).is_empty());

    let suppressed = run_rule("AZTEC003", &fixture_source("aztec003_suppressed.nr"));
    assert_single_suppressed_with_reason(&suppressed, "AZTEC003");
}

#[test]
fn aztec021_fixture_pair_and_scoped_suppression() {
    let positive = run_rule("AZTEC021", &fixture_source("aztec021_positive.nr"));
    assert!(!positive.is_empty());
    assert!(
        positive
            .iter()
            .all(|diagnostic| !diagnostic.suggestion_groups.is_empty())
    );
    assert!(positive.iter().all(|diagnostic| {
        diagnostic.suggestion_groups.iter().all(|suggestion| {
            suggestion.applicability == aztec_lint_core::diagnostics::Applicability::MaybeIncorrect
        })
    }));
    assert!(run_rule("AZTEC021", &fixture_source("aztec021_negative.nr")).is_empty());
    assert!(
        !run_rule(
            "AZTEC021",
            &fixture_source("aztec021_guard_after_hash_positive.nr")
        )
        .is_empty()
    );

    let suppressed = run_rule("AZTEC021", &fixture_source("aztec021_suppressed.nr"));
    assert_single_suppressed_with_reason(&suppressed, "AZTEC021");
}

#[test]
fn aztec022_fixture_pair() {
    assert!(!run_rule("AZTEC022", &fixture_source("aztec022_positive.nr")).is_empty());
    assert!(run_rule("AZTEC022", &fixture_source("aztec022_negative.nr")).is_empty());

    let suppressed = run_rule("AZTEC022", &fixture_source("aztec022_suppressed.nr"));
    assert_single_suppressed_with_reason(&suppressed, "AZTEC022");
}
