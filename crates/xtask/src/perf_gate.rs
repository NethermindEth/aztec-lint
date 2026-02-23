use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use aztec_lint_aztec::taint::{analyze_intra_procedural, build_def_use_graph};
use aztec_lint_aztec::{SourceUnit, build_aztec_model};
use aztec_lint_core::config::AztecConfig;
use toml::Value;

use crate::common::{
    DynError, ensure_no_unknown_options, parse_flags_and_options, read_text_file, run_command,
    workspace_root,
};

const SCENARIOS_FILE: &str = "benchmarks/scenarios.toml";
const BUDGETS_FILE: &str = "benchmarks/budgets.toml";

#[derive(Clone, Debug)]
struct PerfScenario {
    id: String,
    fixture: String,
    expected_min_flows: usize,
    iterations: usize,
}

#[derive(Clone, Copy, Debug)]
struct PerfBudget {
    median_ms: f64,
    p95_ms: f64,
}

#[derive(Clone, Debug)]
struct PerfConfig {
    warmup_runs: usize,
    sample_runs: usize,
    noise_percent: f64,
    hard_fail_percent: f64,
    scenarios: Vec<PerfScenario>,
    budgets: BTreeMap<String, PerfBudget>,
    allowlist: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct ScenarioMeasurement {
    id: String,
    fixture: String,
    iterations: usize,
    median_ms: f64,
    p95_ms: f64,
    max_flows: usize,
}

pub fn run(args: &[String]) -> Result<(), DynError> {
    let (mut flags, options) = parse_flags_and_options(args)?;
    let check = flags.remove("check");
    let locked = flags.remove("locked");
    ensure_no_unknown_options(&flags, &options)?;

    let root = workspace_root()?;
    let scenarios_path = root.join(SCENARIOS_FILE);
    let budgets_path = root.join(BUDGETS_FILE);
    let config = load_config(scenarios_path.as_path(), budgets_path.as_path())?;

    let results = execute_scenarios(&root, &config)?;
    print_report(&results, &config);

    if check {
        enforce_budgets(&results, &config)?;
        run_smoke_test(&root, locked)?;
        println!("perf-gate check: scenario budgets and smoke gate passed");
    } else {
        println!("perf-gate: report-only mode (use --check to enforce budgets)");
    }

    Ok(())
}

fn load_config(scenarios_path: &Path, budgets_path: &Path) -> Result<PerfConfig, DynError> {
    if !scenarios_path.is_file() {
        return Err(format!(
            "missing benchmark scenarios file '{}'",
            scenarios_path.display()
        )
        .into());
    }
    if !budgets_path.is_file() {
        return Err(format!(
            "missing benchmark budgets file '{}'",
            budgets_path.display()
        )
        .into());
    }

    let scenarios_text = read_text_file(scenarios_path)?;
    let budgets_text = read_text_file(budgets_path)?;
    let scenarios_toml: Value = toml::from_str(&scenarios_text)?;
    let budgets_toml: Value = toml::from_str(&budgets_text)?;

    let scenarios_table = scenarios_toml
        .as_table()
        .ok_or("benchmarks/scenarios.toml must be a TOML table")?;
    let runner = scenarios_table
        .get("runner")
        .and_then(Value::as_table)
        .ok_or("benchmarks/scenarios.toml is missing [runner]")?;

    let warmup_runs = parse_positive_usize(runner, "warmup_runs", "runner")?;
    let sample_runs = parse_positive_usize(runner, "sample_runs", "runner")?;

    let raw_scenarios = scenarios_table
        .get("scenario")
        .and_then(Value::as_array)
        .ok_or("benchmarks/scenarios.toml must declare one or more [[scenario]] entries")?;
    if raw_scenarios.is_empty() {
        return Err("benchmarks/scenarios.toml has no [[scenario]] entries".into());
    }

    let mut scenarios = Vec::<PerfScenario>::new();
    let mut seen_ids = BTreeSet::<String>::new();
    for (idx, raw) in raw_scenarios.iter().enumerate() {
        let section = format!("scenario[{idx}]");
        let table = raw
            .as_table()
            .ok_or_else(|| format!("{section} must be a table"))?;
        let id = parse_non_empty_string(table, "id", &section)?;
        if !seen_ids.insert(id.clone()) {
            return Err(format!("duplicate benchmark scenario id '{id}'").into());
        }
        let fixture = parse_non_empty_string(table, "fixture", &section)?;
        let expected_min_flows =
            parse_non_negative_usize(table, "expected_min_flows", &section)?.unwrap_or(1usize);
        let iterations = parse_non_negative_usize(table, "iterations", &section)?.unwrap_or(1);
        if iterations == 0 {
            return Err(format!("{section}.iterations must be greater than zero").into());
        }
        scenarios.push(PerfScenario {
            id,
            fixture,
            expected_min_flows,
            iterations,
        });
    }

    let budgets_table = budgets_toml
        .as_table()
        .ok_or("benchmarks/budgets.toml must be a TOML table")?;
    let budget_sections = budgets_table
        .get("budget")
        .and_then(Value::as_table)
        .ok_or("benchmarks/budgets.toml is missing [budget.<scenario_id>] entries")?;

    let mut budgets = BTreeMap::<String, PerfBudget>::new();
    for (scenario_id, raw_budget) in budget_sections {
        let section = format!("budget.{scenario_id}");
        let table = raw_budget
            .as_table()
            .ok_or_else(|| format!("{section} must be a table"))?;
        let median_ms = parse_positive_f64(table, "median_ms", &section)?;
        let p95_ms = parse_positive_f64(table, "p95_ms", &section)?;
        budgets.insert(scenario_id.clone(), PerfBudget { median_ms, p95_ms });
    }

    let policy_table = budgets_table
        .get("policy")
        .and_then(Value::as_table)
        .cloned()
        .unwrap_or_default();
    let noise_percent = if policy_table.is_empty() {
        3.0
    } else {
        parse_positive_f64(&policy_table, "noise_percent", "policy")?
    };
    let hard_fail_percent = if policy_table.is_empty() {
        8.0
    } else {
        parse_positive_f64(&policy_table, "hard_fail_percent", "policy")?
    };
    if noise_percent >= hard_fail_percent {
        return Err(format!(
            "policy.noise_percent ({noise_percent}) must be lower than policy.hard_fail_percent ({hard_fail_percent})"
        )
        .into());
    }

    let mut allowlist = BTreeSet::<String>::new();
    if let Some(table) = budgets_table.get("allowlist").and_then(Value::as_table)
        && let Some(items) = table.get("scenario_ids")
    {
        let array = items
            .as_array()
            .ok_or("allowlist.scenario_ids must be an array of scenario IDs")?;
        for item in array {
            let id = item
                .as_str()
                .ok_or("allowlist.scenario_ids entries must be strings")?
                .trim();
            if id.is_empty() {
                return Err("allowlist.scenario_ids entries cannot be empty".into());
            }
            allowlist.insert(id.to_string());
        }
    }

    let scenario_ids = scenarios
        .iter()
        .map(|scenario| scenario.id.clone())
        .collect::<BTreeSet<_>>();
    let budget_ids = budgets.keys().cloned().collect::<BTreeSet<_>>();

    let missing_budgets = scenario_ids
        .difference(&budget_ids)
        .cloned()
        .collect::<Vec<_>>();
    if !missing_budgets.is_empty() {
        return Err(format!(
            "benchmarks/budgets.toml is missing budget entries for scenario IDs: {}",
            missing_budgets.join(", ")
        )
        .into());
    }

    let extra_budgets = budget_ids
        .difference(&scenario_ids)
        .cloned()
        .collect::<Vec<_>>();
    if !extra_budgets.is_empty() {
        return Err(format!(
            "benchmarks/budgets.toml has unknown budget entries: {}",
            extra_budgets.join(", ")
        )
        .into());
    }

    let unknown_allowlist_ids = allowlist
        .difference(&scenario_ids)
        .cloned()
        .collect::<Vec<_>>();
    if !unknown_allowlist_ids.is_empty() {
        return Err(format!(
            "allowlist references unknown scenario IDs: {}",
            unknown_allowlist_ids.join(", ")
        )
        .into());
    }

    Ok(PerfConfig {
        warmup_runs,
        sample_runs,
        noise_percent,
        hard_fail_percent,
        scenarios,
        budgets,
        allowlist,
    })
}

fn execute_scenarios(
    root: &Path,
    config: &PerfConfig,
) -> Result<Vec<ScenarioMeasurement>, DynError> {
    let mut measurements = Vec::<ScenarioMeasurement>::new();
    let aztec_config = AztecConfig::default();

    for scenario in &config.scenarios {
        let fixture_path = root.join(&scenario.fixture);
        if !fixture_path.is_file() {
            return Err(format!(
                "benchmark fixture '{}' for scenario '{}' does not exist",
                fixture_path.display(),
                scenario.id
            )
            .into());
        }
        let source = read_text_file(&fixture_path)?;

        for _ in 0..config.warmup_runs {
            let _ = run_once(
                &scenario.fixture,
                &source,
                &aztec_config,
                scenario.expected_min_flows,
                scenario.iterations,
            )?;
        }

        let mut samples_ms = Vec::<f64>::new();
        let mut max_flows = 0usize;
        for _ in 0..config.sample_runs {
            let (elapsed_ms, flow_count) = run_once(
                &scenario.fixture,
                &source,
                &aztec_config,
                scenario.expected_min_flows,
                scenario.iterations,
            )?;
            max_flows = max_flows.max(flow_count);
            samples_ms.push(elapsed_ms);
        }

        let (median_ms, p95_ms) = summary_metrics(&samples_ms)?;
        measurements.push(ScenarioMeasurement {
            id: scenario.id.clone(),
            fixture: scenario.fixture.clone(),
            iterations: scenario.iterations,
            median_ms,
            p95_ms,
            max_flows,
        });
    }

    Ok(measurements)
}

fn run_once(
    fixture: &str,
    source: &str,
    config: &AztecConfig,
    expected_min_flows: usize,
    iterations: usize,
) -> Result<(f64, usize), DynError> {
    let started = Instant::now();
    let mut max_flows = 0usize;
    for _ in 0..iterations {
        let sources = vec![SourceUnit::new(fixture, source)];
        let model = build_aztec_model(&sources, config);
        let graph = build_def_use_graph(&sources, &model, config);
        let analysis = analyze_intra_procedural(&graph);
        max_flows = max_flows.max(analysis.flows.len());
    }
    let elapsed_ms = started.elapsed().as_secs_f64() * 1_000.0;

    if max_flows < expected_min_flows {
        return Err(format!(
            "scenario fixture '{fixture}' produced {max_flows} taint flows; expected at least {expected_min_flows}"
        )
        .into());
    }

    Ok((elapsed_ms, max_flows))
}

fn summary_metrics(samples_ms: &[f64]) -> Result<(f64, f64), DynError> {
    if samples_ms.is_empty() {
        return Err("cannot compute benchmark summary for empty sample set".into());
    }

    let mut sorted = samples_ms.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));

    let median_ms = if sorted.len() % 2 == 1 {
        sorted[sorted.len() / 2]
    } else {
        let high = sorted.len() / 2;
        let low = high.saturating_sub(1);
        (sorted[low] + sorted[high]) / 2.0
    };

    let p95_index = ((sorted.len() as f64) * 0.95).ceil() as usize;
    let p95_index = p95_index.saturating_sub(1).min(sorted.len() - 1);
    let p95_ms = sorted[p95_index];

    Ok((median_ms, p95_ms))
}

