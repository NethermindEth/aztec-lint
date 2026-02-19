use std::path::PathBuf;
use std::process::ExitCode;

use clap::Args;

use crate::cli::{CliError, CommonLintFlags};
use crate::commands::check::{CheckArgs, run as run_check};

#[derive(Clone, Debug, Args)]
pub struct AztecScanArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(long)]
    pub changed_only: bool,
    #[command(flatten)]
    pub lint: CommonLintFlags,
}

pub fn run(args: AztecScanArgs) -> Result<ExitCode, CliError> {
    run_check(CheckArgs {
        path: args.path,
        profile: "aztec".to_string(),
        changed_only: args.changed_only,
        lint: args.lint,
    })
}
