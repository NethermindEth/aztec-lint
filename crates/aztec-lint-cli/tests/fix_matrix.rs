use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

#[path = "support/mod.rs"]
mod support;

fn fix_cases_root() -> PathBuf {
    support::fixtures_root().join("fix/cases")
}

fn discover_fix_cases() -> Vec<PathBuf> {
    let cases = support::sorted_dir_entries(&fix_cases_root())
        .into_iter()
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    assert!(!cases.is_empty(), "no fix matrix cases were discovered");
    cases
}

fn verify_case_contract(case_dir: &Path) {
    let versioned_report = format!("report.{}.json", support::version_tag());
    let mut allowed = BTreeMap::<String, bool>::new();
    allowed.insert("before.nr".to_string(), false);
    allowed.insert("after.nr".to_string(), false);
    allowed.insert(versioned_report.clone(), false);

    for entry in support::sorted_dir_entries(case_dir) {
        let name = entry
            .file_name()
            .and_then(|value| value.to_str())
            .expect("fixture name should be utf-8")
            .to_string();
        if let Some(seen) = allowed.get_mut(&name) {
            *seen = true;
            continue;
        }
        panic!(
            "unexpected fixture file {} in {}; allowed files: before.nr, after.nr, {}",
            entry.display(),
            case_dir.display(),
            versioned_report
        );
    }

    for required in ["before.nr", "after.nr"] {
        assert!(
            *allowed
                .get(required)
                .expect("required fixture should exist in map"),
            "missing required {} in {}",
            required,
            case_dir.display()
        );
    }
}

fn parse_numeric_metrics(stdout: &str) -> BTreeMap<String, usize> {
    let mut metrics = BTreeMap::<String, usize>::new();
    for token in stdout.split_whitespace() {
        let Some((key, value)) = token.split_once('=') else {
            continue;
        };
        if let Ok(parsed) = value.parse::<usize>() {
            metrics.insert(key.to_string(), parsed);
        }
    }
    metrics
}

fn run_fix(project: &Path) -> (i32, String, String) {
    let mut cmd = support::cli_bin();
    cmd.current_dir(project);
    cmd.arg("fix").arg(".");

    let output = cmd.output().expect("fix command should execute");
    support::ensure_allowed_exit(&output, "fix-matrix run");

    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn fix_matrix_applies_before_after_contracts() {
    for case in discover_fix_cases() {
        verify_case_contract(case.as_path());

        let before = support::read_file(case.join("before.nr").as_path());
        let expected_after = support::read_file(case.join("after.nr").as_path());

        let (_tmp_a, project_a) = support::create_temp_project(
            before.as_str(),
            &format!(
                "fix_{}",
                case.file_name()
                    .and_then(|value| value.to_str())
                    .expect("case name should be utf-8")
            ),
        );
        let (code_a, stdout_a, stderr_a) = run_fix(project_a.as_path());

        let (_tmp_b, project_b) = support::create_temp_project(
            before.as_str(),
            &format!(
                "fix_{}_repeat",
                case.file_name()
                    .and_then(|value| value.to_str())
                    .expect("case name should be utf-8")
            ),
        );
        let (code_b, stdout_b, stderr_b) = run_fix(project_b.as_path());

        assert_eq!(
            code_a,
            code_b,
            "exit code is not deterministic for {}",
            case.display()
        );
        assert_eq!(
            support::normalize_output(stdout_a.as_str(), project_a.as_path()),
            support::normalize_output(stdout_b.as_str(), project_b.as_path()),
            "stdout is not deterministic for {}",
            case.display()
        );
        assert_eq!(
            support::normalize_output(stderr_a.as_str(), project_a.as_path()),
            support::normalize_output(stderr_b.as_str(), project_b.as_path()),
            "stderr is not deterministic for {}",
            case.display()
        );

        let actual_after = support::read_file(project_a.join("src/main.nr").as_path());
        assert_eq!(
            actual_after.trim_end(),
            expected_after.trim_end(),
            "post-fix source mismatch for {}",
            case.display()
        );

        let report = case.join(format!("report.{}.json", support::version_tag()));
        if report.is_file() {
            let expected: Value =
                serde_json::from_str(support::read_file(report.as_path()).as_str())
                    .expect("report json should parse");

            if let Some(exit_code) = expected["exit_code"].as_i64() {
                assert_eq!(
                    code_a,
                    exit_code as i32,
                    "unexpected fix exit code for {}",
                    case.display()
                );
            }

            let metrics = parse_numeric_metrics(stdout_a.as_str());
            if let Some(object) = expected["metrics"].as_object() {
                for (key, value) in object {
                    let expected_metric = value
                        .as_u64()
                        .unwrap_or_else(|| panic!("metric '{key}' must be a number"));
                    let actual_metric = metrics
                        .get(key)
                        .copied()
                        .unwrap_or_else(|| panic!("missing metric '{key}' in fix output"));
                    assert_eq!(
                        actual_metric as u64,
                        expected_metric,
                        "metric '{key}' mismatch for {}",
                        case.display()
                    );
                }
            }

            if let Some(items) = expected["stdout_contains"].as_array() {
                for needle in items {
                    let needle = needle
                        .as_str()
                        .expect("stdout_contains entries must be strings");
                    assert!(
                        stdout_a.contains(needle),
                        "missing '{needle}' in fix output for {}",
                        case.display()
                    );
                }
            }
        }
    }
}
