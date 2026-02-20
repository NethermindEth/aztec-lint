use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Confidence, Diagnostic, Severity, diagnostic_sort_key};
use crate::output::ansi::{Colorizer, Stream};

pub struct CheckTextReport<'a> {
    pub path: &'a Path,
    pub source_root: &'a Path,
    pub show_run_header: bool,
    pub profile: &'a str,
    pub changed_only: bool,
    pub active_rules: usize,
    pub diagnostics: &'a [&'a Diagnostic],
}

pub fn render_check_report(report: CheckTextReport<'_>) -> String {
    let mut output = String::new();
    let mut diagnostics = report.diagnostics.to_vec();
    let mut source_cache = HashMap::<String, Option<Vec<String>>>::new();
    let colors = Colorizer::for_stream(Stream::Stdout);
    diagnostics.sort_by_key(|diagnostic| diagnostic_sort_key(diagnostic));

    if report.show_run_header {
        let _ = writeln!(
            output,
            "checked={} profile={} changed_only={} active_rules={}",
            report.path.display(),
            report.profile,
            report.changed_only,
            report.active_rules
        );
    }

    if diagnostics.is_empty() {
        let _ = writeln!(output, "No diagnostics.");
        return output;
    }

    for diagnostic in &diagnostics {
        render_diagnostic(
            &mut output,
            report.source_root,
            diagnostic,
            &mut source_cache,
            colors,
        );
        let _ = writeln!(output);
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

fn render_diagnostic(
    output: &mut String,
    source_root: &Path,
    diagnostic: &Diagnostic,
    source_cache: &mut HashMap<String, Option<Vec<String>>>,
    colors: Colorizer,
) {
    let severity = severity_label(diagnostic.severity);
    let severity = match diagnostic.severity {
        Severity::Warning => colors.warning(severity),
        Severity::Error => colors.error(severity),
    };
    let accent_bar = colors.accent("|");
    let accent_arrow = colors.accent("-->");

    let _ = writeln!(
        output,
        "{}[{}]: {}",
        severity, diagnostic.rule_id, diagnostic.message
    );
    let _ = writeln!(
        output,
        "  {} {}:{}:{}",
        accent_arrow,
        diagnostic.primary_span.file,
        diagnostic.primary_span.line,
        diagnostic.primary_span.col
    );
    let _ = writeln!(output, "   {accent_bar}");

    let line_no = diagnostic.primary_span.line.to_string();
    let gutter_width = line_no.len();
    if let Some(line_text) = source_line(
        source_root,
        &diagnostic.primary_span.file,
        source_cache,
        diagnostic.primary_span.line,
    ) {
        let marker = match diagnostic.severity {
            Severity::Warning => {
                colors.warning(&marker_line(&line_text, diagnostic.primary_span.col))
            }
            Severity::Error => colors.error(&marker_line(&line_text, diagnostic.primary_span.col)),
        };
        let _ = writeln!(output, " {line_no:>gutter_width$} {accent_bar} {line_text}");
        let _ = writeln!(output, " {:>gutter_width$} {accent_bar} {}", "", marker);
    } else {
        let marker = match diagnostic.severity {
            Severity::Warning => colors.warning("^"),
            Severity::Error => colors.error("^"),
        };
        let _ = writeln!(
            output,
            " {line_no:>gutter_width$} {accent_bar} <source unavailable>"
        );
        let _ = writeln!(output, " {:>gutter_width$} {accent_bar} {marker}", "");
    }
    let _ = writeln!(output, "   {accent_bar}");
    let note_label = colors.note("note");
    let _ = writeln!(
        output,
        "   = {note_label}: confidence={}, policy={}",
        confidence_label(diagnostic.confidence),
        diagnostic.policy
    );

    if diagnostic.suppressed {
        let reason = diagnostic
            .suppression_reason
            .as_deref()
            .unwrap_or("suppressed");
        let _ = writeln!(output, "   = {note_label}: [suppressed: {reason}]");
    }

    let help_label = colors.help("help");
    for suggestion in &diagnostic.suggestions {
        let _ = writeln!(output, "   = {help_label}: {suggestion}");
    }
}

fn source_line(
    source_root: &Path,
    file: &str,
    source_cache: &mut HashMap<String, Option<Vec<String>>>,
    line_number: u32,
) -> Option<String> {
    let lines = source_cache.entry(file.to_string()).or_insert_with(|| {
        let path = source_path(source_root, file);
        let contents = fs::read_to_string(path).ok()?;
        Some(contents.lines().map(str::to_string).collect::<Vec<_>>())
    });

    let line_index = usize::try_from(line_number.saturating_sub(1)).ok()?;
    lines.as_ref()?.get(line_index).cloned()
}

fn source_path(source_root: &Path, file: &str) -> PathBuf {
    let path = Path::new(file);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        source_root.join(path)
    }
}

fn marker_line(line_text: &str, col: u32) -> String {
    let line_width = line_text.chars().count();
    let col = usize::try_from(col.saturating_sub(1)).unwrap_or(0);
    let padding = " ".repeat(col.min(line_width));
    format!("{padding}^")
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
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

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
            notes: Vec::new(),
            helps: Vec::new(),
            structured_suggestions: Vec::new(),
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
            source_root: Path::new("."),
            show_run_header: true,
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

    #[test]
    fn check_text_output_includes_clippy_style_snippet() {
        let temp = tempdir().expect("temp dir should be created");
        let root = temp.path();
        fs::create_dir_all(root.join("src")).expect("source directory should be created");
        fs::write(root.join("src/main.nr"), "fn main() { let x = 7; }\n")
            .expect("source file should be written");

        let issue = diagnostic(
            "src/main.nr",
            1,
            17,
            "NOIR100",
            "magic number `7` should be named",
        );
        let report = CheckTextReport {
            path: root,
            source_root: root,
            show_run_header: true,
            profile: "default",
            changed_only: false,
            active_rules: 1,
            diagnostics: &[&issue],
        };

        let output = render_check_report(report);
        assert!(output.contains("warning[NOIR100]: magic number `7` should be named"));
        assert!(output.contains("  --> src/main.nr:1:17"));
        assert!(output.contains("1 | fn main() { let x = 7; }"));
        assert!(output.contains("|                 ^"));
    }
}
