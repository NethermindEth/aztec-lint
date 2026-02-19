use std::fs;
use std::path::PathBuf;

use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::model::ProjectModel;
use aztec_lint_rules::Rule;
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
