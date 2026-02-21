use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

use crate::diagnostics::{Applicability, Diagnostic};
use crate::model::Span;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiagnosticViolationKind {
    EmptyRuleId,
    EmptyPolicy,
    EmptyMessage,
    InvalidPrimarySpan {
        start: u32,
        end: u32,
    },
    InvalidSecondarySpan {
        index: usize,
        start: u32,
        end: u32,
    },
    InvalidNoteSpan {
        index: usize,
        start: u32,
        end: u32,
    },
    InvalidHelpSpan {
        index: usize,
        start: u32,
        end: u32,
    },
    InvalidStructuredSuggestionSpan {
        index: usize,
        start: u32,
        end: u32,
    },
    InvalidFixSpan {
        index: usize,
        start: u32,
        end: u32,
    },
    InvalidSuggestionGroupEditSpan {
        group_id: String,
        edit_index: usize,
        start: u32,
        end: u32,
    },
    OverlappingSuggestionGroupEdits {
        group_id: String,
        first_edit_index: usize,
        second_edit_index: usize,
        first_start: u32,
        first_end: u32,
        second_start: u32,
        second_end: u32,
    },
    MissingSuppressionReason,
    OverlappingStructuredSuggestionGroupSpans {
        message: String,
        applicability: Applicability,
        first_index: usize,
        second_index: usize,
        first_start: u32,
        first_end: u32,
        second_start: u32,
        second_end: u32,
    },
}

impl Display for DiagnosticViolationKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyRuleId => write!(f, "rule_id must be non-empty"),
            Self::EmptyPolicy => write!(f, "policy must be non-empty"),
            Self::EmptyMessage => write!(f, "message must be non-empty"),
            Self::InvalidPrimarySpan { start, end } => {
                write!(f, "primary_span is invalid (start={start}, end={end})")
            }
            Self::InvalidSecondarySpan { index, start, end } => write!(
                f,
                "secondary_spans[{index}] is invalid (start={start}, end={end})"
            ),
            Self::InvalidNoteSpan { index, start, end } => {
                write!(
                    f,
                    "notes[{index}].span is invalid (start={start}, end={end})"
                )
            }
            Self::InvalidHelpSpan { index, start, end } => {
                write!(
                    f,
                    "helps[{index}].span is invalid (start={start}, end={end})"
                )
            }
            Self::InvalidStructuredSuggestionSpan { index, start, end } => write!(
                f,
                "structured_suggestions[{index}].span is invalid (start={start}, end={end})"
            ),
            Self::InvalidFixSpan { index, start, end } => {
                write!(
                    f,
                    "fixes[{index}].span is invalid (start={start}, end={end})"
                )
            }
            Self::InvalidSuggestionGroupEditSpan {
                group_id,
                edit_index,
                start,
                end,
            } => write!(
                f,
                "suggestion_groups['{group_id}'].edits[{edit_index}] span is invalid (start={start}, end={end})"
            ),
            Self::OverlappingSuggestionGroupEdits {
                group_id,
                first_edit_index,
                second_edit_index,
                first_start,
                first_end,
                second_start,
                second_end,
            } => write!(
                f,
                "suggestion_groups['{group_id}'] has overlapping edits: [{}]({first_start},{first_end}) vs [{}]({second_start},{second_end})",
                first_edit_index, second_edit_index
            ),
            Self::MissingSuppressionReason => write!(
                f,
                "suppression_reason must be present when diagnostic is suppressed"
            ),
            Self::OverlappingStructuredSuggestionGroupSpans {
                message,
                applicability,
                first_index,
                second_index,
                first_start,
                first_end,
                second_start,
                second_end,
            } => write!(
                f,
                "structured suggestions in implicit group (message='{message}', applicability='{}') overlap: [{}]({first_start},{first_end}) vs [{}]({second_start},{second_end})",
                applicability.as_str(),
                first_index,
                second_index
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticViolation {
    pub diagnostic_index: usize,
    pub rule_id: String,
    pub primary_span: Span,
    pub kind: DiagnosticViolationKind,
}

impl Display for DiagnosticViolation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let rule_id = if self.rule_id.is_empty() {
            "<empty>"
        } else {
            &self.rule_id
        };
        write!(
            f,
            "diagnostic #{index} rule '{rule_id}' at {file}:{start}-{end}: {kind}",
            index = self.diagnostic_index,
            file = self.primary_span.file,
            start = self.primary_span.start,
            end = self.primary_span.end,
            kind = self.kind
        )
    }
}

