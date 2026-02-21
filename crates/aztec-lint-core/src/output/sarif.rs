use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::diagnostics::{
    Diagnostic, Severity, diagnostic_fingerprint, diagnostic_sort_key, normalize_file_path,
};
use crate::model::Span;

const PARTIAL_FINGERPRINT_KEY: &str = "aztecLint/v1";

pub fn render_diagnostics(
    repo_root: &Path,
    diagnostics: &[&Diagnostic],
) -> Result<String, serde_json::Error> {
    let mut sorted = diagnostics
        .iter()
        .map(|diagnostic| normalize_for_sarif((**diagnostic).clone()))
        .collect::<Vec<_>>();
    sorted.sort_by_key(diagnostic_sort_key);

    let mut rule_descriptions = BTreeMap::<String, String>::new();
    for diagnostic in &sorted {
        rule_descriptions
            .entry(diagnostic.rule_id.clone())
            .or_insert_with(|| diagnostic.message.clone());
    }

    let rules = rule_descriptions
        .into_iter()
        .map(|(rule_id, description)| {
            json!({
                "id": rule_id,
                "name": rule_id,
                "shortDescription": { "text": description },
            })
        })
        .collect::<Vec<_>>();

    let (artifacts, artifact_indices) = build_artifact_catalog(repo_root, &sorted);
    let results = sorted
        .iter()
        .map(|diagnostic| render_result(repo_root, diagnostic, &artifact_indices))
        .collect::<Vec<_>>();

    let sarif = json!({
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "aztec-lint",
                    "rules": rules,
                }
            },
            "artifacts": artifacts,
            "results": results,
        }]
    });

    serde_json::to_string_pretty(&sarif)
}

fn normalize_for_sarif(mut diagnostic: Diagnostic) -> Diagnostic {
    diagnostic.merge_legacy_fields_from_suggestion_groups();
    diagnostic.suggestions.sort();
    diagnostic.notes.sort_by_key(|note| {
        if let Some(span) = &note.span {
            (
                0u8,
                span.file.clone(),
                span.line,
                span.col,
                span.start,
                span.end,
                note.message.clone(),
            )
        } else {
            (
                1u8,
                String::new(),
                0u32,
                0u32,
                0u32,
                0u32,
                note.message.clone(),
            )
        }
    });
    diagnostic.helps.sort_by_key(|help| {
        if let Some(span) = &help.span {
            (
                0u8,
                span.file.clone(),
                span.line,
                span.col,
                span.start,
                span.end,
                help.message.clone(),
            )
        } else {
            (
                1u8,
                String::new(),
                0u32,
                0u32,
                0u32,
                0u32,
                help.message.clone(),
            )
        }
    });
    diagnostic.structured_suggestions.sort_by_key(|suggestion| {
        (
            normalize_file_path(&suggestion.span.file),
            suggestion.span.start,
            suggestion.span.end,
            suggestion.message.clone(),
            suggestion.replacement.clone(),
            suggestion.applicability.as_str().to_string(),
        )
    });
    for group in &mut diagnostic.suggestion_groups {
        group.edits.sort_by_key(|edit| {
            (
                normalize_file_path(&edit.span.file),
                edit.span.start,
                edit.span.end,
                edit.replacement.clone(),
            )
        });
    }
    diagnostic.suggestion_groups.sort_by_key(|group| {
        (
            group.id.clone(),
            group.message.clone(),
            group.applicability.as_str().to_string(),
            group.provenance.clone().unwrap_or_default(),
            group.edits.len(),
        )
    });
    diagnostic.fixes.sort_by_key(|fix| {
        (
            normalize_file_path(&fix.span.file),
            fix.span.start,
            fix.span.end,
            fix.description.clone(),
            fix.replacement.clone(),
            format!("{:?}", fix.safety),
        )
    });
    diagnostic
}

