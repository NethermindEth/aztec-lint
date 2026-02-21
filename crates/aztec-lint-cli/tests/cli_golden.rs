use assert_cmd::prelude::OutputAssertExt;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn cli_bin() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aztec-lint-cli"));
    cmd.env("CARGO_TERM_COLOR", "never");
    cmd.env("CLICOLOR", "0");
    cmd.env("CLICOLOR_FORCE", "0");
    cmd
}

fn fixture_dir(path: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(path)
}

fn create_git_project(main_source: &str) -> (tempfile::TempDir, PathBuf) {
    let workspace = tempdir().expect("temp dir should be created");
    let project = workspace.path().join("project");
    fs::create_dir_all(project.join("src")).expect("src dir should be created");
    fs::write(
        project.join("Nargo.toml"),
        "[package]\nname=\"project\"\ntype=\"bin\"\nauthors=[\"\"]\n",
    )
    .expect("nargo file should be written");
    fs::write(project.join("src/main.nr"), main_source).expect("main source should be written");

    git(project.as_path(), &["init", "--quiet"]);
    git(project.as_path(), &["config", "user.name", "Test User"]);
    git(
        project.as_path(),
        &["config", "user.email", "test@example.com"],
    );
    git(project.as_path(), &["add", "."]);
    git(project.as_path(), &["commit", "-m", "init", "--quiet"]);

    (workspace, project)
}

fn create_workspace_with_members() -> (tempfile::TempDir, PathBuf) {
    let workspace = tempdir().expect("temp dir should be created");
    let root = workspace.path().join("workspace");
    fs::create_dir_all(&root).expect("workspace root should be created");
    fs::write(
        root.join("Nargo.toml"),
        "[workspace]\nmembers=[\"a\",\"b\"]\n",
    )
    .expect("workspace nargo should be written");

    for member in ["a", "b"] {
        let member_root = root.join(member);
        fs::create_dir_all(member_root.join("src")).expect("member src should be created");
        fs::write(
            member_root.join("Nargo.toml"),
            format!(
                "[package]\nname=\"{member}\"\ntype=\"bin\"\nauthors=[\"\"]\n",
                member = member
            ),
        )
        .expect("member nargo should be written");
        fs::write(
            member_root.join("src/main.nr"),
            "fn main() { let x = 1; assert(x == 1); }\n",
        )
        .expect("member source should be written");
    }

    (workspace, root)
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git command should execute");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn rules_command_matches_golden_output() {
    let expected = "\
RULE_ID\tPACK\tPOLICY\tCONFIDENCE\tSUMMARY\n\
AZTEC001\taztec_pack\tprivacy\tmedium\tPrivate data reaches a public sink.\n\
AZTEC002\taztec_pack\tprivacy\tlow\tSecret-dependent branching affects public state.\n\
AZTEC003\taztec_pack\tprivacy\tmedium\tPrivate entrypoint uses debug logging.\n\
AZTEC010\taztec_pack\tprotocol\thigh\tPrivate to public bridge requires #[only_self].\n\
AZTEC020\taztec_pack\tsoundness\thigh\tUnconstrained influence reaches commitments, storage, or nullifiers.\n\
AZTEC021\taztec_pack\tsoundness\tmedium\tMissing range constraints before hashing or serialization.\n\
AZTEC022\taztec_pack\tsoundness\tmedium\tSuspicious Merkle witness usage.\n\
NOIR001\tnoir_core\tcorrectness\thigh\tUnused variable or import.\n\
NOIR002\tnoir_core\tcorrectness\tmedium\tSuspicious shadowing.\n\
NOIR010\tnoir_core\tcorrectness\thigh\tBoolean computed but not asserted.\n\
NOIR020\tnoir_core\tcorrectness\tmedium\tArray indexing without bounds validation.\n\
NOIR030\tnoir_core\tcorrectness\tmedium\tUnconstrained value influences constrained logic.\n\
NOIR100\tnoir_core\tmaintainability\tlow\tMagic number literal should be named.\n\
NOIR110\tnoir_core\tmaintainability\tlow\tFunction complexity exceeds threshold.\n\
NOIR120\tnoir_core\tmaintainability\tlow\tFunction nesting depth exceeds threshold.\n";

    let mut cmd = cli_bin();
    cmd.arg("rules");
    cmd.assert().success().stdout(expected);
}