pub fn validate_diagnostic(diagnostic: &Diagnostic) -> Vec<DiagnosticViolationKind> {
    let mut violations = Vec::<DiagnosticViolationKind>::new();

    if diagnostic.rule_id.trim().is_empty() {
        violations.push(DiagnosticViolationKind::EmptyRuleId);
    }
    if diagnostic.policy.trim().is_empty() {
        violations.push(DiagnosticViolationKind::EmptyPolicy);
    }
    if diagnostic.message.trim().is_empty() {
        violations.push(DiagnosticViolationKind::EmptyMessage);
    }
    if !span_is_valid(&diagnostic.primary_span) {
        violations.push(DiagnosticViolationKind::InvalidPrimarySpan {
            start: diagnostic.primary_span.start,
            end: diagnostic.primary_span.end,
        });
    }

    for (index, span) in diagnostic.secondary_spans.iter().enumerate() {
        if !span_is_valid(span) {
            violations.push(DiagnosticViolationKind::InvalidSecondarySpan {
                index,
                start: span.start,
                end: span.end,
            });
        }
    }

    for (index, note) in diagnostic.notes.iter().enumerate() {
        let Some(span) = note.span.as_ref() else {
            continue;
        };
        if !span_is_valid(span) {
            violations.push(DiagnosticViolationKind::InvalidNoteSpan {
                index,
                start: span.start,
                end: span.end,
            });
        }
    }

    for (index, help) in diagnostic.helps.iter().enumerate() {
        let Some(span) = help.span.as_ref() else {
            continue;
        };
        if !span_is_valid(span) {
            violations.push(DiagnosticViolationKind::InvalidHelpSpan {
                index,
                start: span.start,
                end: span.end,
            });
        }
    }

    for (index, suggestion) in diagnostic.structured_suggestions.iter().enumerate() {
        if !span_is_valid(&suggestion.span) {
            violations.push(DiagnosticViolationKind::InvalidStructuredSuggestionSpan {
                index,
                start: suggestion.span.start,
                end: suggestion.span.end,
            });
        }
    }

    for (index, fix) in diagnostic.fixes.iter().enumerate() {
        if !span_is_valid(&fix.span) {
            violations.push(DiagnosticViolationKind::InvalidFixSpan {
                index,
                start: fix.span.start,
                end: fix.span.end,
            });
        }
    }

    if diagnostic.suppressed && diagnostic.suppression_reason.is_none() {
        violations.push(DiagnosticViolationKind::MissingSuppressionReason);
    }

    if !diagnostic.suggestion_groups.is_empty() {
        for group in &diagnostic.suggestion_groups {
            for (edit_index, edit) in group.edits.iter().enumerate() {
                if !span_is_valid(&edit.span) {
                    violations.push(DiagnosticViolationKind::InvalidSuggestionGroupEditSpan {
                        group_id: group.id.clone(),
                        edit_index,
                        start: edit.span.start,
                        end: edit.span.end,
                    });
                }
            }

            for i in 0..group.edits.len() {
                for j in (i + 1)..group.edits.len() {
                    let first = &group.edits[i];
                    let second = &group.edits[j];
                    if first.span.file != second.span.file {
                        continue;
                    }
                    if ranges_overlap(
                        first.span.start,
                        first.span.end,
                        second.span.start,
                        second.span.end,
                    ) {
                        violations.push(DiagnosticViolationKind::OverlappingSuggestionGroupEdits {
                            group_id: group.id.clone(),
                            first_edit_index: i,
                            second_edit_index: j,
                            first_start: first.span.start,
                            first_end: first.span.end,
                            second_start: second.span.start,
                            second_end: second.span.end,
                        });
                    }
                }
            }
        }
    } else {
        // Legacy fallback for flattened multipart suggestions.
        let mut grouped_spans =
            BTreeMap::<(String, Applicability, String), Vec<(usize, u32, u32)>>::new();
        for (index, suggestion) in diagnostic.structured_suggestions.iter().enumerate() {
            grouped_spans
                .entry((
                    suggestion.message.clone(),
                    suggestion.applicability,
                    suggestion.span.file.clone(),
                ))
                .or_default()
                .push((index, suggestion.span.start, suggestion.span.end));
        }

        for ((message, applicability, _file), spans) in grouped_spans {
            if spans.len() < 2 {
                continue;
            }

            let mut ordered = spans;
            ordered.sort_by_key(|(index, start, end)| (*start, *end, *index));
            for i in 0..ordered.len() {
                for j in (i + 1)..ordered.len() {
                    let (first_index, first_start, first_end) = ordered[i];
                    let (second_index, second_start, second_end) = ordered[j];
                    if ranges_overlap(first_start, first_end, second_start, second_end) {
                        violations.push(
                            DiagnosticViolationKind::OverlappingStructuredSuggestionGroupSpans {
                                message: message.clone(),
                                applicability,
                                first_index,
                                second_index,
                                first_start,
                                first_end,
                                second_start,
                                second_end,
                            },
                        );
                    }
                }
            }
        }
    }

    violations
}

