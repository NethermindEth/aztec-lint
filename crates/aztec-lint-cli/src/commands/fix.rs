use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::fix::{FixApplicationMode, FixApplicationReport, apply_fixes};
use aztec_lint_core::output::json as json_output;
use aztec_lint_core::output::sarif as sarif_output;
use aztec_lint_core::output::text::{CheckTextReport, render_check_report};
use clap::Args;

use crate::cli::{CliError, CommonLintFlags, OutputFormat};
use crate::commands::check::{
    collect_lint_run, has_blocking_diagnostics, passes_thresholds, suppression_visible,
};
use crate::exit_codes;

#[derive(Clone, Debug, Args)]
pub struct FixArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(long, default_value = "default")]
    pub profile: String,
    #[arg(long)]
    pub changed_only: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    pub lint: CommonLintFlags,
}

pub fn run(args: FixArgs) -> Result<ExitCode, CliError> {
    let initial = collect_lint_run(
        args.path.as_path(),
        &args.profile,
        args.changed_only,
        args.lint.rule_overrides(),
    )?;

    let fix_mode = if args.dry_run {
        FixApplicationMode::DryRun
    } else {
        FixApplicationMode::Apply
    };
    let candidates = diagnostics_for_fix(
        &initial.diagnostics,
        args.lint.min_confidence,
        args.lint.severity_threshold,
    );
    let fix_report = apply_fixes(initial.report_root.as_path(), &candidates, fix_mode)
        .map_err(|source| CliError::Runtime(format!("failed to apply fixes: {source}")))?;

    let should_rerun_after_fix = !args.dry_run && !fix_report.selected.is_empty();
    let final_run = if !should_rerun_after_fix {
        initial.clone()
    } else {
        collect_lint_run(
            args.path.as_path(),
            &args.profile,
            args.changed_only,
            args.lint.rule_overrides(),
        )?
    };

    let include_suppressed = suppression_visible(args.lint.format, args.lint.show_suppressed);
    let diagnostics = diagnostics_for_output(
        &final_run.diagnostics,
        args.lint.min_confidence,
        args.lint.severity_threshold,
        include_suppressed,
    );

    render_fix_result(FixRenderContext {
        format: args.lint.format,
        path: args.path.as_path(),
        profile: &args.profile,
        changed_only: args.changed_only,
        dry_run: args.dry_run,
        effective_rules: final_run.effective_rules,
        diagnostics: &diagnostics,
        sarif_root: final_run.report_root.as_path(),
        fix_report: &fix_report,
    })?;

    let blocking = has_blocking_diagnostics(
        &final_run.diagnostics,
        args.lint.min_confidence,
        args.lint.severity_threshold,
    );
    Ok(exit_codes::diagnostics_found(blocking))
}

fn diagnostics_for_fix(
    diagnostics: &[Diagnostic],
    min_confidence: crate::cli::MinConfidence,
    severity_threshold: crate::cli::SeverityThreshold,
) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| {
            !diagnostic.suppressed
                && passes_thresholds(diagnostic, min_confidence, severity_threshold)
        })
        .cloned()
        .collect()
}

fn diagnostics_for_output(
    diagnostics: &[Diagnostic],
    min_confidence: crate::cli::MinConfidence,
    severity_threshold: crate::cli::SeverityThreshold,
    include_suppressed: bool,
) -> Vec<&Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| {
            if diagnostic.suppressed {
                return include_suppressed;
            }
            passes_thresholds(diagnostic, min_confidence, severity_threshold)
        })
        .collect()
}

struct FixRenderContext<'a> {
    format: OutputFormat,
    path: &'a Path,
    profile: &'a str,
    changed_only: bool,
    dry_run: bool,
    effective_rules: usize,
    diagnostics: &'a [&'a Diagnostic],
    sarif_root: &'a Path,
    fix_report: &'a FixApplicationReport,
}

fn render_fix_result(context: FixRenderContext<'_>) -> Result<(), CliError> {
    match context.format {
        OutputFormat::Text => {
            let mode_label = if context.dry_run { "dry-run" } else { "apply" };
            println!(
                "fix path={} profile={} changed_only={} mode={} active_rules={effective_rules}",
                context.path.display(),
                context.profile,
                context.changed_only,
                mode_label,
                effective_rules = context.effective_rules
            );
            println!(
                "fixes_total={} fixes_selected={} fixes_skipped={} files_changed={}",
                context.fix_report.total_candidates,
                context.fix_report.selected.len(),
                context.fix_report.skipped.len(),
                context.fix_report.files_changed,
            );

            let rendered = render_check_report(CheckTextReport {
                path: context.path,
                source_root: context.sarif_root,
                show_run_header: false,
                profile: context.profile,
                changed_only: context.changed_only,
                active_rules: context.effective_rules,
                diagnostics: context.diagnostics,
            });
            print!("{rendered}");
            Ok(())
        }
        OutputFormat::Json => {
            let rendered =
                json_output::render_diagnostics(context.diagnostics).map_err(|source| {
                    CliError::Runtime(format!(
                        "failed to serialize fix diagnostics as JSON: {source}"
                    ))
                })?;
            println!("{rendered}");
            Ok(())
        }
        OutputFormat::Sarif => {
            let rendered =
                sarif_output::render_diagnostics(context.sarif_root, context.diagnostics).map_err(
                    |source| {
                        CliError::Runtime(format!(
                            "failed to serialize fix diagnostics as SARIF: {source}"
                        ))
                    },
                )?;
            println!("{rendered}");
            Ok(())
        }
    }
}
