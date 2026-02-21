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
    let mut context = RuleContext::from_sources(
        &project,
        vec![("src/main.nr".to_string(), source.to_string())],
    );
    let sources = vec![SourceUnit::new("src/main.nr", source)];
    let model = build_aztec_model(&sources, &AztecConfig::default());
    context.set_aztec_model(model);

    let engine = RuleEngine::new();
    engine
        .run(
            &context,
            &BTreeMap::from([(rule_id.to_string(), RuleLevel::Deny)]),
        )
        .expect("engine run should succeed")
}

#[test]
fn aztec001_fixture_pair() {
    assert!(!run_rule("AZTEC001", &fixture_source("aztec001_positive.nr")).is_empty());
    assert!(run_rule("AZTEC001", &fixture_source("aztec001_negative.nr")).is_empty());
}

#[test]
fn aztec010_fixture_pair_and_cross_contract_guard() {
    assert!(!run_rule("AZTEC010", &fixture_source("aztec010_positive.nr")).is_empty());
    assert!(run_rule("AZTEC010", &fixture_source("aztec010_negative.nr")).is_empty());
    assert!(run_rule("AZTEC010", &fixture_source("aztec010_cross_contract.nr")).is_empty());
}

#[test]
fn aztec020_fixture_pair() {
    assert!(!run_rule("AZTEC020", &fixture_source("aztec020_positive.nr")).is_empty());
    assert!(run_rule("AZTEC020", &fixture_source("aztec020_negative.nr")).is_empty());
}

#[test]
fn aztec_suppression_short_and_scoped_forms() {
    let short = run_rule("AZTEC001", &fixture_source("aztec001_suppressed.nr"));
    assert_eq!(short.len(), 1);
    assert!(short[0].suppressed);
    assert_eq!(
        short[0].suppression_reason.as_deref(),
        Some("allow(AZTEC001)")
    );

    let scoped = run_rule("AZTEC020", &fixture_source("aztec020_suppressed.nr"));
    assert_eq!(scoped.len(), 1);
    assert!(scoped[0].suppressed);
    assert_eq!(
        scoped[0].suppression_reason.as_deref(),
        Some("allow(AZTEC020)")
    );
}
