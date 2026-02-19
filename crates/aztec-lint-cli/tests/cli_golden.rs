use assert_cmd::prelude::OutputAssertExt;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn cli_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aztec-lint-cli"))
}

fn fixture_dir(path: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(path)
}

#[test]
fn rules_command_matches_golden_output() {
    let expected = "\
RULE_ID\tPACK\tPOLICY\tCONFIDENCE\tSUMMARY\n\
AZTEC001\taztec_pack\tprivacy\tmedium\tPrivate data reaches a public sink.\n\
AZTEC002\taztec_pack\tprivacy\tlow\tSecret-dependent branching affects public state.\n\
AZTEC003\taztec_pack\tprivacy\tmedium\tPrivate entrypoint uses debug logging.\n\
AZTEC010\taztec_pack\tprotocol\thigh\tPrivate to public bridge requires #[only_self].\n\
AZTEC011\taztec_pack\tprotocol\tmedium\tNullifier domain separation fields are missing.\n\
AZTEC012\taztec_pack\tprotocol\tmedium\tCommitment domain separation fields are missing.\n\
AZTEC020\taztec_pack\tsoundness\thigh\tUnconstrained influence reaches commitments, storage, or nullifiers.\n\
AZTEC021\taztec_pack\tsoundness\tmedium\tMissing range constraints before hashing or serialization.\n\
AZTEC022\taztec_pack\tsoundness\tmedium\tSuspicious Merkle witness usage.\n\
AZTEC040\taztec_pack\tconstraint_cost\tlow\tExpensive primitive appears inside a loop.\n\
AZTEC041\taztec_pack\tconstraint_cost\tlow\tRepeated membership proofs detected.\n\
NOIR001\tnoir_core\tcorrectness\thigh\tDetects trivially unreachable branch conditions.\n\
NOIR002\tnoir_core\tcorrectness\tmedium\tDetects suspicious variable shadowing.\n\
NOIR010\tnoir_core\tcorrectness\thigh\tBoolean value computed but never asserted.\n\
NOIR020\tnoir_core\tcorrectness\tmedium\tArray indexing appears without bounds validation.\n\
NOIR030\tnoir_core\tcorrectness\tmedium\tUnconstrained value influences constrained logic.\n\
NOIR100\tnoir_core\tmaintainability\tlow\tDetects magic-number literals that should be named.\n\
NOIR110\tnoir_core\tmaintainability\tlow\tFunction complexity exceeds the recommended limit.\n\
NOIR120\tnoir_core\tmaintainability\tlow\tExcessive nesting reduces code readability.\n\
NOIR200\tnoir_core\tperformance\tlow\tHeavy operation appears inside a loop.\n";

    let mut cmd = cli_bin();
    cmd.arg("rules");
    cmd.assert().success().stdout(expected);
}

#[test]
fn explain_command_matches_golden_output() {
    let expected = "\
Rule: AZTEC001\n\
Pack: aztec_pack\n\
Policy: privacy\n\
Confidence: medium\n\
Summary: Private data reaches a public sink.\n";

    let mut cmd = cli_bin();
    cmd.args(["explain", "AZTEC001"]);
    cmd.assert().success().stdout(expected);
}

#[test]
fn invalid_flag_combination_returns_exit_code_two() {
    let mut cmd = cli_bin();
    cmd.args(["rules", "--profile", "aztec"]);
    cmd.assert().code(2);
}

#[test]
fn explain_supports_rules_from_full_catalog() {
    let expected = "\
Rule: AZTEC041\n\
Pack: aztec_pack\n\
Policy: constraint_cost\n\
Confidence: low\n\
Summary: Repeated membership proofs detected.\n";

    let mut cmd = cli_bin();
    cmd.args(["explain", "aztec041"]);
    cmd.assert().success().stdout(expected);
}

