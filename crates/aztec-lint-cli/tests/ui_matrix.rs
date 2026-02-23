use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[path = "support/mod.rs"]
mod support;

fn ui_cases_dir() -> PathBuf {
    support::fixtures_root().join("ui/cases")
}

fn accepted_fixture_root() -> PathBuf {
    support::fixtures_root().join("ui/accepted")
}

fn run_check(project: &Path, format: Option<&str>) -> (String, String) {
    let mut cmd = support::cli_bin();
    cmd.current_dir(project);
    cmd.arg("check").arg(".");
    if let Some(format) = format {
        cmd.args(["--format", format]);
    }

    let output = cmd.output().expect("check command should execute");
    support::ensure_allowed_exit(&output, "ui-matrix check");

    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn versioned_snapshot(case: &Path, format: &str) -> PathBuf {
    let stem = case
        .file_stem()
        .and_then(|value| value.to_str())
        .expect("case file stem should be valid utf-8");
    case.with_file_name(format!("{stem}.{format}.{}.snap", support::version_tag()))
}

fn discover_ui_cases() -> Vec<PathBuf> {
    let mut cases = support::sorted_dir_entries(&ui_cases_dir())
        .into_iter()
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("nr"))
        .collect::<Vec<_>>();
    cases.sort();
    assert!(
        !cases.is_empty(),
        "no UI matrix fixture cases were discovered"
    );
    cases
}

fn accepted_ids_from_new_lints() -> BTreeSet<String> {
    let path = support::workspace_root().join("docs/NEW_LINTS.md");
    let markdown = support::read_file(path.as_path());
    let mut lines = markdown.lines();
    let header = "| Proposal | Status | Canonical mapping | Notes |";

    while let Some(line) = lines.next() {
        if line.trim() != header {
            continue;
        }

        let mut accepted = BTreeSet::<String>::new();
        for row in lines.by_ref() {
            let trimmed = row.trim();
            if !trimmed.starts_with('|') {
                break;
            }
            if trimmed.starts_with("|---") {
                continue;
            }
            let cells = trimmed
                .split('|')
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .collect::<Vec<_>>();
            if cells.len() != 4 {
                continue;
            }
            if cells[1] != "`accepted`" {
                continue;
            }
            let canonical = cells[2].trim_matches('`').trim();
            if canonical.starts_with("AZTEC") || canonical.starts_with("NOIR") {
                accepted.insert(canonical.to_string());
            }
        }

        return accepted;
    }

    panic!(
        "failed to find lint intake mapping table in {}",
        path.display()
    );
}

#[test]
fn ui_matrix_snapshots_match_versioned_contracts() {
    let cases = discover_ui_cases();

    for case in cases {
        let source = support::read_file(case.as_path());
        let expected_text = versioned_snapshot(case.as_path(), "text");
        let expected_json = versioned_snapshot(case.as_path(), "json");
        let expected_sarif = versioned_snapshot(case.as_path(), "sarif");

        for snapshot in [&expected_text, &expected_json, &expected_sarif] {
            assert!(
                snapshot.is_file(),
                "missing snapshot {} for case {}",
                snapshot.display(),
                case.display()
            );
        }

        let (_tmp, project) = support::create_temp_project(
            source.as_str(),
            &format!(
                "ui_{}",
                case.file_stem()
                    .and_then(|value| value.to_str())
                    .expect("case stem should be utf-8")
            ),
        );

        let (text_first, text_stderr_first) = run_check(project.as_path(), None);
        let (text_second, text_stderr_second) = run_check(project.as_path(), None);
        assert_eq!(
            support::normalize_output(text_first.as_str(), project.as_path()),
            support::normalize_output(text_second.as_str(), project.as_path()),
            "text output is not deterministic for {}",
            case.display()
        );
        assert_eq!(
            support::normalize_output(text_stderr_first.as_str(), project.as_path()),
            support::normalize_output(text_stderr_second.as_str(), project.as_path()),
            "text stderr output is not deterministic for {}",
            case.display()
        );
        let expected = support::read_file(expected_text.as_path());
        assert_eq!(
            support::normalize_output(text_first.as_str(), project.as_path()).trim_end(),
            expected.trim_end(),
            "text snapshot mismatch for {}",
            case.display()
        );

        let (json_first, json_stderr_first) = run_check(project.as_path(), Some("json"));
        let (json_second, json_stderr_second) = run_check(project.as_path(), Some("json"));
        assert_eq!(
            json_first,
            json_second,
            "json output is not deterministic for {}",
            case.display()
        );
        assert_eq!(
            json_stderr_first,
            json_stderr_second,
            "json stderr output is not deterministic for {}",
            case.display()
        );
        let expected = support::read_file(expected_json.as_path());
        assert_eq!(
            json_first.trim_end(),
            expected.trim_end(),
            "json snapshot mismatch for {}",
            case.display()
        );

        let (sarif_first, sarif_stderr_first) = run_check(project.as_path(), Some("sarif"));
        let (sarif_second, sarif_stderr_second) = run_check(project.as_path(), Some("sarif"));
        assert_eq!(
            sarif_first,
            sarif_second,
            "sarif output is not deterministic for {}",
            case.display()
        );
        assert_eq!(
            sarif_stderr_first,
            sarif_stderr_second,
            "sarif stderr output is not deterministic for {}",
            case.display()
        );
        let expected = support::read_file(expected_sarif.as_path());
        assert_eq!(
            sarif_first.trim_end(),
            expected.trim_end(),
            "sarif snapshot mismatch for {}",
            case.display()
        );
    }
}

#[test]
fn accepted_lints_have_required_ui_fixture_pack() {
    let required_files = [
        "positive.nr",
        "negative.nr",
        "suppressed.nr",
        "false_positive_guard.nr",
    ];

    for lint_id in accepted_ids_from_new_lints() {
        let lint_dir = accepted_fixture_root().join(&lint_id);
        assert!(
            lint_dir.is_dir(),
            "missing accepted lint fixture directory {}",
            lint_dir.display()
        );

        for required in required_files {
            let path = lint_dir.join(required);
            assert!(
                path.is_file(),
                "missing required fixture {} for accepted lint {}",
                path.display(),
                lint_id
            );
            let source = support::read_file(path.as_path());
            assert!(
                !source.trim().is_empty(),
                "fixture {} must not be empty",
                path.display()
            );
        }
    }
}
