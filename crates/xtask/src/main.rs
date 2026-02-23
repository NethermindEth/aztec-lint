mod common;
mod docs_portal;
mod lint_intake;
mod new_lint;
mod perf_gate;
mod update_lints;

use std::env;
use std::process::ExitCode;

use common::DynError;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("xtask error: {err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), DynError> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_usage();
        return Err("missing command".into());
    }

    let command = args.remove(0);
    match command.as_str() {
        "new-lint" => new_lint::run(&args),
        "update-lints" => update_lints::run(&args),
        "docs-portal" => docs_portal::run(&args),
        "perf-gate" => perf_gate::run(&args),
        "lint-intake" => lint_intake::run(&args),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => Err(format!("unknown command '{other}'").into()),
    }
}

fn print_usage() {
    eprintln!(
        "xtask usage:\n\
  cargo xtask new-lint --id <RULE_ID> --pack <PACK> --category <CATEGORY> --tier <TIER> [--policy <POLICY>] [--dry-run]\n\
  cargo xtask update-lints [--check] [--locked]\n\
  cargo xtask lint-intake --source docs/NEW_LINTS.md [--check]\n\
  cargo xtask docs-portal [--check]\n\
  cargo xtask perf-gate [--check] [--locked]"
    );
}