fn print_report(results: &[ScenarioMeasurement], config: &PerfConfig) {
    println!(
        "perf-gate benchmark run: scenarios={} warmup_runs={} sample_runs={} noise_percent={:.2}% hard_fail_percent={:.2}%",
        results.len(),
        config.warmup_runs,
        config.sample_runs,
        config.noise_percent,
        config.hard_fail_percent,
    );
    for result in results {
        let budget = config
            .budgets
            .get(&result.id)
            .expect("budget existence was validated during load");
        let median_regression = regression_percent(result.median_ms, budget.median_ms);
        let p95_regression = regression_percent(result.p95_ms, budget.p95_ms);
        println!(
            "scenario={} fixture={} iterations={} median_ms={:.3} p95_ms={:.3} budget_median_ms={:.3} budget_p95_ms={:.3} median_regression_percent={:.2} p95_regression_percent={:.2} max_flows={}",
            result.id,
            result.fixture,
            result.iterations,
            result.median_ms,
            result.p95_ms,
            budget.median_ms,
            budget.p95_ms,
            median_regression,
            p95_regression,
            result.max_flows
        );
    }
}

fn enforce_budgets(results: &[ScenarioMeasurement], config: &PerfConfig) -> Result<(), DynError> {
    let mut soft_violations = Vec::<String>::new();
    let mut hard_violations = Vec::<String>::new();

    for result in results {
        let budget = config
            .budgets
            .get(&result.id)
            .expect("budget existence was validated during load");
        let median_regression = regression_percent(result.median_ms, budget.median_ms);
        let p95_regression = regression_percent(result.p95_ms, budget.p95_ms);
        let max_regression = median_regression.max(p95_regression);
        if max_regression <= config.noise_percent {
            continue;
        }

        let violation = format!(
            "scenario={} median_ms={:.3} (budget {:.3}, +{:.2}%) p95_ms={:.3} (budget {:.3}, +{:.2}%)",
            result.id,
            result.median_ms,
            budget.median_ms,
            median_regression,
            result.p95_ms,
            budget.p95_ms,
            p95_regression
        );
        if config.allowlist.contains(&result.id) {
            println!("perf-gate allowlisted regression: {violation}");
        } else if max_regression > config.hard_fail_percent {
            hard_violations.push(violation);
        } else {
            soft_violations.push(violation);
        }
    }

    if soft_violations.is_empty() && hard_violations.is_empty() {
        return Ok(());
    }

    let mut message = String::new();
    if !soft_violations.is_empty() {
        message.push_str(&format!(
            "perf-gate regressions above noise threshold ({:.2}%):\n{}\n",
            config.noise_percent,
            soft_violations.join("\n")
        ));
    }
    if !hard_violations.is_empty() {
        message.push_str(&format!(
            "perf-gate hard regressions above block threshold ({:.2}%):\n{}\n",
            config.hard_fail_percent,
            hard_violations.join("\n")
        ));
    }
    message.push_str(
        "Add intentional slowdowns to benchmarks/budgets.toml [allowlist].scenario_ids with rationale, or re-baseline budgets with reviewer sign-off.",
    );
    Err(message.into())
}

