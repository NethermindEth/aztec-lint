use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

use aztec_lint_core::lints::{all_lints, render_lints_reference_markdown};

use crate::common::{
    DynError, ensure_no_unknown_options, parse_flags_and_options, read_text_file, run_command,
    validate_rule_id, workspace_root, write_text_file,
};

pub fn run(args: &[String]) -> Result<(), DynError> {
    let (mut flags, options) = parse_flags_and_options(args)?;
    let check = flags.remove("check");
    let locked = flags.remove("locked");
    ensure_no_unknown_options(&flags, &options)?;

    validate_catalog_ids()?;

    let root = workspace_root()?;
    let docs_path = root.join("docs/lints-reference.md");
    let expected_docs = render_lints_reference_markdown();

    if check {
        let actual_docs = read_text_file(&docs_path)?;
        if actual_docs != expected_docs {
            return Err(
                "docs/lints-reference.md is out of date; run `cargo xtask update-lints`".into(),
            );
        }
    } else {
        write_text_file(&docs_path, &expected_docs)?;
    }

    let mut registry_test = Command::new("cargo");
    registry_test
        .arg("test")
        .arg("-p")
        .arg("aztec-lint-rules")
        .arg("full_registry_matches_canonical_lint_catalog");
    if locked {
        registry_test.arg("--locked");
    }
    registry_test.current_dir(&root);
    run_command(&mut registry_test)?;

    ensure_generated_targets_clean(&root)?;

    if check {
        println!("update-lints check: all generated artifacts are in sync");
    } else {
        println!("update-lints: generated artifacts refreshed and verified clean");
    }
    Ok(())
}

fn validate_catalog_ids() -> Result<(), DynError> {
    let mut seen = BTreeSet::<String>::new();
    for lint in all_lints() {
        validate_rule_id(lint.id)?;
        let normalized = lint.id.trim().to_ascii_uppercase();
        if normalized != lint.id {
            return Err(format!("lint id '{}' is not canonical uppercase", lint.id).into());
        }
        if !seen.insert(normalized.clone()) {
            return Err(format!("duplicate lint id '{normalized}' in canonical catalog").into());
        }
    }
    Ok(())
}

fn ensure_generated_targets_clean(root: &PathBuf) -> Result<(), DynError> {
    let generated_targets = [
        "crates/aztec-lint-core/src/lints/mod.rs",
        "crates/aztec-lint-rules/src/engine/registry.rs",
        "docs/lints-reference.md",
    ];

    let output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .arg("--untracked-files=all")
        .arg("--")
        .args(generated_targets)
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err("failed to query git status for generated targets".into());
    }

    if output.stdout.is_empty() {
        return Ok(());
    }

    let dirty = String::from_utf8_lossy(&output.stdout);
    Err(format!(
        "generated lint artifacts are dirty after update; commit or discard generated diff:\n{}",
        dirty.trim_end()
    )
    .into())
}