#[test]
fn explain_command_matches_golden_output() {
    let expected = "\
Rule: AZTEC001\n\
Pack: aztec_pack\n\
Category: privacy\n\
Policy: privacy\n\
Default Level: deny\n\
Confidence: medium\n\
Introduced In: 0.1.0\n\
Lifecycle: active\n\
\n\
Summary:\n\
Private data reaches a public sink.\n\
\n\
What It Does:\n\
Flags flows where secret or note-derived values are emitted through public channels.\n\
\n\
Why This Matters:\n\
Leaking private values through public outputs can permanently expose sensitive state.\n\
\n\
Known Limitations:\n\
Flow analysis is conservative and may miss leaks routed through unsupported abstractions.\n\
\n\
How To Fix:\n\
Keep private values in constrained private paths and sanitize or avoid public emission points.\n\
\n\
Examples:\n\
- Avoid emitting note-derived values from public entrypoints.\n\
\n\
References:\n\
- docs/suppression.md\n\
- docs/rule-authoring.md\n";

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
fn unknown_cli_override_fails_fast_with_actionable_error() {
    let mut cmd = cli_bin();
    let fixture = fixture_dir("noir_core/minimal");
    cmd.args([
        "check",
        fixture.to_string_lossy().as_ref(),
        "--deny",
        "DOES_NOT_EXIST",
    ]);

    let output = cmd.output().expect("command should execute");
    assert_eq!(
        output.status.code(),
        Some(2),
        "unknown overrides should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown rule id 'DOES_NOT_EXIST' in --deny override"),
        "stderr was: {stderr}"
    );
    assert!(
        stderr.contains("run `aztec-lint rules`"),
        "stderr was: {stderr}"
    );
}

#[test]
fn unknown_profile_override_fails_before_analysis_starts() {
    let workspace = tempdir().expect("temp dir should be created");
    let project = workspace.path().join("project");
    fs::create_dir_all(project.join("src")).expect("src dir should be created");
    fs::write(
        project.join("aztec-lint.toml"),
        "[profile.default]\nruleset=[\"noir_core\"]\ndeny=[\"NOIR404\"]\n",
    )
    .expect("config should be written");
    fs::write(
        project.join("Nargo.toml"),
        "[package]\nname=\"project\"\ntype=\"bin\"\nauthors=[\"\"]\n",
    )
    .expect("nargo file should be written");
    fs::write(project.join("src/main.nr"), "fn main() { assert(true); }\n")
        .expect("entry source should be written");

    let mut cmd = cli_bin();
    cmd.current_dir(&project);
    cmd.args(["check", "."]);
    let output = cmd.output().expect("command should execute");
    assert_eq!(output.status.code(), Some(2), "invalid config should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown rule id 'NOIR404' in profile 'default' deny override"),
        "stderr was: {stderr}"
    );
    assert!(
        stderr.contains("run `aztec-lint rules`"),
        "stderr was: {stderr}"
    );
}

#[test]
fn unknown_default_mode_override_fails_fast_with_actionable_error() {
    let mut cmd = cli_bin();
    let fixture = fixture_dir("noir_core/minimal");
    cmd.args([
        fixture.to_string_lossy().as_ref(),
        "--deny",
        "DOES_NOT_EXIST",
    ]);

    let output = cmd.output().expect("command should execute");
    assert_eq!(
        output.status.code(),
        Some(2),
        "unknown overrides should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown rule id 'DOES_NOT_EXIST' in --deny override"),
        "stderr was: {stderr}"
    );
    assert!(
        stderr.contains("run `aztec-lint rules`"),
        "stderr was: {stderr}"
    );
}

