use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::diagnostics::{
    Diagnostic, Severity, diagnostic_fingerprint, diagnostic_sort_key, normalize_file_path,
};

const PARTIAL_FINGERPRINT_KEY: &str = "aztecLint/v1";

pub fn render_diagnostics(
    repo_root: &Path,
    diagnostics: &[&Diagnostic],
) -> Result<String, serde_json::Error> {
    let mut sorted = diagnostics.to_vec();
    sorted.sort_by_key(|diagnostic| diagnostic_sort_key(diagnostic));

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

    let results = sorted
        .iter()
        .map(|diagnostic| {
            json!({
                "ruleId": diagnostic.rule_id,
                "level": sarif_level(diagnostic.severity),
                "message": { "text": diagnostic.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": repository_relative_uri(repo_root, &diagnostic.primary_span.file) },
                        "region": {
                            "startLine": diagnostic.primary_span.line,
                            "startColumn": diagnostic.primary_span.col,
                        }
                    }
                }],
                "partialFingerprints": {
                    PARTIAL_FINGERPRINT_KEY: diagnostic_fingerprint(diagnostic),
                },
                "properties": {
                    "confidence": diagnostic.confidence,
                    "policy": diagnostic.policy,
                }
            })
        })
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
            "results": results,
        }]
    });

    serde_json::to_string_pretty(&sarif)
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
    use crate::diagnostics::{Confidence, Diagnostic, Severity};
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
}
