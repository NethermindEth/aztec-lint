#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

pub fn cli_bin() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aztec-lint-cli"));
    cmd.env("CARGO_TERM_COLOR", "never");
    cmd.env("CLICOLOR", "0");
    cmd.env("CLICOLOR_FORCE", "0");
    cmd
}

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn fixtures_root() -> PathBuf {
    workspace_root().join("fixtures")
}

pub fn version_tag() -> String {
    format!("v{}", env!("CARGO_PKG_VERSION"))
}

pub fn read_file(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

pub fn create_temp_project(main_source: &str, package_name: &str) -> (TempDir, PathBuf) {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let project = temp.path().join(package_name);
    fs::create_dir_all(project.join("src")).expect("src directory should be created");

    fs::write(
        project.join("Nargo.toml"),
        format!("[package]\nname=\"{package_name}\"\ntype=\"bin\"\nauthors=[\"\"]\n"),
    )
    .expect("Nargo.toml should be written");

    fs::write(project.join("src/main.nr"), main_source).expect("main.nr should be written");

    (temp, project)
}

pub fn sorted_dir_entries(path: &Path) -> Vec<PathBuf> {
    let mut entries = fs::read_dir(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
        .map(|entry| entry.expect("dir entry should be readable").path())
        .collect::<Vec<_>>();
    entries.sort();
    entries
}

pub fn normalize_output(raw: &str, project_root: &Path) -> String {
    let root = project_root.to_string_lossy();
    raw.replace(root.as_ref(), "<PROJECT_ROOT>")
        .replace("\r\n", "\n")
}

pub fn ensure_allowed_exit(output: &Output, context: &str) {
    let code = output.status.code().unwrap_or(-1);
    assert!(
        code == 0 || code == 1,
        "unexpected exit code {code} for {context}; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
