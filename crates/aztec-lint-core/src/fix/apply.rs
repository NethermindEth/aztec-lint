use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Confidence, Diagnostic, FixSafety, normalize_file_path};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixApplicationMode {
    Apply,
    DryRun,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum FixSource {
    ExplicitFix,
    StructuredSuggestion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixApplicationResult {
    pub rule_id: String,
    pub source: FixSource,
    pub group_id: String,
    pub provenance: Option<String>,
    pub file: String,
    pub start: u32,
    pub end: u32,
    pub edit_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkippedFixReason {
    SuppressedDiagnostic,
    UnsafeFix,
    MixedFileGroup,
    GroupOverlap,
    InvalidGroupSpan,
    GroupNoop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkippedFix {
    pub rule_id: String,
    pub source: FixSource,
    pub group_id: String,
    pub provenance: Option<String>,
    pub file: String,
    pub start: u32,
    pub end: u32,
    pub edit_count: usize,
    pub reason: SkippedFixReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixApplicationReport {
    pub mode: FixApplicationMode,
    pub total_candidates: usize,
    pub selected: Vec<FixApplicationResult>,
    pub skipped: Vec<SkippedFix>,
    pub files_changed: usize,
}

#[derive(Debug)]
pub enum FixError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl Display for FixError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    f,
                    "failed to process fix file '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl Error for FixError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GroupEdit {
    file: String,
    start: usize,
    end: usize,
    replacement: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FixGroupCandidate {
    ordinal: usize,
    rule_id: String,
    source: FixSource,
    confidence: Confidence,
    group_id: String,
    provenance: Option<String>,
    file: String,
    edits: Vec<GroupEdit>,
}

#[derive(Clone, Debug)]
struct PendingGroupEdit {
    span: crate::model::Span,
    replacement: String,
}

#[derive(Clone, Debug)]
struct PendingFixGroup {
    source: FixSource,
    group_id: String,
    provenance: Option<String>,
    edits: Vec<PendingGroupEdit>,
    safety: FixSafety,
}

#[derive(Clone, Debug)]
struct PendingGroupSummary {
    rule_id: String,
    source: FixSource,
    group_id: String,
    provenance: Option<String>,
    file: String,
    start: u32,
    end: u32,
    edit_count: usize,
}

impl PendingGroupSummary {
    fn to_skipped(&self, reason: SkippedFixReason) -> SkippedFix {
        SkippedFix {
            rule_id: self.rule_id.clone(),
            source: self.source,
            group_id: self.group_id.clone(),
            provenance: self.provenance.clone(),
            file: self.file.clone(),
            start: self.start,
            end: self.end,
            edit_count: self.edit_count,
            reason,
        }
    }
}

impl FixGroupCandidate {
    fn bounds(&self) -> (usize, usize) {
        let start = self.edits.iter().map(|edit| edit.start).min().unwrap_or(0);
        let end = self.edits.iter().map(|edit| edit.end).max().unwrap_or(0);
        (start, end)
    }

    fn to_skipped(&self, reason: SkippedFixReason) -> SkippedFix {
        let (start, end) = self.bounds();
        SkippedFix {
            rule_id: self.rule_id.clone(),
            source: self.source,
            group_id: self.group_id.clone(),
            provenance: self.provenance.clone(),
            file: self.file.clone(),
            start: u32::try_from(start).unwrap_or(u32::MAX),
            end: u32::try_from(end).unwrap_or(u32::MAX),
            edit_count: self.edits.len(),
            reason,
        }
    }

    fn to_selected(&self) -> FixApplicationResult {
        let (start, end) = self.bounds();
        FixApplicationResult {
            rule_id: self.rule_id.clone(),
            source: self.source,
            group_id: self.group_id.clone(),
            provenance: self.provenance.clone(),
            file: self.file.clone(),
            start: u32::try_from(start).unwrap_or(u32::MAX),
            end: u32::try_from(end).unwrap_or(u32::MAX),
            edit_count: self.edits.len(),
        }
    }
}

fn suggestion_covered_by_groups(
    suggestion: &crate::diagnostics::StructuredSuggestion,
    groups: &[crate::diagnostics::SuggestionGroup],
) -> bool {
    groups.iter().any(|group| {
        group.message == suggestion.message
            && group.applicability == suggestion.applicability
            && group.edits.iter().any(|edit| {
                edit.span == suggestion.span && edit.replacement == suggestion.replacement
            })
    })
}

fn pending_fix_groups(diagnostic: &Diagnostic) -> Vec<PendingFixGroup> {
    let mut pending = diagnostic
        .fixes
        .iter()
        .enumerate()
        .map(|(index, fix)| PendingFixGroup {
            source: FixSource::ExplicitFix,
            group_id: format!("legacy_fix_{:04}", index + 1),
            provenance: None,
            edits: vec![PendingGroupEdit {
                span: fix.span.clone(),
                replacement: fix.replacement.clone(),
            }],
            safety: fix.safety,
        })
        .collect::<Vec<_>>();

    if !diagnostic.suggestion_groups.is_empty() {
        pending.extend(
            diagnostic
                .suggestion_groups
                .iter()
                .map(|group| PendingFixGroup {
                    source: FixSource::StructuredSuggestion,
                    group_id: group.id.clone(),
                    provenance: group.provenance.clone(),
                    edits: group
                        .edits
                        .iter()
                        .map(|edit| PendingGroupEdit {
                            span: edit.span.clone(),
                            replacement: edit.replacement.clone(),
                        })
                        .collect(),
                    safety: group.applicability.to_fix_safety(),
                }),
        );

        pending.extend(
            diagnostic
                .structured_suggestions
                .iter()
                .enumerate()
                .filter(|(_, suggestion)| {
                    !suggestion_covered_by_groups(suggestion, &diagnostic.suggestion_groups)
                })
                .map(|(index, suggestion)| PendingFixGroup {
                    source: FixSource::StructuredSuggestion,
                    group_id: format!("legacy_structured_{:04}", index + 1),
                    provenance: None,
                    edits: vec![PendingGroupEdit {
                        span: suggestion.span.clone(),
                        replacement: suggestion.replacement.clone(),
                    }],
                    safety: suggestion.applicability.to_fix_safety(),
                }),
        );
    } else {
        pending.extend(
            diagnostic
                .structured_suggestions
                .iter()
                .enumerate()
                .map(|(index, suggestion)| PendingFixGroup {
                    source: FixSource::StructuredSuggestion,
                    group_id: format!("legacy_structured_{:04}", index + 1),
                    provenance: None,
                    edits: vec![PendingGroupEdit {
                        span: suggestion.span.clone(),
                        replacement: suggestion.replacement.clone(),
                    }],
                    safety: suggestion.applicability.to_fix_safety(),
                }),
        );
    }

    pending
}

fn pending_group_summary(rule_id: &str, group: &PendingFixGroup) -> PendingGroupSummary {
    let file = group
        .edits
        .first()
        .map(|edit| normalize_file_path(&edit.span.file))
        .unwrap_or_default();
    let start = group
        .edits
        .iter()
        .map(|edit| edit.span.start)
        .min()
        .unwrap_or(0);
    let end = group
        .edits
        .iter()
        .map(|edit| edit.span.end)
        .max()
        .unwrap_or(0);

    PendingGroupSummary {
        rule_id: rule_id.to_string(),
        source: group.source,
        group_id: group.group_id.clone(),
        provenance: group.provenance.clone(),
        file,
        start,
        end,
        edit_count: group.edits.len(),
    }
}

fn normalize_group_candidate(
    rule_id: &str,
    confidence: Confidence,
    ordinal: usize,
    group: PendingFixGroup,
) -> Result<FixGroupCandidate, SkippedFixReason> {
    if group.edits.is_empty() {
        return Err(SkippedFixReason::GroupNoop);
    }

    let file = normalize_file_path(&group.edits[0].span.file);
    let edits = group
        .edits
        .into_iter()
        .map(|edit| GroupEdit {
            file: normalize_file_path(&edit.span.file),
            start: usize::try_from(edit.span.start).unwrap_or(usize::MAX),
            end: usize::try_from(edit.span.end).unwrap_or(usize::MAX),
            replacement: edit.replacement,
        })
        .collect::<Vec<_>>();

    if edits.iter().any(|edit| edit.file != file) {
        return Err(SkippedFixReason::MixedFileGroup);
    }

    for i in 0..edits.len() {
        for j in (i + 1)..edits.len() {
            if ranges_overlap(edits[i].start, edits[i].end, edits[j].start, edits[j].end) {
                return Err(SkippedFixReason::GroupOverlap);
            }
        }
    }

    Ok(FixGroupCandidate {
        ordinal,
        rule_id: rule_id.to_string(),
        source: group.source,
        confidence,
        group_id: group.group_id,
        provenance: group.provenance,
        file,
        edits,
    })
}

pub fn apply_fixes(
    root: &Path,
    diagnostics: &[Diagnostic],
    mode: FixApplicationMode,
) -> Result<FixApplicationReport, FixError> {
    let mut report = FixApplicationReport {
        mode,
        total_candidates: 0,
        selected: Vec::new(),
        skipped: Vec::new(),
        files_changed: 0,
    };

    let mut candidates_by_file = BTreeMap::<String, Vec<FixGroupCandidate>>::new();
    let mut ordinal = 0usize;

    for diagnostic in diagnostics {
        for pending_group in pending_fix_groups(diagnostic) {
            report.total_candidates += 1;
            ordinal += 1;

            let summary = pending_group_summary(&diagnostic.rule_id, &pending_group);

            if diagnostic.suppressed {
                report
                    .skipped
                    .push(summary.to_skipped(SkippedFixReason::SuppressedDiagnostic));
                continue;
            }

            if pending_group.safety != FixSafety::Safe {
                report
                    .skipped
                    .push(summary.to_skipped(SkippedFixReason::UnsafeFix));
                continue;
            }

            match normalize_group_candidate(
                &diagnostic.rule_id,
                diagnostic.confidence,
                ordinal,
                pending_group,
            ) {
                Ok(candidate) => {
                    candidates_by_file
                        .entry(candidate.file.clone())
                        .or_default()
                        .push(candidate);
                }
                Err(reason) => {
                    report.skipped.push(summary.to_skipped(reason));
                }
            }
        }
    }

    for (file, mut candidates) in candidates_by_file {
        let winners = resolve_group_overlaps(&mut candidates, &mut report.skipped);
        if winners.is_empty() {
            continue;
        }

        let path = resolve_path(root, &file);
        let mut content = fs::read_to_string(&path).map_err(|source| FixError::Io {
            path: path.clone(),
            source,
        })?;
        let mut changed = false;

        let mut edit_order = winners
            .iter()
            .enumerate()
            .flat_map(|(group_index, candidate)| {
                candidate
                    .edits
                    .iter()
                    .enumerate()
                    .map(move |(edit_index, edit)| {
                        (
                            group_index,
                            edit_index,
                            edit.start,
                            edit.end,
                            candidate.ordinal,
                        )
                    })
            })
            .collect::<Vec<_>>();

        edit_order.sort_by_key(|(_, _, start, end, ordinal)| {
            (std::cmp::Reverse(*start), std::cmp::Reverse(*end), *ordinal)
        });

        let mut group_state = vec![GroupApplyState::Unknown; winners.len()];

        for (group_index, edit_index, _, _, _) in edit_order {
            match group_state[group_index] {
                GroupApplyState::Rejected => continue,
                GroupApplyState::Unknown => {
                    let candidate = &winners[group_index];
                    if !group_spans_valid_for_content(candidate, &content) {
                        report
                            .skipped
                            .push(candidate.to_skipped(SkippedFixReason::InvalidGroupSpan));
                        group_state[group_index] = GroupApplyState::Rejected;
                        continue;
                    }

                    let all_noop = candidate
                        .edits
                        .iter()
                        .all(|edit| content[edit.start..edit.end] == edit.replacement);
                    if all_noop {
                        report
                            .skipped
                            .push(candidate.to_skipped(SkippedFixReason::GroupNoop));
                        group_state[group_index] = GroupApplyState::Rejected;
                        continue;
                    }

                    report.selected.push(candidate.to_selected());
                    group_state[group_index] = GroupApplyState::Accepted;
                }
                GroupApplyState::Accepted => {}
            }

            if group_state[group_index] != GroupApplyState::Accepted {
                continue;
            }

            let edit = &winners[group_index].edits[edit_index];
            if content[edit.start..edit.end] != edit.replacement {
                content.replace_range(edit.start..edit.end, &edit.replacement);
                changed = true;
            }
        }

        if changed {
            report.files_changed += 1;
            if mode == FixApplicationMode::Apply {
                fs::write(&path, content).map_err(|source| FixError::Io {
                    path: path.clone(),
                    source,
                })?;
            }
        }
    }

    Ok(report)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GroupApplyState {
    Unknown,
    Accepted,
    Rejected,
}

fn resolve_group_overlaps(
    candidates: &mut [FixGroupCandidate],
    skipped: &mut Vec<SkippedFix>,
) -> Vec<FixGroupCandidate> {
    candidates.sort_by(|left, right| {
        (
            left.bounds(),
            left.rule_id.as_str(),
            left.source,
            left.ordinal,
            left.group_id.as_str(),
        )
            .cmp(&(
                right.bounds(),
                right.rule_id.as_str(),
                right.source,
                right.ordinal,
                right.group_id.as_str(),
            ))
    });

    let mut winners = Vec::<FixGroupCandidate>::new();
    for candidate in candidates.iter().cloned() {
        let overlapping = winners
            .iter()
            .enumerate()
            .filter_map(|(idx, existing)| {
                group_candidates_overlap(&candidate, existing).then_some(idx)
            })
            .collect::<Vec<_>>();

        if overlapping.is_empty() {
            winners.push(candidate);
            continue;
        }

        let candidate_wins = overlapping
            .iter()
            .all(|idx| outranks_group(&candidate, &winners[*idx]));

        if !candidate_wins {
            skipped.push(candidate.to_skipped(SkippedFixReason::GroupOverlap));
            continue;
        }

        for idx in overlapping.into_iter().rev() {
            let loser = winners.remove(idx);
            skipped.push(loser.to_skipped(SkippedFixReason::GroupOverlap));
        }

        winners.push(candidate);
    }

    winners
}

fn group_candidates_overlap(left: &FixGroupCandidate, right: &FixGroupCandidate) -> bool {
    left.edits.iter().any(|l| {
        right
            .edits
            .iter()
            .any(|r| ranges_overlap(l.start, l.end, r.start, r.end))
    })
}

fn group_spans_valid_for_content(candidate: &FixGroupCandidate, content: &str) -> bool {
    candidate
        .edits
        .iter()
        .all(|edit| valid_span(content, edit.start, edit.end))
}

fn valid_span(content: &str, start: usize, end: usize) -> bool {
    start <= end
        && end <= content.len()
        && content.is_char_boundary(start)
        && content.is_char_boundary(end)
}

fn resolve_path(root: &Path, file: &str) -> PathBuf {
    let file_path = Path::new(file);
    if file_path.is_absolute() {
        return file_path.to_path_buf();
    }
    root.join(file_path)
}

fn ranges_overlap(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    let a_zero = a_start == a_end;
    let b_zero = b_start == b_end;

    match (a_zero, b_zero) {
        (true, true) => a_start == b_start,
        // Treat insertion at the start of a replacement as conflicting.
        // Applying both edits at identical start offsets depends on execution order.
        (true, false) => b_start <= a_start && a_start < b_end,
        (false, true) => a_start <= b_start && b_start < a_end,
        (false, false) => a_start < b_end && b_start < a_end,
    }
}

fn outranks_group(candidate: &FixGroupCandidate, incumbent: &FixGroupCandidate) -> bool {
    match confidence_rank(candidate.confidence).cmp(&confidence_rank(incumbent.confidence)) {
        Ordering::Greater => return true,
        Ordering::Less => return false,
        Ordering::Equal => {}
    }

    match candidate.rule_id.cmp(&incumbent.rule_id) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => {
            match source_rank(candidate.source).cmp(&source_rank(incumbent.source)) {
                Ordering::Greater => true,
                Ordering::Less => false,
                Ordering::Equal => candidate.ordinal < incumbent.ordinal,
            }
        }
    }
}

fn confidence_rank(confidence: Confidence) -> u8 {
    match confidence {
        Confidence::Low => 1,
        Confidence::Medium => 2,
        Confidence::High => 3,
    }
}

fn source_rank(source: FixSource) -> u8 {
    match source {
        FixSource::ExplicitFix => 2,
        FixSource::StructuredSuggestion => 1,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{FixApplicationMode, FixSource, SkippedFixReason, apply_fixes};
    use crate::diagnostics::{
        Applicability, Confidence, Diagnostic, Fix, FixSafety, Severity, StructuredSuggestion,
        SuggestionGroup, TextEdit,
    };
    use crate::model::Span;

    fn diagnostic_with_fix(
        rule_id: &str,
        confidence: Confidence,
        file: &str,
        start: u32,
        end: u32,
        replacement: &str,
    ) -> Diagnostic {
        Diagnostic {
            rule_id: rule_id.to_string(),
            severity: Severity::Warning,
            confidence,
            policy: "maintainability".to_string(),
            message: "message".to_string(),
            primary_span: Span::new(file, start, end, 1, 1),
            secondary_spans: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
            structured_suggestions: Vec::new(),
            suggestion_groups: Vec::new(),
            fixes: vec![Fix {
                description: "replace span".to_string(),
                span: Span::new(file, start, end, 1, 1),
                replacement: replacement.to_string(),
                safety: FixSafety::Safe,
            }],
            suppressed: false,
            suppression_reason: None,
        }
    }

    fn diagnostic_with_structured_suggestion(
        rule_id: &str,
        confidence: Confidence,
        file: &str,
        start: u32,
        end: u32,
        replacement: &str,
        applicability: Applicability,
    ) -> Diagnostic {
        Diagnostic {
            rule_id: rule_id.to_string(),
            severity: Severity::Warning,
            confidence,
            policy: "maintainability".to_string(),
            message: "message".to_string(),
            primary_span: Span::new(file, start, end, 1, 1),
            secondary_spans: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
            structured_suggestions: vec![StructuredSuggestion {
                message: "replace span".to_string(),
                span: Span::new(file, start, end, 1, 1),
                replacement: replacement.to_string(),
                applicability,
            }],
            suggestion_groups: Vec::new(),
            fixes: Vec::new(),
            suppressed: false,
            suppression_reason: None,
        }
    }

    fn diagnostic_with_grouped_suggestion(
        rule_id: &str,
        confidence: Confidence,
        file: &str,
        edits: Vec<(u32, u32, &str)>,
        applicability: Applicability,
    ) -> Diagnostic {
        Diagnostic {
            rule_id: rule_id.to_string(),
            severity: Severity::Warning,
            confidence,
            policy: "maintainability".to_string(),
            message: "message".to_string(),
            primary_span: Span::new(file, 0, 1, 1, 1),
            secondary_spans: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
            structured_suggestions: Vec::new(),
            suggestion_groups: vec![SuggestionGroup {
                id: "sg0001".to_string(),
                message: "grouped".to_string(),
                applicability,
                edits: edits
                    .into_iter()
                    .map(|(start, end, replacement)| TextEdit {
                        span: Span::new(file, start, end, 1, 1),
                        replacement: replacement.to_string(),
                    })
                    .collect(),
                provenance: Some("rule-emitter".to_string()),
            }],
            fixes: Vec::new(),
            suppressed: false,
            suppression_reason: None,
        }
    }

    #[test]
    fn dry_run_reports_fix_without_writing_file() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "let x = 1;\n").expect("fixture should be written");

        let diagnostics = vec![diagnostic_with_fix(
            "NOIR100",
            Confidence::Medium,
            "src/main.nr",
            8,
            9,
            "2",
        )];
        let report = apply_fixes(dir.path(), &diagnostics, FixApplicationMode::DryRun)
            .expect("dry run should succeed");

        assert_eq!(report.selected.len(), 1);
        assert_eq!(report.selected[0].edit_count, 1);
        assert_eq!(report.files_changed, 1);
        let after = fs::read_to_string(&source_path).expect("file should still exist");
        assert_eq!(after, "let x = 1;\n");
    }

    #[test]
    fn apply_is_idempotent_for_already_applied_edit() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "let x = 1;\n").expect("fixture should be written");

        let diagnostics = vec![diagnostic_with_fix(
            "NOIR100",
            Confidence::Medium,
            "src/main.nr",
            8,
            9,
            "2",
        )];

        let first = apply_fixes(dir.path(), &diagnostics, FixApplicationMode::Apply)
            .expect("first apply should succeed");
        assert_eq!(first.selected.len(), 1);
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "let x = 2;\n"
        );

        let second = apply_fixes(dir.path(), &diagnostics, FixApplicationMode::Apply)
            .expect("second apply should succeed");
        assert!(second.selected.is_empty());
        assert!(
            second
                .skipped
                .iter()
                .any(|skip| skip.reason == SkippedFixReason::GroupNoop)
        );
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "let x = 2;\n"
        );
    }

    #[test]
    fn overlap_prefers_higher_confidence_candidate() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "abcdef\n").expect("fixture should be written");

        let high = diagnostic_with_fix("NOIR001", Confidence::High, "src/main.nr", 1, 4, "HIGH");
        let low = diagnostic_with_fix("NOIR200", Confidence::Low, "src/main.nr", 2, 5, "LOW");
        let report = apply_fixes(dir.path(), &[low, high], FixApplicationMode::Apply)
            .expect("apply should succeed");

        assert_eq!(report.selected.len(), 1);
        assert!(report.skipped.iter().any(|skip| {
            skip.rule_id == "NOIR200" && skip.reason == SkippedFixReason::GroupOverlap
        }));
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "aHIGHef\n"
        );
    }

    #[test]
    fn overlap_tie_prefers_lexically_lower_rule_id() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "abcdef\n").expect("fixture should be written");

        let winner = diagnostic_with_fix("NOIR001", Confidence::Medium, "src/main.nr", 1, 4, "A");
        let loser = diagnostic_with_fix("NOIR200", Confidence::Medium, "src/main.nr", 1, 4, "B");
        let report = apply_fixes(dir.path(), &[loser, winner], FixApplicationMode::Apply)
            .expect("apply should succeed");

        assert_eq!(report.selected.len(), 1);
        assert!(report.skipped.iter().any(|skip| {
            skip.rule_id == "NOIR200" && skip.reason == SkippedFixReason::GroupOverlap
        }));
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "aAef\n"
        );
    }

    #[test]
    fn invalid_span_is_skipped() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "abc\n").expect("fixture should be written");

        let diagnostics = vec![diagnostic_with_fix(
            "NOIR100",
            Confidence::Medium,
            "src/main.nr",
            99,
            120,
            "x",
        )];
        let report = apply_fixes(dir.path(), &diagnostics, FixApplicationMode::Apply)
            .expect("apply should succeed");

        assert!(report.selected.is_empty());
        assert!(
            report
                .skipped
                .iter()
                .any(|skip| skip.reason == SkippedFixReason::InvalidGroupSpan)
        );
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "abc\n"
        );
    }

    #[test]
    fn insertion_at_same_start_as_replacement_is_treated_as_overlap() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "abc\n").expect("fixture should be written");

        let replacement =
            diagnostic_with_fix("NOIR001", Confidence::High, "src/main.nr", 1, 2, "X");
        let insertion = diagnostic_with_fix("NOIR200", Confidence::Low, "src/main.nr", 1, 1, "Y");

        let report = apply_fixes(
            dir.path(),
            &[insertion, replacement],
            FixApplicationMode::Apply,
        )
        .expect("apply should succeed");

        assert_eq!(report.selected.len(), 1);
        assert!(report.skipped.iter().any(|skip| {
            skip.rule_id == "NOIR200" && skip.reason == SkippedFixReason::GroupOverlap
        }));
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "aXc\n"
        );
    }

    #[test]
    fn machine_applicable_structured_suggestion_is_applied() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "let x = 1;\n").expect("fixture should be written");

        let diagnostics = vec![diagnostic_with_structured_suggestion(
            "NOIR100",
            Confidence::Medium,
            "src/main.nr",
            8,
            9,
            "2",
            Applicability::MachineApplicable,
        )];

        let report = apply_fixes(dir.path(), &diagnostics, FixApplicationMode::Apply)
            .expect("apply should succeed");
        assert_eq!(report.total_candidates, 1);
        assert_eq!(report.selected.len(), 1);
        assert_eq!(report.selected[0].source, FixSource::StructuredSuggestion);
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "let x = 2;\n"
        );
    }

    #[test]
    fn non_machine_structured_suggestion_is_reported_as_unsafe() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "let x = 1;\n").expect("fixture should be written");

        let diagnostics = vec![diagnostic_with_structured_suggestion(
            "NOIR100",
            Confidence::Medium,
            "src/main.nr",
            8,
            9,
            "2",
            Applicability::MaybeIncorrect,
        )];

        let report = apply_fixes(dir.path(), &diagnostics, FixApplicationMode::Apply)
            .expect("apply should succeed");
        assert_eq!(report.total_candidates, 1);
        assert!(report.selected.is_empty());
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].source, FixSource::StructuredSuggestion);
        assert_eq!(report.skipped[0].reason, SkippedFixReason::UnsafeFix);
        assert_eq!(report.skipped[0].edit_count, 1);
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "let x = 1;\n"
        );
    }

    #[test]
    fn non_machine_grouped_suggestion_is_reported_as_unsafe() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "abcXYZ\n").expect("fixture should be written");

        let diagnostics = vec![diagnostic_with_grouped_suggestion(
            "NOIR500",
            Confidence::High,
            "src/main.nr",
            vec![(0, 1, "A"), (3, 6, "123")],
            Applicability::MaybeIncorrect,
        )];

        let report = apply_fixes(dir.path(), &diagnostics, FixApplicationMode::Apply)
            .expect("apply should succeed");

        assert_eq!(report.total_candidates, 1);
        assert!(report.selected.is_empty());
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].reason, SkippedFixReason::UnsafeFix);
        assert_eq!(report.skipped[0].edit_count, 2);
        assert_eq!(report.skipped[0].group_id, "sg0001");
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "abcXYZ\n"
        );
    }

    #[test]
    fn overlap_reports_source_provenance() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "let x = 1;\n").expect("fixture should be written");

        let explicit = diagnostic_with_fix("NOIR100", Confidence::Medium, "src/main.nr", 8, 9, "2");
        let structured = diagnostic_with_structured_suggestion(
            "NOIR100",
            Confidence::Medium,
            "src/main.nr",
            8,
            9,
            "3",
            Applicability::MachineApplicable,
        );

        let report = apply_fixes(
            dir.path(),
            &[structured, explicit],
            FixApplicationMode::Apply,
        )
        .expect("apply should succeed");

        assert_eq!(report.total_candidates, 2);
        assert_eq!(report.selected.len(), 1);
        assert_eq!(report.selected[0].source, FixSource::ExplicitFix);
        assert!(report.skipped.iter().any(|skip| {
            skip.source == FixSource::StructuredSuggestion
                && skip.reason == SkippedFixReason::GroupOverlap
        }));
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "let x = 2;\n"
        );
    }

    #[test]
    fn grouped_fix_is_applied_atomically() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "abcXYZ\n").expect("fixture should be written");

        let diagnostics = vec![diagnostic_with_grouped_suggestion(
            "NOIR500",
            Confidence::High,
            "src/main.nr",
            vec![(0, 1, "A"), (3, 6, "123")],
            Applicability::MachineApplicable,
        )];

        let report = apply_fixes(dir.path(), &diagnostics, FixApplicationMode::Apply)
            .expect("apply should succeed");

        assert_eq!(report.total_candidates, 1);
        assert_eq!(report.selected.len(), 1);
        assert_eq!(report.selected[0].edit_count, 2);
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "Abc123\n"
        );
    }

    #[test]
    fn grouped_fix_atomic_rollback_when_one_edit_is_invalid() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "abcdef\n").expect("fixture should be written");

        let diagnostics = vec![diagnostic_with_grouped_suggestion(
            "NOIR500",
            Confidence::High,
            "src/main.nr",
            vec![(1, 2, "X"), (99, 100, "Y")],
            Applicability::MachineApplicable,
        )];

        let report = apply_fixes(dir.path(), &diagnostics, FixApplicationMode::Apply)
            .expect("apply should succeed");

        assert!(report.selected.is_empty());
        assert!(
            report
                .skipped
                .iter()
                .any(|skip| skip.reason == SkippedFixReason::InvalidGroupSpan)
        );
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "abcdef\n"
        );
    }

    #[test]
    fn same_group_overlap_is_rejected() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "abcdef\n").expect("fixture should be written");

        let diagnostics = vec![diagnostic_with_grouped_suggestion(
            "NOIR500",
            Confidence::High,
            "src/main.nr",
            vec![(1, 4, "X"), (3, 5, "Y")],
            Applicability::MachineApplicable,
        )];

        let report = apply_fixes(dir.path(), &diagnostics, FixApplicationMode::Apply)
            .expect("apply should succeed");

        assert!(report.selected.is_empty());
        assert!(
            report
                .skipped
                .iter()
                .any(|skip| skip.reason == SkippedFixReason::GroupOverlap)
        );
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "abcdef\n"
        );
    }

    #[test]
    fn overlapping_groups_choose_deterministic_winner() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "abcdef\n").expect("fixture should be written");

        let high = diagnostic_with_grouped_suggestion(
            "NOIR100",
            Confidence::High,
            "src/main.nr",
            vec![(1, 3, "AA")],
            Applicability::MachineApplicable,
        );
        let low = diagnostic_with_grouped_suggestion(
            "NOIR200",
            Confidence::Low,
            "src/main.nr",
            vec![(2, 4, "BB")],
            Applicability::MachineApplicable,
        );

        let report = apply_fixes(dir.path(), &[low, high], FixApplicationMode::Apply)
            .expect("apply should succeed");

        assert_eq!(report.selected.len(), 1);
        assert_eq!(report.selected[0].rule_id, "NOIR100");
        assert!(
            report
                .skipped
                .iter()
                .any(|skip| skip.rule_id == "NOIR200"
                    && skip.reason == SkippedFixReason::GroupOverlap)
        );
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "aAAdef\n"
        );
    }

    #[test]
    fn grouped_fix_is_idempotent_across_runs() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "abcXYZ\n").expect("fixture should be written");

        let diagnostics = vec![diagnostic_with_grouped_suggestion(
            "NOIR500",
            Confidence::High,
            "src/main.nr",
            vec![(0, 1, "A"), (3, 6, "123")],
            Applicability::MachineApplicable,
        )];

        let first = apply_fixes(dir.path(), &diagnostics, FixApplicationMode::Apply)
            .expect("first apply should succeed");
        assert_eq!(first.selected.len(), 1);

        let second = apply_fixes(dir.path(), &diagnostics, FixApplicationMode::Apply)
            .expect("second apply should succeed");
        assert!(second.selected.is_empty());
        assert!(
            second
                .skipped
                .iter()
                .any(|skip| skip.reason == SkippedFixReason::GroupNoop)
        );
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "Abc123\n"
        );
    }

    #[test]
    fn grouped_fix_rejects_mixed_file_edits() {
        let dir = tempdir().expect("tempdir should be created");
        let source_path = dir.path().join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, "abc\n").expect("fixture should be written");

        let diagnostic = Diagnostic {
            rule_id: "NOIR500".to_string(),
            severity: Severity::Warning,
            confidence: Confidence::High,
            policy: "maintainability".to_string(),
            message: "message".to_string(),
            primary_span: Span::new("src/main.nr", 0, 1, 1, 1),
            secondary_spans: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
            structured_suggestions: Vec::new(),
            suggestion_groups: vec![SuggestionGroup {
                id: "sg0001".to_string(),
                message: "grouped".to_string(),
                applicability: Applicability::MachineApplicable,
                edits: vec![
                    TextEdit {
                        span: Span::new("src/main.nr", 0, 1, 1, 1),
                        replacement: "A".to_string(),
                    },
                    TextEdit {
                        span: Span::new("src/other.nr", 0, 1, 1, 1),
                        replacement: "B".to_string(),
                    },
                ],
                provenance: None,
            }],
            fixes: Vec::new(),
            suppressed: false,
            suppression_reason: None,
        };

        let report = apply_fixes(dir.path(), &[diagnostic], FixApplicationMode::Apply)
            .expect("apply should succeed");

        assert!(report.selected.is_empty());
        assert!(
            report
                .skipped
                .iter()
                .any(|skip| skip.reason == SkippedFixReason::MixedFileGroup)
        );
    }
}
