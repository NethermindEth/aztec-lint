use std::fmt::Write;
use std::path::Path;

use crate::diagnostics::{Confidence, Diagnostic, Severity, diagnostic_sort_key};

pub struct CheckTextReport<'a> {
    pub path: &'a Path,
    pub profile: &'a str,
    pub changed_only: bool,
    pub active_rules: usize,
    pub diagnostics: &'a [&'a Diagnostic],
}

pub fn render_check_report(report: CheckTextReport<'_>) -> String {
    let mut output = String::new();
    let mut diagnostics = report.diagnostics.to_vec();
    diagnostics.sort_by_key(|diagnostic| diagnostic_sort_key(diagnostic));

    let _ = writeln!(
        output,
        "checked={} profile={} changed_only={} active_rules={}",
        report.path.display(),
        report.profile,
        report.changed_only,
        report.active_rules
    );

    if diagnostics.is_empty() {
        let _ = writeln!(output, "No diagnostics.");
        return output;
    }

    for diagnostic in &diagnostics {
        let _ = writeln!(
            output,
            "{}:{}:{}: {}[{}] {} (confidence={}, policy={})",
            diagnostic.primary_span.file,
            diagnostic.primary_span.line,
            diagnostic.primary_span.col,
            severity_label(diagnostic.severity),
            diagnostic.rule_id,
            diagnostic.message,
            confidence_label(diagnostic.confidence),
            diagnostic.policy
        );
    }

    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count();
    let warnings = diagnostics.len().saturating_sub(errors);
    let _ = writeln!(
        output,
        "diagnostics={} errors={} warnings={warnings}",
        diagnostics.len(),
        errors
    );
    output
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

fn confidence_label(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::Low => "low",
        Confidence::Medium => "medium",
        Confidence::High => "high",
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{CheckTextReport, render_check_report};
    use crate::diagnostics::{Confidence, Diagnostic, Severity};
    use crate::model::Span;

    fn diagnostic(file: &str, line: u32, col: u32, rule_id: &str, message: &str) -> Diagnostic {
        Diagnostic {
            rule_id: rule_id.to_string(),
            severity: Severity::Warning,
            confidence: Confidence::Medium,
            policy: "privacy".to_string(),
            message: message.to_string(),
            primary_span: Span::new(file, 1, 2, line, col),
            secondary_spans: Vec::new(),
            suggestions: Vec::new(),
            fixes: Vec::new(),
            suppressed: false,
            suppression_reason: None,
        }
    }

    #[test]
    fn check_text_output_is_stably_sorted() {
        let second = diagnostic("src/main.nr", 3, 1, "AZTEC020", "second message");
        let first = diagnostic("src/main.nr", 1, 1, "AZTEC001", "first message");
        let report = CheckTextReport {
            path: Path::new("."),
            profile: "default",
            changed_only: false,
            active_rules: 2,
            diagnostics: &[&second, &first],
        };

        let output = render_check_report(report);
        let first_index = output
            .find("AZTEC001")
            .expect("first rule should exist in output");
        let second_index = output
            .find("AZTEC020")
            .expect("second rule should exist in output");
        assert!(first_index < second_index);
    }
}
