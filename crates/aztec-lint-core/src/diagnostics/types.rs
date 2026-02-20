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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Applicability {
    MachineApplicable,
    MaybeIncorrect,
    HasPlaceholders,
    Unspecified,
}

impl Applicability {
    pub fn to_fix_safety(self) -> FixSafety {
        match self {
            Self::MachineApplicable => FixSafety::Safe,
            Self::MaybeIncorrect | Self::HasPlaceholders | Self::Unspecified => {
                FixSafety::NeedsReview
            }
        }
    }
}

impl From<Applicability> for FixSafety {
    fn from(value: Applicability) -> Self {
        value.to_fix_safety()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Fix {
    pub description: String,
    pub span: Span,
    pub replacement: String,
    pub safety: FixSafety,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StructuredMessage {
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StructuredSuggestion {
    pub message: String,
    pub span: Span,
    pub replacement: String,
    pub applicability: Applicability,
}

impl StructuredSuggestion {
    pub fn to_fix(&self) -> Fix {
        Fix {
            description: self.message.clone(),
            span: self.span.clone(),
            replacement: self.replacement.clone(),
            safety: self.applicability.into(),
        }
    }
}

impl Diagnostic {
    pub fn fixes_from_structured_suggestions(&self) -> Vec<Fix> {
        self.structured_suggestions
            .iter()
            .map(StructuredSuggestion::to_fix)
            .collect()
    }
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<StructuredMessage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub helps: Vec<StructuredMessage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub structured_suggestions: Vec<StructuredSuggestion>,
    pub fixes: Vec<Fix>,
    pub suppressed: bool,
    pub suppression_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{
        Applicability, Confidence, Diagnostic, Fix, FixSafety, Severity, StructuredMessage,
        StructuredSuggestion,
    };
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
            notes: vec![StructuredMessage {
                message: "taint reached a public sink".to_string(),
                span: Some(Span::new("src/contract.nr", 5, 12, 2, 1)),
            }],
            helps: vec![StructuredMessage {
                message: "consider constraining the value before emitting".to_string(),
                span: None,
            }],
            structured_suggestions: vec![StructuredSuggestion {
                message: "replace with constrained sink".to_string(),
                span: Span::new("src/contract.nr", 20, 30, 3, 5),
                replacement: "self.safe_sink(value);".to_string(),
                applicability: Applicability::MaybeIncorrect,
            }],
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
            "notes": [
                {
                    "message": "taint reached a public sink",
                    "span": {
                        "file": "src/contract.nr",
                        "start": 5,
                        "end": 12,
                        "line": 2,
                        "col": 1
                    }
                }
            ],
            "helps": [
                {
                    "message": "consider constraining the value before emitting"
                }
            ],
            "structured_suggestions": [
                {
                    "message": "replace with constrained sink",
                    "span": {
                        "file": "src/contract.nr",
                        "start": 20,
                        "end": 30,
                        "line": 3,
                        "col": 5
                    },
                    "replacement": "self.safe_sink(value);",
                    "applicability": "maybe_incorrect"
                }
            ],
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
            notes: Vec::new(),
            helps: Vec::new(),
            structured_suggestions: Vec::new(),
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

    #[test]
    fn machine_applicable_suggestion_maps_to_safe_fix() {
        let suggestion = StructuredSuggestion {
            message: "replace value".to_string(),
            span: Span::new("src/main.nr", 10, 12, 2, 3),
            replacement: "42".to_string(),
            applicability: Applicability::MachineApplicable,
        };

        let fix = suggestion.to_fix();
        assert_eq!(fix.safety, FixSafety::Safe);
    }

    #[test]
    fn non_machine_applicable_suggestion_maps_to_needs_review_fix() {
        let suggestion = StructuredSuggestion {
            message: "replace value".to_string(),
            span: Span::new("src/main.nr", 10, 12, 2, 3),
            replacement: "42".to_string(),
            applicability: Applicability::MaybeIncorrect,
        };

        let fix = suggestion.to_fix();
        assert_eq!(fix.safety, FixSafety::NeedsReview);
    }

    #[test]
    fn all_non_machine_applicable_suggestions_map_to_needs_review_fix() {
        let variants = [
            Applicability::MaybeIncorrect,
            Applicability::HasPlaceholders,
            Applicability::Unspecified,
        ];

        for applicability in variants {
            let suggestion = StructuredSuggestion {
                message: "replace value".to_string(),
                span: Span::new("src/main.nr", 10, 12, 2, 3),
                replacement: "42".to_string(),
                applicability,
            };
            assert_eq!(suggestion.to_fix().safety, FixSafety::NeedsReview);
        }
    }

    #[test]
    fn diagnostic_can_derive_fixes_from_structured_suggestions() {
        let diagnostic = Diagnostic {
            rule_id: "NOIR999".to_string(),
            severity: Severity::Warning,
            confidence: Confidence::Medium,
            policy: "maintainability".to_string(),
            message: "message".to_string(),
            primary_span: Span::new("src/main.nr", 1, 2, 1, 1),
            secondary_spans: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
            structured_suggestions: vec![StructuredSuggestion {
                message: "replace value".to_string(),
                span: Span::new("src/main.nr", 10, 12, 2, 3),
                replacement: "42".to_string(),
                applicability: Applicability::MachineApplicable,
            }],
            fixes: Vec::new(),
            suppressed: false,
            suppression_reason: None,
        };

        let derived = diagnostic.fixes_from_structured_suggestions();
        assert_eq!(derived.len(), 1);
        assert_eq!(derived[0].description, "replace value");
        assert_eq!(derived[0].replacement, "42");
        assert_eq!(derived[0].safety, FixSafety::Safe);
    }

    #[test]
    fn legacy_diagnostic_json_deserializes_with_phase1_defaults() {
        let legacy = json!({
            "rule_id": "NOIR100",
            "severity": "warning",
            "confidence": "low",
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
            "suppressed": false,
            "suppression_reason": null
        });

        let diagnostic: Diagnostic =
            serde_json::from_value(legacy).expect("legacy shape should deserialize");

        assert!(diagnostic.notes.is_empty());
        assert!(diagnostic.helps.is_empty());
        assert!(diagnostic.structured_suggestions.is_empty());
    }
}
