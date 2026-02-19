use serde::{Deserialize, Serialize};

use crate::model::Span;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixSafety {
    Safe,
    NeedsReview,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Fix {
    pub description: String,
    pub span: Span,
    pub replacement: String,
    pub safety: FixSafety,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub rule_id: String,
    pub severity: Severity,
    pub confidence: Confidence,
    pub policy: String,
    pub message: String,
    pub primary_span: Span,
    pub secondary_spans: Vec<Span>,
    pub suggestions: Vec<String>,
    pub fixes: Vec<Fix>,
    pub suppressed: bool,
    pub suppression_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{Confidence, Diagnostic, Fix, FixSafety, Severity};
    use crate::model::Span;

    #[test]
    fn diagnostic_json_shape_matches_spec_contract() {
        let diagnostic = Diagnostic {
            rule_id: "AZTEC001".to_string(),
            severity: Severity::Error,
            confidence: Confidence::High,
            policy: "privacy".to_string(),
            message: "secret value reaches public sink".to_string(),
            primary_span: Span::new("src/contract.nr", 20, 30, 3, 5),
            secondary_spans: vec![Span::new("src/contract.nr", 5, 12, 2, 1)],
            suggestions: vec!["remove the sink".to_string()],
            fixes: vec![Fix {
                description: "replace with constrained sink".to_string(),
                span: Span::new("src/contract.nr", 20, 30, 3, 5),
                replacement: "self.safe_sink(value);".to_string(),
                safety: FixSafety::NeedsReview,
            }],
            suppressed: false,
            suppression_reason: None,
        };

        let value: Value =
            serde_json::to_value(diagnostic).expect("diagnostic serialization must succeed");
        let expected = json!({
            "rule_id": "AZTEC001",
            "severity": "error",
            "confidence": "high",
            "policy": "privacy",
            "message": "secret value reaches public sink",
            "primary_span": {
                "file": "src/contract.nr",
                "start": 20,
                "end": 30,
                "line": 3,
                "col": 5
            },
            "secondary_spans": [
                {
                    "file": "src/contract.nr",
                    "start": 5,
                    "end": 12,
                    "line": 2,
                    "col": 1
                }
            ],
            "suggestions": ["remove the sink"],
            "fixes": [
                {
                    "description": "replace with constrained sink",
                    "span": {
                        "file": "src/contract.nr",
                        "start": 20,
                        "end": 30,
                        "line": 3,
                        "col": 5
                    },
                    "replacement": "self.safe_sink(value);",
                    "safety": "needs_review"
                }
            ],
            "suppressed": false,
            "suppression_reason": null
        });

        assert_eq!(value, expected);
    }

    #[test]
    fn minimal_diagnostic_shape_stays_stable() {
        let diagnostic = Diagnostic {
            rule_id: "NOIR100".to_string(),
            severity: Severity::Warning,
            confidence: Confidence::Medium,
            policy: "maintainability".to_string(),
            message: "magic number".to_string(),
            primary_span: Span::new("src/main.nr", 1, 2, 1, 1),
            secondary_spans: Vec::new(),
            suggestions: Vec::new(),
            fixes: Vec::new(),
            suppressed: true,
            suppression_reason: Some("allow(noir_core::NOIR100)".to_string()),
        };

        let rendered = serde_json::to_string_pretty(&diagnostic)
            .expect("diagnostic pretty serialization must succeed");
        let expected = r#"{
  "rule_id": "NOIR100",
  "severity": "warning",
  "confidence": "medium",
  "policy": "maintainability",
  "message": "magic number",
  "primary_span": {
    "file": "src/main.nr",
    "start": 1,
    "end": 2,
    "line": 1,
    "col": 1
  },
  "secondary_spans": [],
  "suggestions": [],
  "fixes": [],
  "suppressed": true,
  "suppression_reason": "allow(noir_core::NOIR100)"
}"#;

        assert_eq!(rendered, expected);
    }
}
