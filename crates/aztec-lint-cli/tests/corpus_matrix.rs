use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

#[path = "support/mod.rs"]
mod support;

const MIN_CORPUS_PROJECTS: usize = 5;

fn corpus_projects_root() -> PathBuf {
    support::fixtures_root().join("corpus/projects")
}

fn discover_corpus_projects() -> Vec<PathBuf> {
    let mut projects = support::sorted_dir_entries(&corpus_projects_root())
        .into_iter()
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    projects.sort();
    assert!(
        projects.len() >= MIN_CORPUS_PROJECTS,
        "corpus matrix requires at least {MIN_CORPUS_PROJECTS} projects; found {}",
        projects.len()
    );
    projects
}

fn run_json_check(project: &Path) -> (i32, String, String) {
    let mut cmd = support::cli_bin();
    cmd.current_dir(project);
    cmd.args(["check", ".", "--format", "json"]);

    let output = cmd.output().expect("corpus check command should execute");
    support::ensure_allowed_exit(&output, "corpus-matrix run");

    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn summary_from_diagnostics(diagnostics: &[Value]) -> BTreeMap<&'static str, usize> {
    let mut summary = BTreeMap::from([
        ("total", diagnostics.len()),
        ("errors", 0usize),
        ("warnings", 0usize),
        ("suppressed", 0usize),
    ]);

    for diagnostic in diagnostics {
        match diagnostic["severity"].as_str() {
            Some("error") => *summary.get_mut("errors").expect("errors key should exist") += 1,
            Some("warning") => {
                *summary
                    .get_mut("warnings")
                    .expect("warnings key should exist") += 1
            }
            _ => {}
        }
        if diagnostic["suppressed"] == Value::Bool(true) {
            *summary
                .get_mut("suppressed")
                .expect("suppressed key should exist") += 1;
        }
    }

    summary
}

#[test]
fn corpus_matrix_matches_expected_summaries_and_golden_diagnostics() {
    let versioned_expected = format!("expected.{}.json", support::version_tag());

    for project in discover_corpus_projects() {
        assert!(
            project.join("Nargo.toml").is_file(),
            "missing Nargo.toml in corpus project {}",
            project.display()
        );
        assert!(
            project.join("src/main.nr").is_file(),
            "missing src/main.nr in corpus project {}",
            project.display()
        );
        let expected_path = project.join(&versioned_expected);
        assert!(
            expected_path.is_file(),
            "missing {} in corpus project {}",
            versioned_expected,
            project.display()
        );

        let (code_first, stdout_first, stderr_first) = run_json_check(project.as_path());
        let (code_second, stdout_second, stderr_second) = run_json_check(project.as_path());

        assert_eq!(
            code_first,
            code_second,
            "corpus exit code is not deterministic for {}",
            project.display()
        );
        assert_eq!(
            stdout_first,
            stdout_second,
            "corpus json output is not deterministic for {}",
            project.display()
        );
        assert_eq!(
            support::normalize_output(stderr_first.as_str(), project.as_path()),
            support::normalize_output(stderr_second.as_str(), project.as_path()),
            "corpus stderr output is not deterministic for {}",
            project.display()
        );

        let diagnostics: Value =
            serde_json::from_str(stdout_first.as_str()).expect("json output should parse");
        let diagnostics = diagnostics
            .as_array()
            .expect("json output should contain a diagnostics array");

        let expected: Value =
            serde_json::from_str(support::read_file(expected_path.as_path()).as_str())
                .expect("expected corpus contract should be valid json");

        if let Some(exit_code) = expected["exit_code"].as_i64() {
            assert_eq!(
                code_first,
                exit_code as i32,
                "unexpected corpus exit code for {}",
                project.display()
            );
        }

        if let Some(summary_expected) = expected["summary"].as_object() {
            let summary_actual = summary_from_diagnostics(diagnostics);
            for (key, value) in summary_expected {
                let expected_value = value
                    .as_u64()
                    .unwrap_or_else(|| panic!("summary '{key}' must be numeric"));
                let actual_value = summary_actual
                    .get(key.as_str())
                    .copied()
                    .unwrap_or_else(|| panic!("summary '{}' missing in actual diagnostics", key));
                assert_eq!(
                    actual_value as u64,
                    expected_value,
                    "summary '{}' mismatch for {}",
                    key,
                    project.display()
                );
            }
        }

        if let Some(golden_items) = expected["golden"].as_array() {
            for golden in golden_items {
                let rule_id = golden["rule_id"]
                    .as_str()
                    .expect("golden rule_id must be provided");
                let file = golden["file"]
                    .as_str()
                    .expect("golden file must be provided");
                let severity = golden["severity"]
                    .as_str()
                    .expect("golden severity must be provided");

                let found = diagnostics.iter().any(|diagnostic| {
                    diagnostic["rule_id"] == Value::String(rule_id.to_string())
                        && diagnostic["severity"] == Value::String(severity.to_string())
                        && diagnostic["primary_span"]["file"] == Value::String(file.to_string())
                });

                assert!(
                    found,
                    "missing golden diagnostic rule={} file={} severity={} in {}",
                    rule_id,
                    file,
                    severity,
                    project.display()
                );
            }
        }
    }
}