#[test]
fn explain_supports_rules_from_canonical_catalog() {
    let expected = "\
Rule: AZTEC022\n\
Pack: aztec_pack\n\
Category: soundness\n\
Policy: soundness\n\
Default Level: deny\n\
Confidence: medium\n\
Introduced In: 0.1.0\n\
Lifecycle: active\n\
\n\
Summary:\n\
Suspicious Merkle witness usage.\n\
\n\
What It Does:\n\
Finds witness handling patterns that likely violate expected Merkle proof semantics.\n\
\n\
Why This Matters:\n\
Incorrect witness usage can invalidate inclusion guarantees.\n\
\n\
Known Limitations:\n\
Complex custom witness manipulation may produce conservative warnings.\n\
\n\
How To Fix:\n\
Verify witness ordering and path semantics against the target Merkle API contract.\n\
\n\
Examples:\n\
- Ensure witness paths and leaf values are paired using the expected order.\n\
\n\
References:\n\
- docs/rule-authoring.md\n\
- docs/decisions/0003-confidence-model.md\n";

    let mut cmd = cli_bin();
    cmd.args(["explain", "aztec022"]);
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
    assert!(stdout.contains("active_rules=7"), "stdout was: {stdout}");
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
    assert!(stdout.contains("active_rules=8"), "stdout was: {stdout}");
}

#[test]
fn profile_noir_excludes_aztec_pack() {
    let mut cmd = cli_bin();
    let fixture = fixture_dir("noir_core/minimal");
    cmd.args([
        "check",
        fixture.to_string_lossy().as_ref(),
        "--profile",
        "noir",
    ]);

    let output = cmd.output().expect("command should execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("active_rules=8"), "stdout was: {stdout}");
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
    assert!(stdout.contains("active_rules=15"), "stdout was: {stdout}");
}

#[test]
fn bare_invocation_runs_check_with_aztec_profile_by_default() {
    let mut cmd = cli_bin();
    let fixture = fixture_dir("noir_core/minimal");
    cmd.arg(fixture.to_string_lossy().as_ref());

    let output = cmd.output().expect("command should execute");
    assert_eq!(output.status.code(), Some(1), "run should report findings");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("active_rules=15"),
        "bare invocation should default to aztec profile: {stdout}"
    );
}

#[test]
fn bare_invocation_fix_flag_runs_fix_mode() {
    let mut cmd = cli_bin();
    let fixture = fixture_dir("noir_core/minimal");
    cmd.args([fixture.to_string_lossy().as_ref(), "--fix", "--dry-run"]);

    let output = cmd.output().expect("command should execute");
    assert_eq!(output.status.code(), Some(1), "run should report findings");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fix path="),
        "expected fix banner in stdout: {stdout}"
    );
}

#[test]
fn fix_accepts_changed_only_flag() {
    let (_workspace, project) = create_git_project("fn main() { let x = 3; assert(x == 3); }\n");
    fs::write(
        project.join("src/main.nr"),
        "fn main() { let x = 7; assert(x == 7); }\n",
    )
    .expect("changed source should be written");

    let mut cmd = cli_bin();
    cmd.current_dir(&project);
    cmd.args(["fix", ".", "--changed-only"]);
    cmd.assert().code(1);
}

#[test]
fn check_changed_only_ignores_diagnostics_in_unchanged_files() {
    let (_workspace, project) = create_git_project("fn main() { let x = 3; assert(x == 3); }\n");

    let mut cmd = cli_bin();
    cmd.current_dir(&project);
    cmd.args(["check", ".", "--changed-only"]);
    cmd.assert().code(0);
}

#[test]
fn check_changed_only_includes_unstaged_changes() {
    let (_workspace, project) = create_git_project("fn main() { let x = 3; assert(x == 3); }\n");
    fs::write(
        project.join("src/main.nr"),
        "fn main() { let x = 11; assert(x == 11); }\n",
    )
    .expect("changed source should be written");

    let mut cmd = cli_bin();
    cmd.current_dir(&project);
    cmd.args(["check", ".", "--changed-only"]);
    cmd.assert().code(1);
}

