use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aztec_lint_core::config::load_from_dir;
use clap::Args;

use crate::cli::{CliError, CommonLintFlags, OutputFormat};
use crate::commands::check::config_root_for_target;

#[derive(Clone, Debug, Args)]
pub struct FixArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(long, default_value = "default")]
    pub profile: String,
    #[arg(long)]
    pub changed_only: bool,
    #[command(flatten)]
    pub lint: CommonLintFlags,
}

pub fn run(args: FixArgs) -> Result<ExitCode, CliError> {
    let loaded = load_from_dir(config_root_for_target(args.path.as_path()))?;
    let effective_rules = loaded
        .config
        .effective_rule_levels(&args.profile, &args.lint.rule_overrides())?;

    render_fix_result(
        args.lint.format,
        args.path.as_path(),
        &args.profile,
        args.changed_only,
        effective_rules.len(),
    );
    Ok(ExitCode::from(0))
}

fn render_fix_result(
    format: OutputFormat,
    path: &Path,
    profile: &str,
    changed_only: bool,
    effective_rules: usize,
) {
    match format {
        OutputFormat::Text => {
            println!(
                "fix path={} profile={} changed_only={} active_rules={effective_rules}",
                path.display(),
                profile,
                changed_only
            );
            println!("No fixes applied.");
        }
        OutputFormat::Json => {
            println!("[]");
        }
        OutputFormat::Sarif => {
            println!(r#"{{"version":"2.1.0","runs":[]}}"#);
        }
    }
}