#[test]
fn check_loads_config_from_target_path() {
    let workspace = tempdir().expect("temp dir should be created");
    let project = workspace.path().join("project");
    fs::create_dir(&project).expect("project dir should be created");
    fs::create_dir(project.join("src")).expect("src dir should be created");
    fs::write(
        project.join("aztec-lint.toml"),
        "[profile.default]\nruleset=[\"aztec_pack\"]\n",
    )
    .expect("config should be written");
    fs::write(
        project.join("Nargo.toml"),
        "[package]\nname=\"project\"\ntype=\"bin\"\nauthors=[\"\"]\n",
    )
    .expect("nargo file should be written");
    fs::write(
        project.join("src/main.nr"),
        "fn main() { let x = 1; assert(x == 1); }\n",
    )
    .expect("entry source should be written");

    let mut cmd = cli_bin();
    cmd.current_dir(workspace.path());
    cmd.arg("check").arg(&project);
    let output = cmd.output().expect("command should execute");
    assert!(
        output.status.success(),
        "expected success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("active_rules=11"), "stdout was: {stdout}");
}

#[test]
fn check_fixture_directory_returns_failure_when_any_project_has_errors() {
    let mut cmd = cli_bin();
    let fixture = fixture_dir("noir_core");
    cmd.args(["check", fixture.to_string_lossy().as_ref()]);
    cmd.assert().code(1);
}

#[test]
fn check_subdirectory_inside_project_discovers_parent_project() {
    let mut cmd = cli_bin();
    let fixture = fixture_dir("noir_core/minimal/src");
    cmd.args(["check", fixture.to_string_lossy().as_ref()]);
    cmd.assert().code(1);
}

#[test]
fn check_severity_threshold_error_ignores_warning_only_results() {
    let mut cmd = cli_bin();
    let fixture = fixture_dir("noir_core/warnings_only");
    cmd.args([
        "check",
        fixture.to_string_lossy().as_ref(),
        "--severity-threshold",
        "error",
    ]);
    cmd.assert().code(0);
}

#[test]
fn check_warning_threshold_reports_warning_only_results() {
    let mut cmd = cli_bin();
    let fixture = fixture_dir("noir_core/warnings_only");
    cmd.args([
        "check",
        fixture.to_string_lossy().as_ref(),
        "--severity-threshold",
        "warning",
    ]);
    cmd.assert().code(1);
}

#[test]
fn profile_default_excludes_aztec_pack() {
    let mut cmd = cli_bin();
    let fixture = fixture_dir("noir_core/minimal");
    cmd.args([
        "check",
        fixture.to_string_lossy().as_ref(),
        "--profile",
        "default",
    ]);

    let output = cmd.output().expect("command should execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("active_rules=9"), "stdout was: {stdout}");
}

#[test]
fn profile_aztec_includes_default_and_aztec_pack() {
    let mut cmd = cli_bin();
    let fixture = fixture_dir("noir_core/minimal");
    cmd.args([
        "check",
        fixture.to_string_lossy().as_ref(),
        "--profile",
        "aztec",
    ]);

    let output = cmd.output().expect("command should execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("active_rules=20"), "stdout was: {stdout}");
}

#[test]
fn fix_accepts_changed_only_flag() {
    let mut cmd = cli_bin();
    cmd.args(["fix", ".", "--changed-only"]);
    cmd.assert().success();
}

#[test]
fn check_json_output_is_deterministic() {
    let fixture = fixture_dir("noir_core/minimal");

    let mut first_cmd = cli_bin();
    first_cmd.args([
        "check",
        fixture.to_string_lossy().as_ref(),
        "--format",
        "json",
    ]);
    let first = first_cmd.output().expect("first run should execute");
    assert_eq!(
        first.status.code(),
        Some(1),
        "first run should report findings"
    );

    let mut second_cmd = cli_bin();
    second_cmd.args([
        "check",
        fixture.to_string_lossy().as_ref(),
        "--format",
        "json",
    ]);
    let second = second_cmd.output().expect("second run should execute");
    assert_eq!(
        second.status.code(),
        Some(1),
        "second run should report findings"
    );

    assert_eq!(
        first.stdout, second.stdout,
        "json output should be deterministic across runs"
    );
}

#[test]
fn check_sarif_output_matches_golden_snapshot() {
    let fixture = fixture_dir("noir_core/minimal");
    let expected_path = fixture_dir("sarif/noir_core_minimal.sarif.json");
    let expected = fs::read_to_string(expected_path).expect("snapshot should be readable");

    let mut cmd = cli_bin();
    cmd.args([
        "check",
        fixture.to_string_lossy().as_ref(),
        "--format",
        "sarif",
    ]);
    let output = cmd.output().expect("command should execute");
    assert_eq!(output.status.code(), Some(1), "run should report findings");

    let actual = String::from_utf8_lossy(&output.stdout);
    assert_eq!(actual.trim_end(), expected.trim_end());
}

#[test]
fn check_sarif_uses_relative_uri_and_partial_fingerprint() {
    let fixture = fixture_dir("noir_core/minimal");
    let fixture_path = fixture.to_string_lossy();

    let mut cmd = cli_bin();
    cmd.args(["check", fixture_path.as_ref(), "--format", "sarif"]);
    let output = cmd.output().expect("command should execute");
    assert_eq!(output.status.code(), Some(1), "run should report findings");

    let value: Value =
        serde_json::from_slice(&output.stdout).expect("sarif output should parse as json");
    let results = value["runs"][0]["results"]
        .as_array()
        .expect("sarif results should be an array");
    assert!(!results.is_empty(), "expected at least one sarif result");

    for result in results {
        let uri = result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
            .as_str()
            .expect("uri should be a string");
        assert!(
            !uri.starts_with('/'),
            "uri should be repository-relative: {uri}"
        );
        assert!(
            !uri.contains(fixture_path.as_ref()),
            "uri should not include absolute fixture path: {uri}"
        );
        let fingerprint = result["partialFingerprints"]["aztecLint/v1"]
            .as_str()
            .expect("partial fingerprint should be present");
        assert!(
            !fingerprint.is_empty(),
            "partial fingerprint should not be empty"
        );
    }
}

#[test]
fn check_sarif_paths_include_project_prefix_for_multi_project_targets() {
    let fixture = fixture_dir("noir_core");
    let mut cmd = cli_bin();
    cmd.args([
        "check",
        fixture.to_string_lossy().as_ref(),
        "--format",
        "sarif",
    ]);

    let output = cmd.output().expect("command should execute");
    assert_eq!(output.status.code(), Some(1), "run should report findings");

    let value: Value =
        serde_json::from_slice(&output.stdout).expect("sarif output should parse as json");
    let results = value["runs"][0]["results"]
        .as_array()
        .expect("sarif results should be an array");
    assert!(!results.is_empty(), "expected at least one sarif result");

    let uris = results
        .iter()
        .filter_map(|result| {
            result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
                .as_str()
                .map(str::to_string)
        })
        .collect::<BTreeSet<_>>();

    assert!(
        uris.iter().any(|uri| uri.starts_with("minimal/")),
        "expected at least one minimal project URI, got: {uris:?}"
    );
    assert!(
        uris.iter().any(|uri| uri.starts_with("warnings_only/")),
        "expected at least one warnings_only project URI, got: {uris:?}"
    );
}

#[test]
fn check_min_confidence_high_ignores_low_confidence_findings() {
    let mut cmd = cli_bin();
    let fixture = fixture_dir("noir_core/warnings_only");
    cmd.args([
        "check",
        fixture.to_string_lossy().as_ref(),
        "--min-confidence",
        "high",
    ]);
    cmd.assert().code(0);
}

#[test]
fn check_without_discoverable_project_returns_internal_error() {
    let workspace = tempdir().expect("temp dir should be created");
    let mut cmd = cli_bin();
    cmd.arg("check").arg(workspace.path());
    cmd.assert().code(2);
}
