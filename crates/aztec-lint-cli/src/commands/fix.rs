use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aztec_lint_core::config::load_from_dir;
use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::output::json as json_output;
use aztec_lint_core::output::sarif as sarif_output;
use clap::Args;

use crate::cli::{CliError, CommonLintFlags, OutputFormat};
use crate::commands::check::config_root_for_target;
use crate::exit_codes;

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
    )?;
    Ok(exit_codes::success())
}

fn render_fix_result(
    format: OutputFormat,
    path: &Path,
    profile: &str,
    changed_only: bool,
    effective_rules: usize,
) -> Result<(), CliError> {
    let diagnostics = Vec::<&Diagnostic>::new();

    match format {
        OutputFormat::Text => {
            println!(
                "fix path={} profile={} changed_only={} active_rules={effective_rules}",
                path.display(),
                profile,
                changed_only
            );
            println!("No fixes applied.");
            Ok(())
        }
        OutputFormat::Json => {
            let rendered = json_output::render_diagnostics(&diagnostics).map_err(|source| {
                CliError::Runtime(format!(
                    "failed to serialize fix diagnostics as JSON: {source}"
                ))
            })?;
            println!("{rendered}");
            Ok(())
        }
        OutputFormat::Sarif => {
            let rendered =
                sarif_output::render_diagnostics(config_root_for_target(path), &diagnostics)
                    .map_err(|source| {
                        CliError::Runtime(format!(
                            "failed to serialize fix diagnostics as SARIF: {source}"
                        ))
                    })?;
            println!("{rendered}");
            Ok(())
        }
    }
}