fn render_result(
    repo_root: &Path,
    diagnostic: &Diagnostic,
    artifact_indices: &BTreeMap<String, usize>,
) -> Value {
    let mut result = Map::<String, Value>::new();
    result.insert(
        "ruleId".to_string(),
        Value::String(diagnostic.rule_id.clone()),
    );
    result.insert(
        "level".to_string(),
        Value::String(sarif_level(diagnostic.severity).to_string()),
    );
    result.insert(
        "message".to_string(),
        json!({ "text": diagnostic.message.clone() }),
    );
    result.insert(
        "locations".to_string(),
        Value::Array(vec![json!({
            "physicalLocation": {
                "artifactLocation": artifact_location_value(
                    &repository_relative_uri(repo_root, &diagnostic.primary_span.file),
                    artifact_indices,
                ),
                "region": region_for_span(&diagnostic.primary_span),
            }
        })]),
    );
    result.insert(
        "partialFingerprints".to_string(),
        json!({
            PARTIAL_FINGERPRINT_KEY: diagnostic_fingerprint(diagnostic),
        }),
    );
    result.insert(
        "properties".to_string(),
        json!({
            "confidence": diagnostic.confidence,
            "policy": diagnostic.policy,
            "suppressed": diagnostic.suppressed,
            "suppressionReason": diagnostic.suppression_reason,
            "legacySuggestions": diagnostic.suggestions,
            "notes": diagnostic.notes,
            "helps": diagnostic.helps,
            "structuredSuggestions": diagnostic.structured_suggestions,
            "legacyFixes": diagnostic.fixes,
        }),
    );

    let fixes = sarif_fixes(repo_root, diagnostic, artifact_indices);
    if !fixes.is_empty() {
        result.insert("fixes".to_string(), Value::Array(fixes));
    }

    Value::Object(result)
}

fn sarif_fixes(
    repo_root: &Path,
    diagnostic: &Diagnostic,
    artifact_indices: &BTreeMap<String, usize>,
) -> Vec<Value> {
    let mut fixes = Vec::<Value>::new();

    let mut legacy_fixes = diagnostic.fixes.clone();
    legacy_fixes.sort_by_key(|fix| {
        (
            normalize_file_path(&fix.span.file),
            fix.span.start,
            fix.span.end,
            fix.description.clone(),
            fix.replacement.clone(),
            format!("{:?}", fix.safety),
        )
    });

    for fix in legacy_fixes {
        let uri = repository_relative_uri(repo_root, &fix.span.file);
        fixes.push(json!({
            "description": { "text": fix.description },
            "artifactChanges": [{
                "artifactLocation": artifact_location_value(&uri, artifact_indices),
                "replacements": [{
                    "deletedRegion": region_for_span(&fix.span),
                    "insertedContent": { "text": fix.replacement },
                }]
            }],
            "properties": {
                "source": "legacy_fix",
                "safety": fix.safety,
            }
        }));
    }

    if !diagnostic.suggestion_groups.is_empty() {
        let mut grouped_suggestion_keys =
            BTreeSet::<(String, u32, u32, String, String, String)>::new();
        let mut groups = diagnostic.suggestion_groups.clone();
        groups.sort_by_key(|group| {
            (
                group.id.clone(),
                group.message.clone(),
                group.applicability.as_str().to_string(),
                group.provenance.clone().unwrap_or_default(),
                group.edits.len(),
            )
        });

        for group in groups {
            for edit in &group.edits {
                grouped_suggestion_keys.insert((
                    normalize_file_path(&edit.span.file),
                    edit.span.start,
                    edit.span.end,
                    group.message.clone(),
                    edit.replacement.clone(),
                    group.applicability.as_str().to_string(),
                ));
            }

            let mut edits_by_uri = BTreeMap::<String, Vec<_>>::new();
            for edit in group.edits {
                let uri = repository_relative_uri(repo_root, &edit.span.file);
                edits_by_uri.entry(uri).or_default().push(edit);
            }

            let artifact_changes = edits_by_uri
                .into_iter()
                .map(|(uri, mut edits)| {
                    edits.sort_by_key(|edit| {
                        (
                            normalize_file_path(&edit.span.file),
                            edit.span.start,
                            edit.span.end,
                            edit.replacement.clone(),
                        )
                    });
                    json!({
                        "artifactLocation": artifact_location_value(&uri, artifact_indices),
                        "replacements": edits
                            .iter()
                            .map(|edit| {
                                json!({
                                    "deletedRegion": region_for_span(&edit.span),
                                    "insertedContent": { "text": edit.replacement },
                                })
                            })
                            .collect::<Vec<_>>(),
                    })
                })
                .collect::<Vec<_>>();

            fixes.push(json!({
                "description": { "text": group.message },
                "artifactChanges": artifact_changes,
                "properties": {
                    "source": "suggestion_group",
                    "groupId": group.id,
                    "applicability": group.applicability,
                    "provenance": group.provenance,
                }
            }));
        }

        let mut structured_suggestions = diagnostic.structured_suggestions.clone();
        structured_suggestions.sort_by_key(|suggestion| {
            (
                normalize_file_path(&suggestion.span.file),
                suggestion.span.start,
                suggestion.span.end,
                suggestion.message.clone(),
                suggestion.replacement.clone(),
                suggestion.applicability.as_str().to_string(),
            )
        });

        for suggestion in structured_suggestions {
            let key = (
                normalize_file_path(&suggestion.span.file),
                suggestion.span.start,
                suggestion.span.end,
                suggestion.message.clone(),
                suggestion.replacement.clone(),
                suggestion.applicability.as_str().to_string(),
            );
            if grouped_suggestion_keys.contains(&key) {
                continue;
            }

            let uri = repository_relative_uri(repo_root, &suggestion.span.file);
            fixes.push(json!({
                "description": { "text": suggestion.message },
                "artifactChanges": [{
                    "artifactLocation": artifact_location_value(&uri, artifact_indices),
                    "replacements": [{
                        "deletedRegion": region_for_span(&suggestion.span),
                        "insertedContent": { "text": suggestion.replacement },
                    }]
                }],
                "properties": {
                    "source": "structured_suggestion",
                    "applicability": suggestion.applicability,
                }
            }));
        }
    } else {
        let mut structured_suggestions = diagnostic.structured_suggestions.clone();
        structured_suggestions.sort_by_key(|suggestion| {
            (
                normalize_file_path(&suggestion.span.file),
                suggestion.span.start,
                suggestion.span.end,
                suggestion.message.clone(),
                suggestion.replacement.clone(),
                suggestion.applicability.as_str().to_string(),
            )
        });

        for suggestion in structured_suggestions {
            let uri = repository_relative_uri(repo_root, &suggestion.span.file);
            fixes.push(json!({
                "description": { "text": suggestion.message },
                "artifactChanges": [{
                    "artifactLocation": artifact_location_value(&uri, artifact_indices),
                    "replacements": [{
                        "deletedRegion": region_for_span(&suggestion.span),
                        "insertedContent": { "text": suggestion.replacement },
                    }]
                }],
                "properties": {
                    "source": "structured_suggestion",
                    "applicability": suggestion.applicability,
                }
            }));
        }
    }

    fixes
}