#[test]
fn check_changed_only_includes_staged_changes() {
    let (_workspace, project) = create_git_project("fn main() { let x = 3; assert(x == 3); }\n");
    fs::write(
        project.join("src/main.nr"),
        "fn main() { let x = 9; assert(x == 9); }\n",
    )
    .expect("changed source should be written");
    git(project.as_path(), &["add", "src/main.nr"]);

    let mut cmd = cli_bin();
    cmd.current_dir(&project);
    cmd.args(["check", ".", "--changed-only"]);
    cmd.assert().code(1);
}

#[test]
fn check_json_output_includes_suppressed_diagnostics() {
    let (_workspace, project) =
        create_git_project("#[allow(NOIR100)]\nfn main() { let x = 42; assert(x == 42); }\n");

    let mut cmd = cli_bin();
    cmd.current_dir(&project);
    cmd.args(["check", ".", "--format", "json"]);
    let output = cmd.output().expect("command should execute");
    assert_eq!(
        output.status.code(),
        Some(0),
        "suppressed findings are non-blocking"
    );

    let diagnostics: Value =
        serde_json::from_slice(&output.stdout).expect("json output should parse");
    let diagnostics = diagnostics
        .as_array()
        .expect("json diagnostics should be an array");
    assert!(
        !diagnostics.is_empty(),
        "expected at least one suppressed diagnostic"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic["suppressed"] == Value::Bool(true))
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic["suppression_reason"] == Value::String("allow(NOIR100)".to_string())
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic["rule_id"] == Value::String("NOIR100".to_string()))
    );
}

#[test]
fn check_text_show_suppressed_flag_controls_visibility() {
    let (_workspace, project) =
        create_git_project("#[allow(NOIR100)]\nfn main() { let x = 42; assert(x == 42); }\n");

    let mut hidden = cli_bin();
    hidden.current_dir(&project);
    hidden.args(["check", "."]);
    let hidden_output = hidden.output().expect("command should execute");
    assert_eq!(hidden_output.status.code(), Some(0));
    let hidden_stdout = String::from_utf8_lossy(&hidden_output.stdout);
    assert!(
        hidden_stdout.contains("No diagnostics."),
        "default text output should hide suppressed diagnostics: {hidden_stdout}"
    );

    let mut visible = cli_bin();
    visible.current_dir(&project);
    visible.args(["check", ".", "--show-suppressed"]);
    let visible_output = visible.output().expect("command should execute");
    assert_eq!(visible_output.status.code(), Some(0));
    let visible_stdout = String::from_utf8_lossy(&visible_output.stdout);
    assert!(
        visible_stdout.contains("[suppressed: allow(NOIR100)]"),
        "show-suppressed output should include suppression metadata: {visible_stdout}"
    );
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
fn check_text_output_matches_golden_snapshot_with_suggestions() {
    let fixture = fixture_dir("noir_core/minimal");
    let expected_path = fixture_dir("text/noir_core_minimal_with_suggestions.txt");
    let expected = fs::read_to_string(expected_path).expect("snapshot should be readable");

    let mut cmd = cli_bin();
    cmd.args(["check", fixture.to_string_lossy().as_ref()]);
    let output = cmd.output().expect("command should execute");
    assert_eq!(output.status.code(), Some(1), "run should report findings");

    let actual = String::from_utf8_lossy(&output.stdout);
    let fixture_display = fixture.to_string_lossy();
    let normalized = actual.replace(fixture_display.as_ref(), "<FIXTURE>");
    assert_eq!(normalized.trim_end(), expected.trim_end());
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

#[test]
fn check_workspace_root_discovers_member_projects() {
    let (_workspace, root) = create_workspace_with_members();
    let mut cmd = cli_bin();
    cmd.arg("check").arg(&root);
    cmd.assert().code(0);
}