pub fn validate_diagnostics(diagnostics: &[Diagnostic]) -> Vec<DiagnosticViolation> {
    let mut violations = Vec::<DiagnosticViolation>::new();

    for (diagnostic_index, diagnostic) in diagnostics.iter().enumerate() {
        let rule_id = diagnostic.rule_id.clone();
        let primary_span = diagnostic.primary_span.clone();
        for kind in validate_diagnostic(diagnostic) {
            violations.push(DiagnosticViolation {
                diagnostic_index,
                rule_id: rule_id.clone(),
                primary_span: primary_span.clone(),
                kind,
            });
        }
    }

    violations
}

fn span_is_valid(span: &Span) -> bool {
    span.start <= span.end
}

fn ranges_overlap(a_start: u32, a_end: u32, b_start: u32, b_end: u32) -> bool {
    let a_zero = a_start == a_end;
    let b_zero = b_start == b_end;

    match (a_zero, b_zero) {
        (true, true) => a_start == b_start,
        (true, false) => b_start <= a_start && a_start < b_end,
        (false, true) => a_start <= b_start && b_start < a_end,
        (false, false) => a_start < b_end && b_start < a_end,
    }
}

#[cfg(test)]
fn validate_span_overlap(a_start: u32, a_end: u32, b_start: u32, b_end: u32) -> bool {
    ranges_overlap(a_start, a_end, b_start, b_end)
}

#[cfg(test)]
mod tests {
    use super::{
        DiagnosticViolationKind, validate_diagnostic, validate_diagnostics, validate_span_overlap,
    };
    use crate::diagnostics::{
        Applicability, Confidence, Diagnostic, Fix, FixSafety, Severity, StructuredMessage,
        StructuredSuggestion, SuggestionGroup, TextEdit,
    };
    use crate::model::Span;

    fn valid_diagnostic() -> Diagnostic {
        Diagnostic {
            rule_id: "NOIR100".to_string(),
            severity: Severity::Warning,
            confidence: Confidence::High,
            policy: "maintainability".to_string(),
            message: "magic number".to_string(),
            primary_span: Span::new("src/main.nr", 10, 12, 2, 9),
            secondary_spans: vec![Span::new("src/main.nr", 0, 5, 1, 1)],
            suggestions: vec!["extract constant".to_string()],
            notes: vec![StructuredMessage {
                message: "context".to_string(),
                span: Some(Span::new("src/main.nr", 0, 5, 1, 1)),
            }],
            helps: vec![StructuredMessage {
                message: "use named constants".to_string(),
                span: Some(Span::new("src/main.nr", 10, 12, 2, 9)),
            }],
            structured_suggestions: vec![StructuredSuggestion {
                message: "replace with named constant".to_string(),
                span: Span::new("src/main.nr", 10, 12, 2, 9),
                replacement: "MAX_FEE".to_string(),
                applicability: Applicability::MaybeIncorrect,
            }],
            suggestion_groups: Vec::new(),
            fixes: vec![Fix {
                description: "replace with named constant".to_string(),
                span: Span::new("src/main.nr", 10, 12, 2, 9),
                replacement: "MAX_FEE".to_string(),
                safety: FixSafety::NeedsReview,
            }],
            suppressed: false,
            suppression_reason: None,
        }
    }

    #[test]
    fn valid_diagnostic_passes_validation() {
        let diagnostic = valid_diagnostic();
        assert!(validate_diagnostic(&diagnostic).is_empty());
        assert!(validate_diagnostics(&[diagnostic]).is_empty());
    }

