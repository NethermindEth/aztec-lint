use std::fs;
use std::path::Path;

use aztec_lint_core::diagnostics::{
    Applicability, Confidence, Diagnostic, DiagnosticViolationKind, Severity, SuggestionGroup,
    TextEdit, validate_diagnostics,
};
use aztec_lint_core::fix::{FixApplicationMode, SkippedFixReason, apply_fixes};
use aztec_lint_core::model::Span;
use aztec_lint_core::output::{json, sarif, text};
use serde_json::Value;
use tempfile::tempdir;

fn diagnostic(file: &str, message: &str) -> Diagnostic {
    Diagnostic {
        rule_id: "NOIR100".to_string(),
        severity: Severity::Warning,
        confidence: Confidence::High,
        policy: "maintainability".to_string(),
        message: message.to_string(),
        primary_span: Span::new(file, 0, 1, 1, 1),
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

fn grouped_diagnostic(file: &str, message: &str, edits: Vec<(u32, u32, &str)>) -> Diagnostic {
    let mut item = diagnostic(file, message);
    item.suggestion_groups = vec![SuggestionGroup {
        id: "sg0001".to_string(),
        message: "grouped replacement".to_string(),
        applicability: Applicability::MachineApplicable,
        edits: edits
            .into_iter()
            .map(|(start, end, replacement)| TextEdit {
                span: Span::new(file, start, end, 1, start + 1),
                replacement: replacement.to_string(),
            })
            .collect(),
        provenance: Some("regression-gate".to_string()),
    }];
    item
}

fn write_source(root: &Path, contents: &str) -> String {
    let file = root.join("src/main.nr");
    fs::create_dir_all(file.parent().expect("source parent should exist"))
        .expect("source directory should be created");
    fs::write(&file, contents).expect("source file should be written");
    "src/main.nr".to_string()
}

#[test]
fn regression_invariants_accept_valid_grouped_diagnostic() {
    let item = grouped_diagnostic(
        "src/main.nr",
        "valid grouped diagnostic",
        vec![(10, 11, "A")],
    );
    let violations = validate_diagnostics(&[item]);
    assert!(violations.is_empty(), "violations should be empty");
}

#[test]
fn regression_invariants_reject_invalid_group_span_and_missing_suppression_reason() {
    let mut item = grouped_diagnostic(
        "src/main.nr",
        "invalid grouped diagnostic",
        vec![(12, 11, "A")],
    );
    item.suppressed = true;
    item.suppression_reason = None;

    let violations = validate_diagnostics(&[item]);
    assert_eq!(violations.len(), 2);
    assert!(violations.iter().any(|violation| matches!(
        violation.kind,
        DiagnosticViolationKind::MissingSuppressionReason
    )));
    assert!(violations.iter().any(|violation| matches!(
        violation.kind,
        DiagnosticViolationKind::InvalidSuggestionGroupEditSpan { .. }
    )));
}

#[test]
fn regression_grouped_fix_rejects_entire_group_when_any_edit_span_is_invalid() {
    let temp = tempdir().expect("temp dir should be created");
    let root = temp.path();
    let file = write_source(root, "fn main() {\n    let lhs = 1;\n    let rhs = 2;\n}\n");

    let mut item = grouped_diagnostic(&file, "atomic grouped fix", vec![(26, 27, "ONE")]);
    item.suggestion_groups[0].edits.push(TextEdit {
        span: Span::new(&file, 999, 1000, 99, 1),
        replacement: "TWO".to_string(),
    });

    let report =
        apply_fixes(root, &[item], FixApplicationMode::Apply).expect("fix application should work");
    let updated = fs::read_to_string(root.join(&file)).expect("source should remain readable");

    assert!(report.selected.is_empty());
    assert_eq!(report.files_changed, 0);
    assert!(
        report
            .skipped
            .iter()
            .any(|skipped| skipped.reason == SkippedFixReason::InvalidGroupSpan)
    );
    assert_eq!(
        updated,
        "fn main() {\n    let lhs = 1;\n    let rhs = 2;\n}\n"
    );
}

#[test]
fn regression_grouped_fix_overlap_winner_is_deterministic_across_input_order() {
    let source = "fn main() { let value = 10; }\n";
    let literal_start =
        u32::try_from(source.find("10").expect("literal must exist")).expect("index must fit u32");
    let literal_end = literal_start + 2;

    let high = grouped_diagnostic(
        "src/main.nr",
        "high confidence",
        vec![(literal_start, literal_end, "HIGH")],
    );
    let low = Diagnostic {
        confidence: Confidence::Low,
        ..grouped_diagnostic(
            "src/main.nr",
            "low confidence",
            vec![(literal_start, literal_end, "LOW")],
        )
    };

    let first_temp = tempdir().expect("temp dir should be created");
    let first_root = first_temp.path();
    let file = write_source(first_root, source);
    let first_report = apply_fixes(
        first_root,
        &[low.clone(), high.clone()],
        FixApplicationMode::Apply,
    )
    .expect("fix application should succeed");
    let first_result =
        fs::read_to_string(first_root.join(&file)).expect("updated source should be readable");

    let second_temp = tempdir().expect("temp dir should be created");
    let second_root = second_temp.path();
    write_source(second_root, source);
    let second_report = apply_fixes(second_root, &[high, low], FixApplicationMode::Apply)
        .expect("fix application should succeed");
    let second_result =
        fs::read_to_string(second_root.join(&file)).expect("updated source should be readable");

    assert_eq!(first_result, "fn main() { let value = HIGH; }\n");
    assert_eq!(second_result, first_result);
    assert_eq!(first_report.selected.len(), 1);
    assert_eq!(second_report.selected.len(), 1);
    assert!(
        first_report
            .skipped
            .iter()
            .any(|skipped| skipped.reason == SkippedFixReason::GroupOverlap)
    );
    assert!(
        second_report
            .skipped
            .iter()
            .any(|skipped| skipped.reason == SkippedFixReason::GroupOverlap)
    );
}

#[test]
fn regression_json_grouped_round_trip_is_deterministic() {
    let item = grouped_diagnostic(
        "src/main.nr",
        "round trip",
        vec![(10, 11, "A"), (20, 21, "B")],
    );
    let first = json::render_diagnostics(&[&item]).expect("json rendering should pass");

    let decoded: Vec<Diagnostic> = serde_json::from_str(&first).expect("json should deserialize");
    let decoded_refs = decoded.iter().collect::<Vec<_>>();
    let second = json::render_diagnostics(&decoded_refs).expect("json rendering should pass");

    let first_value: Value = serde_json::from_str(&first).expect("first json should parse");
    let second_value: Value = serde_json::from_str(&second).expect("second json should parse");
    assert_eq!(first_value, second_value);
}

#[test]
fn regression_sarif_grouped_render_is_deterministic() {
    let item = grouped_diagnostic("src/main.nr", "sarif deterministic", vec![(10, 11, "A")]);
    let first = sarif::render_diagnostics(Path::new("/repo"), &[&item])
        .expect("sarif rendering should pass");
    let second = sarif::render_diagnostics(Path::new("/repo"), &[&item])
        .expect("sarif rendering should pass");
    assert_eq!(first, second);
}

#[test]
fn regression_text_grouped_render_is_deterministic() {
    let temp = tempdir().expect("temp dir should be created");
    let root = temp.path();
    let file = write_source(root, "fn main() { let x = 7; }\n");
    let item = grouped_diagnostic(&file, "text deterministic", vec![(20, 21, "NAMED_CONST")]);

    let report = text::CheckTextReport {
        path: root,
        source_root: root,
        show_run_header: true,
        profile: "default",
        changed_only: false,
        active_rules: 1,
        diagnostics: &[&item],
    };
    let first = text::render_check_report(report);

    let report = text::CheckTextReport {
        path: root,
        source_root: root,
        show_run_header: true,
        profile: "default",
        changed_only: false,
        active_rules: 1,
        diagnostics: &[&item],
    };
    let second = text::render_check_report(report);

    assert_eq!(first, second);
}
