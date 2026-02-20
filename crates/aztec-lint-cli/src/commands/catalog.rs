use aztec_lint_core::diagnostics::Confidence;
use aztec_lint_core::lints::{LintSpec, all_lints, find_lint};

pub type RuleDoc = LintSpec;

pub fn all_rules() -> &'static [RuleDoc] {
    all_lints()
}

pub fn find_rule(rule_id: &str) -> Option<&'static RuleDoc> {
    find_lint(rule_id)
}

pub fn confidence_label(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::Low => "low",
        Confidence::Medium => "medium",
        Confidence::High => "high",
    }
}
