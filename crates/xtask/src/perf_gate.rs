use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use toml::Value;

use crate::common::{
    DynError, ensure_no_unknown_options, parse_flags_and_options, read_text_file, run_command,
    workspace_root,
};

pub fn run(args: &[String]) -> Result<(), DynError> {
    let (mut flags, options) = parse_flags_and_options(args)?;
    let check = flags.remove("check");
    let locked = flags.remove("locked");
    ensure_no_unknown_options(&flags, &options)?;

    let root = workspace_root()?;
    let scenarios = root.join("benchmarks/scenarios.toml");
    let budgets = root.join("benchmarks/budgets.toml");

    if scenarios.is_file() && budgets.is_file() {
        validate_budget_alignment(&scenarios, &budgets)?;
    } else {
        println!(
            "perf-gate: benchmark budget files not present (expected '{}' and '{}'), skipping budget alignment",
            scenarios.display(),
            budgets.display()
        );
    }

    if check {
        let mut smoke = Command::new("cargo");
        smoke
            .arg("test")
            .arg("-p")
            .arg("aztec-lint-aztec")
            .arg("performance_smoke_stays_bounded");
        if locked {
            smoke.arg("--locked");
        }
        smoke.current_dir(&root);
        run_command(&mut smoke)?;
        println!("perf-gate check: smoke benchmark gate passed");
    }

    Ok(())
}

fn validate_budget_alignment(scenarios: &Path, budgets: &Path) -> Result<(), DynError> {
    let scenarios_text = read_text_file(scenarios)?;
    let budgets_text = read_text_file(budgets)?;

    let scenarios_toml: Value = toml::from_str(&scenarios_text)?;
    let budgets_toml: Value = toml::from_str(&budgets_text)?;

    let scenario_keys = top_level_tables(&scenarios_toml);
    let budget_keys = top_level_tables(&budgets_toml);

    let missing = scenario_keys
        .difference(&budget_keys)
        .cloned()
        .collect::<Vec<_>>();

    if missing.is_empty() {
        return Ok(());
    }

    Err(format!(
        "perf-gate budgets missing scenarios: {}",
        missing.join(", ")
    )
    .into())
}

fn top_level_tables(value: &Value) -> BTreeSet<String> {
    let mut keys = BTreeSet::<String>::new();
    let Some(table) = value.as_table() else {
        return keys;
    };

    for (key, val) in table {
        if val.is_table() {
            keys.insert(key.clone());
        }
    }
    keys
}