fn build_artifact_catalog(
    repo_root: &Path,
    diagnostics: &[Diagnostic],
) -> (Vec<Value>, BTreeMap<String, usize>) {
    let mut uris = BTreeSet::<String>::new();
    for diagnostic in diagnostics {
        uris.insert(repository_relative_uri(
            repo_root,
            &diagnostic.primary_span.file,
        ));
        for span in &diagnostic.secondary_spans {
            uris.insert(repository_relative_uri(repo_root, &span.file));
        }
        for note in &diagnostic.notes {
            if let Some(span) = &note.span {
                uris.insert(repository_relative_uri(repo_root, &span.file));
            }
        }
        for help in &diagnostic.helps {
            if let Some(span) = &help.span {
                uris.insert(repository_relative_uri(repo_root, &span.file));
            }
        }
        for suggestion in &diagnostic.structured_suggestions {
            uris.insert(repository_relative_uri(repo_root, &suggestion.span.file));
        }
        for group in &diagnostic.suggestion_groups {
            for edit in &group.edits {
                uris.insert(repository_relative_uri(repo_root, &edit.span.file));
            }
        }
        for fix in &diagnostic.fixes {
            uris.insert(repository_relative_uri(repo_root, &fix.span.file));
        }
    }

    let ordered = uris.into_iter().collect::<Vec<_>>();
    let artifacts = ordered
        .iter()
        .map(|uri| {
            json!({
                "location": {
                    "uri": uri,
                }
            })
        })
        .collect::<Vec<_>>();
    let indices = ordered
        .iter()
        .enumerate()
        .map(|(idx, uri)| (uri.clone(), idx))
        .collect::<BTreeMap<_, _>>();
    (artifacts, indices)
}