    #[test]
    fn invalid_diagnostic_reports_all_expected_violations_in_order() {
        let mut diagnostic = valid_diagnostic();
        diagnostic.rule_id.clear();
        diagnostic.policy.clear();
        diagnostic.message.clear();
        diagnostic.primary_span = Span::new("src/main.nr", 20, 10, 2, 1);
        diagnostic.secondary_spans = vec![Span::new("src/main.nr", 30, 10, 3, 1)];
        diagnostic.notes = vec![StructuredMessage {
            message: "bad note".to_string(),
            span: Some(Span::new("src/main.nr", 15, 12, 3, 1)),
        }];
        diagnostic.helps = vec![StructuredMessage {
            message: "bad help".to_string(),
            span: Some(Span::new("src/main.nr", 99, 40, 8, 1)),
        }];
        diagnostic.structured_suggestions = vec![StructuredSuggestion {
            message: "bad suggestion".to_string(),
            span: Span::new("src/main.nr", 50, 45, 4, 1),
            replacement: "x".to_string(),
            applicability: Applicability::MaybeIncorrect,
        }];
        diagnostic.fixes = vec![Fix {
            description: "bad fix".to_string(),
            span: Span::new("src/main.nr", 9, 2, 1, 1),
            replacement: "x".to_string(),
            safety: FixSafety::Safe,
        }];
        diagnostic.suppressed = true;
        diagnostic.suppression_reason = None;

        assert_eq!(
            validate_diagnostic(&diagnostic),
            vec![
                DiagnosticViolationKind::EmptyRuleId,
                DiagnosticViolationKind::EmptyPolicy,
                DiagnosticViolationKind::EmptyMessage,
                DiagnosticViolationKind::InvalidPrimarySpan { start: 20, end: 10 },
                DiagnosticViolationKind::InvalidSecondarySpan {
                    index: 0,
                    start: 30,
                    end: 10
                },
                DiagnosticViolationKind::InvalidNoteSpan {
                    index: 0,
                    start: 15,
                    end: 12
                },
                DiagnosticViolationKind::InvalidHelpSpan {
                    index: 0,
                    start: 99,
                    end: 40
                },
                DiagnosticViolationKind::InvalidStructuredSuggestionSpan {
                    index: 0,
                    start: 50,
                    end: 45
                },
                DiagnosticViolationKind::InvalidFixSpan {
                    index: 0,
                    start: 9,
                    end: 2
                },
                DiagnosticViolationKind::MissingSuppressionReason,
            ]
        );
    }

    #[test]
    fn overlapping_implicit_structured_suggestion_group_is_rejected() {
        let mut diagnostic = valid_diagnostic();
        diagnostic.structured_suggestions = vec![
            StructuredSuggestion {
                message: "replace pair".to_string(),
                span: Span::new("src/main.nr", 10, 15, 1, 1),
                replacement: "x".to_string(),
                applicability: Applicability::MachineApplicable,
            },
            StructuredSuggestion {
                message: "replace pair".to_string(),
                span: Span::new("src/main.nr", 14, 20, 1, 1),
                replacement: "y".to_string(),
                applicability: Applicability::MachineApplicable,
            },
        ];

        let violations = validate_diagnostic(&diagnostic);
        assert_eq!(violations.len(), 1);
        match &violations[0] {
            DiagnosticViolationKind::OverlappingStructuredSuggestionGroupSpans {
                message,
                applicability,
                first_index,
                second_index,
                ..
            } => {
                assert_eq!(message, "replace pair");
                assert_eq!(*applicability, Applicability::MachineApplicable);
                assert_eq!(*first_index, 0);
                assert_eq!(*second_index, 1);
            }
            other => panic!("unexpected violation kind: {other:?}"),
        }
    }

    #[test]
    fn overlapping_explicit_suggestion_group_edits_are_rejected() {
        let mut diagnostic = valid_diagnostic();
        diagnostic.structured_suggestions.clear();
        diagnostic.suggestion_groups = vec![SuggestionGroup {
            id: "sg0001".to_string(),
            message: "replace pair".to_string(),
            applicability: Applicability::MachineApplicable,
            edits: vec![
                TextEdit {
                    span: Span::new("src/main.nr", 10, 15, 1, 1),
                    replacement: "x".to_string(),
                },
                TextEdit {
                    span: Span::new("src/main.nr", 14, 20, 1, 1),
                    replacement: "y".to_string(),
                },
            ],
            provenance: None,
        }];

        let violations = validate_diagnostic(&diagnostic);
        assert_eq!(violations.len(), 1);
        match &violations[0] {
            DiagnosticViolationKind::OverlappingSuggestionGroupEdits {
                group_id,
                first_edit_index,
                second_edit_index,
                ..
            } => {
                assert_eq!(group_id, "sg0001");
                assert_eq!(*first_edit_index, 0);
                assert_eq!(*second_edit_index, 1);
            }
            other => panic!("unexpected violation kind: {other:?}"),
        }
    }

    #[test]
    fn validate_diagnostics_attaches_diagnostic_context() {
        let valid = valid_diagnostic();
        let mut invalid = valid_diagnostic();
        invalid.rule_id.clear();

        let violations = validate_diagnostics(&[valid, invalid]);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].diagnostic_index, 1);
        assert_eq!(violations[0].rule_id, "");
        assert_eq!(violations[0].kind, DiagnosticViolationKind::EmptyRuleId);
    }

    #[test]
    fn overlap_logic_matches_fix_conflict_contract() {
        assert!(validate_span_overlap(5, 5, 5, 5));
        assert!(validate_span_overlap(5, 5, 5, 8));
        assert!(!validate_span_overlap(5, 5, 8, 10));
        assert!(validate_span_overlap(5, 8, 6, 9));
        assert!(!validate_span_overlap(5, 8, 8, 12));
    }
}
