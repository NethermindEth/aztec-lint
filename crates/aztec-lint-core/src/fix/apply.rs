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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixApplicationResult {
    pub rule_id: String,
    pub file: String,
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkippedFixReason {
    SuppressedDiagnostic,
    UnsafeFix,
    OverlappingFix,
    InvalidSpan,
    Noop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkippedFix {
    pub rule_id: String,
    pub file: String,
    pub start: u32,
    pub end: u32,
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
struct FixCandidate {
    ordinal: usize,
    rule_id: String,
    confidence: Confidence,
    file: String,
    start: usize,
    end: usize,
    replacement: String,
}

impl FixCandidate {
    fn to_skipped(&self, reason: SkippedFixReason) -> SkippedFix {
        SkippedFix {
            rule_id: self.rule_id.clone(),
            file: self.file.clone(),
            start: u32::try_from(self.start).unwrap_or(u32::MAX),
            end: u32::try_from(self.end).unwrap_or(u32::MAX),
            reason,
        }
    }

    fn to_selected(&self) -> FixApplicationResult {
        FixApplicationResult {
            rule_id: self.rule_id.clone(),
            file: self.file.clone(),
            start: u32::try_from(self.start).unwrap_or(u32::MAX),
            end: u32::try_from(self.end).unwrap_or(u32::MAX),
        }
    }
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

    let mut candidates_by_file = BTreeMap::<String, Vec<FixCandidate>>::new();
    let mut ordinal = 0usize;

    for diagnostic in diagnostics {
        for fix in &diagnostic.fixes {
            report.total_candidates += 1;
            ordinal += 1;

            if diagnostic.suppressed {
                report.skipped.push(SkippedFix {
                    rule_id: diagnostic.rule_id.clone(),
                    file: normalize_file_path(&fix.span.file),
                    start: fix.span.start,
                    end: fix.span.end,
                    reason: SkippedFixReason::SuppressedDiagnostic,
                });
                continue;
            }

            if fix.safety != FixSafety::Safe {
                report.skipped.push(SkippedFix {
                    rule_id: diagnostic.rule_id.clone(),
                    file: normalize_file_path(&fix.span.file),
                    start: fix.span.start,
                    end: fix.span.end,
                    reason: SkippedFixReason::UnsafeFix,
                });
                continue;
            }

            candidates_by_file
                .entry(normalize_file_path(&fix.span.file))
                .or_default()
                .push(FixCandidate {
                    ordinal,
                    rule_id: diagnostic.rule_id.clone(),
                    confidence: diagnostic.confidence,
                    file: normalize_file_path(&fix.span.file),
                    start: usize::try_from(fix.span.start).unwrap_or(usize::MAX),
                    end: usize::try_from(fix.span.end).unwrap_or(usize::MAX),
                    replacement: fix.replacement.clone(),
                });
        }
    }

    for (file, mut candidates) in candidates_by_file {
        let winners = resolve_overlaps(&mut candidates, &mut report.skipped);
        if winners.is_empty() {
            continue;
        }

        let path = resolve_path(root, &file);
        let mut content = fs::read_to_string(&path).map_err(|source| FixError::Io {
            path: path.clone(),
            source,
        })?;
        let mut changed = false;

        for candidate in winners {
            if !valid_span(&content, candidate.start, candidate.end) {
                report
                    .skipped
                    .push(candidate.to_skipped(SkippedFixReason::InvalidSpan));
                continue;
            }

            if content[candidate.start..candidate.end] == candidate.replacement {
                report
                    .skipped
                    .push(candidate.to_skipped(SkippedFixReason::Noop));
                continue;
            }

            content.replace_range(candidate.start..candidate.end, &candidate.replacement);
            report.selected.push(candidate.to_selected());
            changed = true;
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

fn resolve_overlaps(
    candidates: &mut [FixCandidate],
    skipped: &mut Vec<SkippedFix>,
) -> Vec<FixCandidate> {
    candidates.sort_by(|left, right| {
        (
            left.start,
            left.end,
            left.rule_id.as_str(),
            left.ordinal,
            left.replacement.as_str(),
        )
            .cmp(&(
                right.start,
                right.end,
                right.rule_id.as_str(),
                right.ordinal,
                right.replacement.as_str(),
            ))
    });

    let mut winners = Vec::<FixCandidate>::new();
    for candidate in candidates.iter().cloned() {
        let overlapping = winners
            .iter()
            .enumerate()
            .filter_map(|(idx, existing)| {
                ranges_overlap(candidate.start, candidate.end, existing.start, existing.end)
                    .then_some(idx)
            })
            .collect::<Vec<_>>();

        if overlapping.is_empty() {
            winners.push(candidate);
            continue;
        }

        let candidate_wins = overlapping
            .iter()
            .all(|idx| outranks(&candidate, &winners[*idx]));

        if !candidate_wins {
            skipped.push(candidate.to_skipped(SkippedFixReason::OverlappingFix));
            continue;
        }

        for idx in overlapping.into_iter().rev() {
            let loser = winners.remove(idx);
            skipped.push(loser.to_skipped(SkippedFixReason::OverlappingFix));
        }
        winners.push(candidate);
    }

    winners.sort_by_key(|candidate| std::cmp::Reverse(candidate.start));
    winners
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

fn outranks(candidate: &FixCandidate, incumbent: &FixCandidate) -> bool {
    match confidence_rank(candidate.confidence).cmp(&confidence_rank(incumbent.confidence)) {
        Ordering::Greater => return true,
        Ordering::Less => return false,
        Ordering::Equal => {}
    }

    match candidate.rule_id.cmp(&incumbent.rule_id) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => candidate.ordinal < incumbent.ordinal,
    }
}

fn confidence_rank(confidence: Confidence) -> u8 {
    match confidence {
        Confidence::Low => 1,
        Confidence::Medium => 2,
        Confidence::High => 3,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{FixApplicationMode, SkippedFixReason, apply_fixes};
    use crate::diagnostics::{Confidence, Diagnostic, Fix, FixSafety, Severity};
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
                .any(|skip| skip.reason == SkippedFixReason::Noop)
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
            skip.rule_id == "NOIR200" && skip.reason == SkippedFixReason::OverlappingFix
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
            skip.rule_id == "NOIR200" && skip.reason == SkippedFixReason::OverlappingFix
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
                .any(|skip| skip.reason == SkippedFixReason::InvalidSpan)
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
            skip.rule_id == "NOIR200" && skip.reason == SkippedFixReason::OverlappingFix
        }));
        assert_eq!(
            fs::read_to_string(&source_path).expect("file should be readable"),
            "aXc\n"
        );
    }
}
