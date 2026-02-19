use assert_cmd::prelude::OutputAssertExt;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn cli_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aztec-lint-cli"))
}

#[test]
fn rules_command_matches_golden_output() {
    let expected = "\
RULE_ID\tPACK\tPOLICY\tCONFIDENCE\tSUMMARY\n\
AZTEC001\taztec_pack\tprivacy\tmedium\tPrivate data reaches a public sink.\n\
AZTEC002\taztec_pack\tprivacy\thigh\tSecret-dependent branching affects public state.\n\
AZTEC003\taztec_pack\tprivacy\thigh\tPrivate entrypoint uses debug logging.\n\
AZTEC010\taztec_pack\tprotocol\thigh\tPrivate to public bridge requires #[only_self].\n\
AZTEC011\taztec_pack\tprotocol\thigh\tNullifier domain separation fields are missing.\n\
AZTEC012\taztec_pack\tprotocol\thigh\tCommitment domain separation fields are missing.\n\
AZTEC020\taztec_pack\tsoundness\thigh\tUnconstrained influence reaches commitments, storage, or nullifiers.\n\
AZTEC021\taztec_pack\tsoundness\thigh\tMissing range constraints before hashing or serialization.\n\
AZTEC022\taztec_pack\tsoundness\thigh\tSuspicious Merkle witness usage.\n\
AZTEC040\taztec_pack\tconstraint_cost\tmedium\tExpensive primitive appears inside a loop.\n\
AZTEC041\taztec_pack\tconstraint_cost\tmedium\tRepeated membership proofs detected.\n\
NOIR001\tnoir_core\tcorrectness\thigh\tDetects trivially unreachable branch conditions.\n\
NOIR002\tnoir_core\tcorrectness\thigh\tDetects suspicious variable shadowing.\n\
NOIR010\tnoir_core\tcorrectness\thigh\tBoolean value computed but never asserted.\n\
NOIR020\tnoir_core\tcorrectness\thigh\tArray indexing appears without bounds validation.\n\
NOIR030\tnoir_core\tcorrectness\thigh\tUnconstrained value influences constrained logic.\n\
NOIR100\tnoir_core\tmaintainability\tlow\tDetects magic-number literals that should be named.\n\
NOIR110\tnoir_core\tmaintainability\tmedium\tFunction complexity exceeds the recommended limit.\n\
NOIR120\tnoir_core\tmaintainability\tmedium\tExcessive nesting reduces code readability.\n\
NOIR200\tnoir_core\tperformance\tmedium\tHeavy operation appears inside a loop.\n";

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
Confidence: medium\n\
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
    fs::write(
        project.join("aztec-lint.toml"),
        "[profile.default]\nruleset=[\"aztec_pack\"]\n",
    )
    .expect("config should be written");

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
fn fix_accepts_changed_only_flag() {
    let mut cmd = cli_bin();
    cmd.args(["fix", ".", "--changed-only"]);
    cmd.assert().success();
}