fn artifact_location_value(uri: &str, artifact_indices: &BTreeMap<String, usize>) -> Value {
    let mut artifact_location = Map::<String, Value>::new();
    artifact_location.insert("uri".to_string(), Value::String(uri.to_string()));
    if let Some(index) = artifact_indices.get(uri) {
        artifact_location.insert("index".to_string(), json!(*index));
    }
    Value::Object(artifact_location)
}

fn region_for_span(span: &Span) -> Value {
    let width = span.end.saturating_sub(span.start);
    let end_column = if width == 0 {
        span.col
    } else {
        span.col.saturating_add(width)
    };
    json!({
        "startLine": span.line,
        "startColumn": span.col,
        "endLine": span.line,
        "endColumn": end_column,
    })
}

fn repository_relative_uri(repo_root: &Path, file: &str) -> String {
    let repo_root = to_absolute_path(repo_root);
    let file_path = Path::new(file);
    let absolute_file = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        repo_root.join(file_path)
    };
    let relative = absolute_file
        .strip_prefix(&repo_root)
        .unwrap_or(absolute_file.as_path());
    normalize_file_path(&relative.to_string_lossy())
}

fn to_absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(path),
        Err(_) => path.to_path_buf(),
    }
}

fn sarif_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::Value;

    use super::{PARTIAL_FINGERPRINT_KEY, render_diagnostics};
    use crate::diagnostics::{
        Applicability, Confidence, Diagnostic, Fix, FixSafety, Severity, StructuredSuggestion,
        SuggestionGroup, TextEdit,
    };
    use crate::model::Span;

    fn diagnostic(rule_id: &str, file: &str, start: u32, line: u32, message: &str) -> Diagnostic {
        Diagnostic {
            rule_id: rule_id.to_string(),
            severity: Severity::Warning,
            confidence: Confidence::Medium,
            policy: "privacy".to_string(),
            message: message.to_string(),
            primary_span: Span::new(file, start, start + 1, line, 2),
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
    fn sarif_output_normalizes_uri_and_partial_fingerprint() {
        let root = Path::new("/repo");
        let issue = diagnostic("AZTEC001", "/repo/src\\main.nr", 10, 2, "message");
        let rendered = render_diagnostics(root, &[&issue]).expect("sarif render should succeed");

        let value: Value = serde_json::from_str(&rendered).expect("sarif should parse");
        let uri =
            value["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]
                ["uri"]
                .as_str()
                .expect("uri should be a string");
        assert_eq!(uri, "src/main.nr");

        let fingerprint = value["runs"][0]["results"][0]["partialFingerprints"]
            [PARTIAL_FINGERPRINT_KEY]
            .as_str()
            .expect("fingerprint should be a string");
        assert!(!fingerprint.is_empty(), "fingerprint should not be empty");
    }

    #[test]
    fn sarif_output_is_deterministic_across_reordered_input() {
        let root = Path::new("/repo");
        let first = diagnostic("AZTEC001", "src/a.nr", 10, 1, "first");
        let second = diagnostic("AZTEC020", "src/b.nr", 11, 2, "second");

        let left = render_diagnostics(root, &[&second, &first]).expect("left render should pass");
        let right = render_diagnostics(root, &[&first, &second]).expect("right render should pass");

        assert_eq!(left, right);
    }

    #[test]
    fn sarif_output_includes_structured_and_legacy_fixes() {
        let root = Path::new("/repo");
        let mut issue = diagnostic("NOIR100", "src/main.nr", 10, 2, "message");
        issue.fixes = vec![Fix {
            description: "legacy fix".to_string(),
            span: Span::new("src/main.nr", 10, 11, 2, 2),
            replacement: "CONST".to_string(),
            safety: FixSafety::Safe,
        }];
        issue.structured_suggestions = vec![StructuredSuggestion {
            message: "structured fix".to_string(),
            span: Span::new("src/main.nr", 10, 11, 2, 2),
            replacement: "NAMED".to_string(),
            applicability: Applicability::MachineApplicable,
        }];
        issue.suggestion_groups = vec![SuggestionGroup {
            id: "sg0001".to_string(),
            message: "structured fix".to_string(),
            applicability: Applicability::MachineApplicable,
            edits: vec![TextEdit {
                span: Span::new("src/main.nr", 10, 11, 2, 2),
                replacement: "NAMED".to_string(),
            }],
            provenance: None,
        }];

        let rendered = render_diagnostics(root, &[&issue]).expect("sarif render should succeed");
        let value: Value = serde_json::from_str(&rendered).expect("sarif should parse");
        let fixes = value["runs"][0]["results"][0]["fixes"]
            .as_array()
            .expect("fixes should be present");

        assert_eq!(fixes.len(), 2);
        assert_eq!(
            fixes[0]["properties"]["source"].as_str(),
            Some("legacy_fix")
        );
        assert_eq!(
            fixes[1]["properties"]["source"].as_str(),
            Some("suggestion_group")
        );
        assert_eq!(
            fixes[1]["properties"]["applicability"].as_str(),
            Some("machine_applicable")
        );
        assert_eq!(
            value["runs"][0]["artifacts"][0]["location"]["uri"].as_str(),
            Some("src/main.nr")
        );
    }

    #[test]
    fn sarif_output_preserves_legacy_structured_suggestions_when_groups_exist() {
        let root = Path::new("/repo");
        let mut issue = diagnostic("NOIR100", "src/main.nr", 10, 2, "message");
        issue.structured_suggestions = vec![
            StructuredSuggestion {
                message: "group fix".to_string(),
                span: Span::new("src/main.nr", 10, 11, 2, 2),
                replacement: "NAMED".to_string(),
                applicability: Applicability::MachineApplicable,
            },
            StructuredSuggestion {
                message: "legacy-only fix".to_string(),
                span: Span::new("src/main.nr", 20, 22, 4, 4),
                replacement: "RENAMED".to_string(),
                applicability: Applicability::MaybeIncorrect,
            },
        ];
        issue.suggestion_groups = vec![SuggestionGroup {
            id: "sg0001".to_string(),
            message: "group fix".to_string(),
            applicability: Applicability::MachineApplicable,
            edits: vec![TextEdit {
                span: Span::new("src/main.nr", 10, 11, 2, 2),
                replacement: "NAMED".to_string(),
            }],
            provenance: None,
        }];

        let rendered = render_diagnostics(root, &[&issue]).expect("sarif render should succeed");
        let value: Value = serde_json::from_str(&rendered).expect("sarif should parse");
        let fixes = value["runs"][0]["results"][0]["fixes"]
            .as_array()
            .expect("fixes should be present");

        assert_eq!(fixes.len(), 2);
        let sources = fixes
            .iter()
            .map(|fix| fix["properties"]["source"].as_str().unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(sources, vec!["suggestion_group", "structured_suggestion"]);
    }

    #[test]
    fn sarif_properties_are_deterministically_normalized() {
        let root = Path::new("/repo");
        let mut issue = diagnostic("NOIR100", "src/main.nr", 10, 2, "message");
        issue.suggestions = vec!["z legacy".to_string(), "a legacy".to_string()];
        issue.notes = vec![
            crate::diagnostics::StructuredMessage {
                message: "z note".to_string(),
                span: Some(Span::new("src/main.nr", 20, 21, 3, 5)),
            },
            crate::diagnostics::StructuredMessage {
                message: "a note".to_string(),
                span: Some(Span::new("src/main.nr", 10, 11, 2, 2)),
            },
        ];
        issue.helps = vec![
            crate::diagnostics::StructuredMessage {
                message: "z help".to_string(),
                span: None,
            },
            crate::diagnostics::StructuredMessage {
                message: "a help".to_string(),
                span: None,
            },
        ];

        let rendered = render_diagnostics(root, &[&issue]).expect("sarif render should succeed");
        let value: Value = serde_json::from_str(&rendered).expect("sarif should parse");
        let props = &value["runs"][0]["results"][0]["properties"];

        let legacy = props["legacySuggestions"]
            .as_array()
            .expect("legacy suggestions should be an array");
        assert_eq!(legacy[0].as_str(), Some("a legacy"));
        assert_eq!(legacy[1].as_str(), Some("z legacy"));

        let notes = props["notes"].as_array().expect("notes should be an array");
        assert_eq!(notes[0]["message"].as_str(), Some("a note"));
        assert_eq!(notes[1]["message"].as_str(), Some("z note"));

        let helps = props["helps"].as_array().expect("helps should be an array");
        assert_eq!(helps[0]["message"].as_str(), Some("a help"));
        assert_eq!(helps[1]["message"].as_str(), Some("z help"));
    }
}
