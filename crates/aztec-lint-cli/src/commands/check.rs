use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aztec_lint_core::config::load_from_dir;
use clap::Args;

use crate::cli::{CliError, CommonLintFlags, OutputFormat};

#[derive(Clone, Debug, Args)]
pub struct CheckArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(long, default_value = "default")]
    pub profile: String,
    #[arg(long)]
    pub changed_only: bool,
    #[command(flatten)]
    pub lint: CommonLintFlags,
}

pub fn run(args: CheckArgs) -> Result<ExitCode, CliError> {
    let loaded = load_from_dir(config_root_for_target(args.path.as_path()))?;
    let effective_rules = loaded
        .config
        .effective_rule_levels(&args.profile, &args.lint.rule_overrides())?;

    render_empty_result(
        args.lint.format,
        args.path.as_path(),
        &args.profile,
        args.changed_only,
        effective_rules.len(),
    );
    Ok(ExitCode::from(0))
}

pub(crate) fn config_root_for_target(path: &Path) -> &Path {
    if path.exists() && path.is_file() {
        return path.parent().unwrap_or(Path::new("."));
    }
    path
}

pub fn render_empty_result(
    format: OutputFormat,
    path: &Path,
    profile: &str,
    changed_only: bool,
    effective_rules: usize,
) {
    match format {
        OutputFormat::Text => {
            println!(
                "checked={} profile={} changed_only={} active_rules={effective_rules}",
                path.display(),
                profile,
                changed_only
            );
            println!("No diagnostics.");
        }
        OutputFormat::Json => {
            println!("[]");
        }
        OutputFormat::Sarif => {
            println!(r#"{{"version":"2.1.0","runs":[]}}"#);
        }
    }
}