fn run_smoke_test(root: &Path, locked: bool) -> Result<(), DynError> {
    let mut smoke = Command::new("cargo");
    smoke
        .arg("test")
        .arg("-p")
        .arg("aztec-lint-aztec")
        .arg("performance_smoke_stays_bounded");
    if locked {
        smoke.arg("--locked");
    }
    smoke.current_dir(root);
    run_command(&mut smoke)
}

fn parse_non_empty_string(
    table: &toml::map::Map<String, Value>,
    key: &str,
    section: &str,
) -> Result<String, DynError> {
    let value = table
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{section}.{key} must be a string"))?
        .trim()
        .to_string();
    if value.is_empty() {
        return Err(format!("{section}.{key} cannot be empty").into());
    }
    Ok(value)
}

fn parse_positive_usize(
    table: &toml::map::Map<String, Value>,
    key: &str,
    section: &str,
) -> Result<usize, DynError> {
    let raw = table
        .get(key)
        .and_then(Value::as_integer)
        .ok_or_else(|| format!("{section}.{key} must be a positive integer"))?;
    let value = usize::try_from(raw)
        .map_err(|_| format!("{section}.{key} must be a non-negative integer"))?;
    if value == 0 {
        return Err(format!("{section}.{key} must be greater than zero").into());
    }
    Ok(value)
}

