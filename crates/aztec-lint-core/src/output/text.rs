use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostics::{
    Confidence, Diagnostic, Severity, StructuredMessage, StructuredSuggestion, SuggestionGroup,
    diagnostic_sort_key,
};
use crate::model::Span;
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
    let diagnostic = diagnostic
        .clone()
        .with_legacy_fields_from_suggestion_groups();
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
    let primary_line_text = source_line(
        source_root,
        &diagnostic.primary_span.file,
        source_cache,
        diagnostic.primary_span.line,
    );
    if let Some(line_text) = primary_line_text.clone() {
        let marker = match diagnostic.severity {
            Severity::Warning => {
                colors.warning(&marker_line(&line_text, diagnostic.primary_span.col))
            }
            Severity::Error => colors.error(&marker_line(&line_text, diagnostic.primary_span.col)),
        };
        let _ = writeln!(output, " {line_no:>gutter_width$} {accent_bar} {line_text}");
        let _ = writeln!(output, " {:>gutter_width$} {accent_bar} {}", "", marker);

        for suggestion in primary_span_suggestions(&diagnostic) {
            let marker = colors.help(&marker_line(&line_text, suggestion.span.col));
            let help_label = colors.help("help");
            let _ = writeln!(
                output,
                " {:>gutter_width$} {accent_bar} {} {help_label}: {}; replace with `{}`",
                "", marker, suggestion.message, suggestion.replacement
            );
        }
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

    for note in sorted_structured_messages(&diagnostic.notes) {
        render_structured_annotation(
            output,
            source_root,
            source_cache,
            note,
            AnnotationKind::Note,
            colors,
        );
    }

    for help in sorted_structured_messages(&diagnostic.helps) {
        render_structured_annotation(
            output,
            source_root,
            source_cache,
            help,
            AnnotationKind::Help,
            colors,
        );
    }

    let help_label = colors.help("help");
    let mut legacy_suggestions = diagnostic.suggestions.clone();
    legacy_suggestions.sort();
    for suggestion in legacy_suggestions {
        let _ = writeln!(output, "   = {help_label}: {suggestion}");
    }

    for group in sorted_suggestion_groups(&diagnostic) {
        let _ = writeln!(
            output,
            "   = {help_label}: suggestion group {} [{}; edits={}]: {}",
            group.id,
            group.applicability.as_str(),
            group.edits.len(),
            group.message
        );
        for edit in group.edits {
            let width = edit.span.end.saturating_sub(edit.span.start);
            let end_col = if width == 0 {
                edit.span.col
            } else {
                edit.span.col.saturating_add(width)
            };
            let _ = writeln!(
                output,
                "   = {help_label}:   edit {}:{}:{}..{} replace with `{}`",
                edit.span.file, edit.span.line, edit.span.col, end_col, edit.replacement
            );
        }
    }

    for suggestion in non_primary_span_suggestions(&diagnostic) {
        render_span_annotation(
            output,
            source_root,
            source_cache,
            &suggestion.span,
            format!(
                "{}; replace with `{}`",
                suggestion.message, suggestion.replacement
            ),
            AnnotationKind::Help,
            colors,
        );
    }

    if primary_line_text.is_none() {
        for suggestion in primary_span_suggestions(&diagnostic) {
            let _ = writeln!(
                output,
                "   = {help_label}: {}; replace with `{}`",
                suggestion.message, suggestion.replacement
            );
        }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AnnotationKind {
    Note,
    Help,
}

fn render_structured_annotation(
    output: &mut String,
    source_root: &Path,
    source_cache: &mut HashMap<String, Option<Vec<String>>>,
    message: StructuredMessage,
    kind: AnnotationKind,
    colors: Colorizer,
) {
    let label = match kind {
        AnnotationKind::Note => colors.note("note"),
        AnnotationKind::Help => colors.help("help"),
    };
    let _ = writeln!(output, "   = {label}: {}", message.message);
    if let Some(span) = message.span {
        render_span_annotation(
            output,
            source_root,
            source_cache,
            &span,
            message.message,
            kind,
            colors,
        );
    }
}

fn render_span_annotation(
    output: &mut String,
    source_root: &Path,
    source_cache: &mut HashMap<String, Option<Vec<String>>>,
    span: &Span,
    annotation: String,
    kind: AnnotationKind,
    colors: Colorizer,
) {
    let accent_bar = colors.accent("|");
    let accent_arrow = colors.accent("-->");
    let _ = writeln!(
        output,
        "  {} {}:{}:{}",
        accent_arrow, span.file, span.line, span.col
    );
    let _ = writeln!(output, "   {accent_bar}");
    let line_no = span.line.to_string();
    let gutter_width = line_no.len();
    let label = match kind {
        AnnotationKind::Note => colors.note("note"),
        AnnotationKind::Help => colors.help("help"),
    };
    if let Some(line_text) = source_line(source_root, &span.file, source_cache, span.line) {
        let marker = marker_line(&line_text, span.col);
        let marker = match kind {
            AnnotationKind::Note => colors.note(&marker),
            AnnotationKind::Help => colors.help(&marker),
        };
        let _ = writeln!(output, " {line_no:>gutter_width$} {accent_bar} {line_text}");
        let _ = writeln!(
            output,
            " {:>gutter_width$} {accent_bar} {} {label}: {annotation}",
            "", marker
        );
    } else {
        let marker = match kind {
            AnnotationKind::Note => colors.note("^"),
            AnnotationKind::Help => colors.help("^"),
        };
        let _ = writeln!(
            output,
            " {line_no:>gutter_width$} {accent_bar} <source unavailable>"
        );
        let _ = writeln!(
            output,
            " {:>gutter_width$} {accent_bar} {} {label}: {annotation}",
            "", marker
        );
    }
    let _ = writeln!(output, "   {accent_bar}");
}

fn sorted_structured_messages(messages: &[StructuredMessage]) -> Vec<StructuredMessage> {
    let mut items = messages.to_vec();
    items.sort_by_key(|item| {
        if let Some(span) = &item.span {
            (
                0u8,
                span.file.clone(),
                span.line,
                span.col,
                span.start,
                span.end,
                item.message.clone(),
            )
        } else {
            (
                1u8,
                String::new(),
                0u32,
                0u32,
                0u32,
                0u32,
                item.message.clone(),
            )
        }
    });
    items
}

fn sorted_structured_suggestions(diagnostic: &Diagnostic) -> Vec<StructuredSuggestion> {
    let mut items = diagnostic.structured_suggestions.clone();
    items.sort_by_key(|suggestion| {
        (
            suggestion.span.file.clone(),
            suggestion.span.line,
            suggestion.span.col,
            suggestion.span.start,
            suggestion.span.end,
            suggestion.message.clone(),
            suggestion.replacement.clone(),
            suggestion.applicability.as_str().to_string(),
        )
    });
    items
}

fn primary_span_suggestions(diagnostic: &Diagnostic) -> Vec<StructuredSuggestion> {
    sorted_structured_suggestions(diagnostic)
        .into_iter()
        .filter(|suggestion| suggestion.span == diagnostic.primary_span)
        .collect()
}

fn non_primary_span_suggestions(diagnostic: &Diagnostic) -> Vec<StructuredSuggestion> {
    sorted_structured_suggestions(diagnostic)
        .into_iter()
        .filter(|suggestion| suggestion.span != diagnostic.primary_span)
        .collect()
}

fn sorted_suggestion_groups(diagnostic: &Diagnostic) -> Vec<SuggestionGroup> {
    let mut groups = diagnostic.suggestion_groups.clone();
    for group in &mut groups {
        group.edits.sort_by_key(|edit| {
            (
                edit.span.file.clone(),
                edit.span.line,
                edit.span.col,
                edit.span.start,
                edit.span.end,
                edit.replacement.clone(),
            )
        });
    }
    groups.sort_by_key(|group| {
        (
            group.id.clone(),
            suggestion_group_edits_sort_key(group),
            group.message.clone(),
            group.applicability.as_str().to_string(),
            group.provenance.clone().unwrap_or_default(),
        )
    });
    groups
}

fn suggestion_group_edits_sort_key(
    group: &SuggestionGroup,
) -> Vec<(String, u32, u32, u32, u32, String)> {
    let mut edits = group
        .edits
        .iter()
        .map(|edit| {
            (
                edit.span.file.clone(),
                edit.span.line,
                edit.span.col,
                edit.span.start,
                edit.span.end,
                edit.replacement.clone(),
            )
        })
        .collect::<Vec<_>>();
    edits.sort();
    edits
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
    use crate::diagnostics::{
        Applicability, Confidence, Diagnostic, Severity, StructuredMessage, StructuredSuggestion,
        SuggestionGroup, TextEdit,
    };
    use crate::model::Span;

    fn strip_ansi(input: &str) -> String {
        let bytes = input.as_bytes();
        let mut output = String::with_capacity(input.len());
        let mut cursor = 0usize;

        while cursor < bytes.len() {
            if bytes[cursor] == 0x1b {
                cursor += 1;
                if cursor < bytes.len() && bytes[cursor] == b'[' {
                    cursor += 1;
                    while cursor < bytes.len() {
                        let b = bytes[cursor];
                        cursor += 1;
                        if b.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                continue;
            }

            output.push(bytes[cursor] as char);
            cursor += 1;
        }

        output
    }

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
            suggestion_groups: Vec::new(),
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

        let output = strip_ansi(&render_check_report(report));
        assert!(output.contains("warning[NOIR100]: magic number `7` should be named"));
        assert!(output.contains("  --> src/main.nr:1:17"));
        assert!(output.contains("1 | fn main() { let x = 7; }"));
        assert!(output.contains("|                 ^"));
    }

    #[test]
    fn check_text_output_renders_structured_notes_helps_and_suggestions() {
        let temp = tempdir().expect("temp dir should be created");
        let root = temp.path();
        fs::create_dir_all(root.join("src")).expect("source directory should be created");
        fs::write(
            root.join("src/main.nr"),
            "fn main() {\n    let x = 7;\n    let y = x;\n}\n",
        )
        .expect("source file should be written");

        let mut issue = diagnostic(
            "src/main.nr",
            2,
            13,
            "NOIR100",
            "magic number `7` should be named",
        );
        issue.notes = vec![
            StructuredMessage {
                message: "z note".to_string(),
                span: Some(Span::new("src/main.nr", 28, 29, 3, 9)),
            },
            StructuredMessage {
                message: "a note".to_string(),
                span: Some(Span::new("src/main.nr", 16, 17, 2, 5)),
            },
            StructuredMessage {
                message: "plain note".to_string(),
                span: None,
            },
        ];
        issue.helps = vec![
            StructuredMessage {
                message: "plain help".to_string(),
                span: None,
            },
            StructuredMessage {
                message: "span help".to_string(),
                span: Some(Span::new("src/main.nr", 16, 17, 2, 5)),
            },
        ];
        issue.suggestions = vec!["z legacy help".to_string(), "a legacy help".to_string()];
        issue.structured_suggestions = vec![
            StructuredSuggestion {
                message: "replace assignment".to_string(),
                span: Span::new("src/main.nr", 28, 33, 3, 9),
                replacement: "let y = NAMED_CONST;".to_string(),
                applicability: Applicability::MaybeIncorrect,
            },
            StructuredSuggestion {
                message: "introduce named constant".to_string(),
                span: Span::new("src/main.nr", 20, 21, 2, 13),
                replacement: "NAMED_CONST".to_string(),
                applicability: Applicability::MachineApplicable,
            },
        ];

        let report = CheckTextReport {
            path: root,
            source_root: root,
            show_run_header: false,
            profile: "default",
            changed_only: false,
            active_rules: 1,
            diagnostics: &[&issue],
        };

        let output = strip_ansi(&render_check_report(report));
        assert!(output.contains("= note: plain note"));
        assert!(output.contains("= help: plain help"));
        assert!(output.contains("help: introduce named constant; replace with `NAMED_CONST`"));
        assert!(output.contains("help: replace assignment; replace with `let y = NAMED_CONST;`"));
        assert!(output.contains("  --> src/main.nr:3:9"));

        let a_note_index = output.find("a note").expect("a note must exist");
        let z_note_index = output.find("z note").expect("z note must exist");
        assert!(a_note_index < z_note_index);

        let a_legacy_index = output
            .find("= help: a legacy help")
            .expect("a legacy help must exist");
        let z_legacy_index = output
            .find("= help: z legacy help")
            .expect("z legacy help must exist");
        assert!(a_legacy_index < z_legacy_index);
    }

    #[test]
    fn check_text_output_renders_grouped_suggestions_via_legacy_compatibility() {
        let temp = tempdir().expect("temp dir should be created");
        let root = temp.path();
        fs::create_dir_all(root.join("src")).expect("source directory should be created");
        fs::write(root.join("src/main.nr"), "fn main() { let x = 7; }\n")
            .expect("source file should be written");

        let mut issue = diagnostic("src/main.nr", 1, 17, "NOIR100", "message");
        issue.suggestion_groups = vec![SuggestionGroup {
            id: "sg0001".to_string(),
            message: "replace literal".to_string(),
            applicability: Applicability::MachineApplicable,
            edits: vec![TextEdit {
                span: Span::new("src/main.nr", 20, 21, 1, 21),
                replacement: "NAMED_CONST".to_string(),
            }],
            provenance: None,
        }];

        let report = CheckTextReport {
            path: root,
            source_root: root,
            show_run_header: false,
            profile: "default",
            changed_only: false,
            active_rules: 1,
            diagnostics: &[&issue],
        };

        let output = strip_ansi(&render_check_report(report));
        assert!(output.contains("help: replace literal; replace with `NAMED_CONST`"));
        assert!(output.contains(
            "help: suggestion group sg0001 [machine-applicable; edits=1]: replace literal"
        ));
    }

    #[test]
    fn check_text_output_sorts_grouped_suggestions_using_edit_spans() {
        let temp = tempdir().expect("temp dir should be created");
        let root = temp.path();
        fs::create_dir_all(root.join("src")).expect("source directory should be created");
        fs::write(root.join("src/main.nr"), "fn main() { let x = 7; }\n")
            .expect("source file should be written");

        let mut issue = diagnostic("src/main.nr", 1, 17, "NOIR100", "message");
        issue.suggestion_groups = vec![
            SuggestionGroup {
                id: "sg0001".to_string(),
                message: "replace literal".to_string(),
                applicability: Applicability::MachineApplicable,
                edits: vec![TextEdit {
                    span: Span::new("src/main.nr", 20, 21, 1, 21),
                    replacement: "B".to_string(),
                }],
                provenance: None,
            },
            SuggestionGroup {
                id: "sg0001".to_string(),
                message: "replace literal".to_string(),
                applicability: Applicability::MachineApplicable,
                edits: vec![TextEdit {
                    span: Span::new("src/main.nr", 10, 11, 1, 11),
                    replacement: "A".to_string(),
                }],
                provenance: None,
            },
        ];

        let report = CheckTextReport {
            path: root,
            source_root: root,
            show_run_header: false,
            profile: "default",
            changed_only: false,
            active_rules: 1,
            diagnostics: &[&issue],
        };

        let output = strip_ansi(&render_check_report(report));
        let a_idx = output
            .find("edit src/main.nr:1:11..12 replace with `A`")
            .expect("A edit should be rendered");
        let b_idx = output
            .find("edit src/main.nr:1:21..22 replace with `B`")
            .expect("B edit should be rendered");
        assert!(a_idx < b_idx);
    }
}