fn parse_non_negative_usize(
    table: &toml::map::Map<String, Value>,
    key: &str,
    section: &str,
) -> Result<Option<usize>, DynError> {
    let Some(raw) = table.get(key) else {
        return Ok(None);
    };
    let value = raw
        .as_integer()
        .ok_or_else(|| format!("{section}.{key} must be an integer"))?;
    let value = usize::try_from(value)
        .map_err(|_| format!("{section}.{key} must be a non-negative integer"))?;
    Ok(Some(value))
}

fn parse_positive_f64(
    table: &toml::map::Map<String, Value>,
    key: &str,
    section: &str,
) -> Result<f64, DynError> {
    let Some(value) = table.get(key) else {
        return Err(format!("{section}.{key} is required").into());
    };
    let parsed = if let Some(float_value) = value.as_float() {
        float_value
    } else if let Some(int_value) = value.as_integer() {
        int_value as f64
    } else {
        return Err(format!("{section}.{key} must be numeric").into());
    };
    if parsed <= 0.0 {
        return Err(format!("{section}.{key} must be greater than zero").into());
    }
    Ok(parsed)
}

fn regression_percent(observed: f64, baseline: f64) -> f64 {
    if observed <= baseline {
        return 0.0;
    }
    ((observed - baseline) / baseline) * 100.0
}
