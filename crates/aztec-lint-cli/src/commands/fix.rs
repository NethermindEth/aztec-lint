use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::fix::{
    FixApplicationMode, FixApplicationReport, FixSource, SkippedFixReason, apply_fixes,
};
use aztec_lint_core::output::json as json_output;
use aztec_lint_core::output::sarif as sarif_output;
use aztec_lint_core::output::text::{CheckTextReport, render_check_report};
use clap::Args;

use crate::cli::{CliError, CommonLintFlags, OutputFormat, TargetSelectionFlags};
use crate::commands::check::{
    collect_lint_run, diagnostics_for_text_display, has_blocking_diagnostics, passes_thresholds,
    suppression_visible, text_display_root,
};
use crate::exit_codes;

#[derive(Clone, Debug, Args)]
pub struct FixArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(long, default_value = "aztec")]
    pub profile: String,
    #[arg(long)]
    pub changed_only: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    pub targets: TargetSelectionFlags,
    #[command(flatten)]
    pub lint: CommonLintFlags,
}

pub fn run(args: FixArgs) -> Result<ExitCode, CliError> {
    let initial = collect_lint_run(
        args.path.as_path(),
        &args.profile,
        args.changed_only,
        args.targets.resolve(),
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
            args.targets.resolve(),
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
            let (selected_explicit, selected_structured) =
                source_breakdown_selected(context.fix_report);
            let (skipped_explicit, skipped_structured) =
                source_breakdown_skipped(context.fix_report);
            println!(
                "fixes_selected_explicit={} fixes_selected_structured={} fixes_skipped_explicit={} fixes_skipped_structured={}",
                selected_explicit, selected_structured, skipped_explicit, skipped_structured,
            );
            let (
                skipped_suppressed,
                skipped_unsafe,
                skipped_mixed_file,
                skipped_overlap,
                skipped_invalid_span,
                skipped_noop,
            ) = skipped_reason_breakdown(context.fix_report);
            println!(
                "fixes_skipped_suppressed={} fixes_skipped_unsafe={} fixes_skipped_mixed_file={} fixes_skipped_overlap={} fixes_skipped_invalid_span={} fixes_skipped_noop={}",
                skipped_suppressed,
                skipped_unsafe,
                skipped_mixed_file,
                skipped_overlap,
                skipped_invalid_span,
                skipped_noop,
            );

            for selected in &context.fix_report.selected {
                println!(
                    "fix_selected rule={} source={} group={} edits={} file={} span={}..{} provenance={}",
                    selected.rule_id,
                    source_label(selected.source),
                    selected.group_id,
                    selected.edit_count,
                    selected.file,
                    selected.start,
                    selected.end,
                    selected.provenance.as_deref().unwrap_or("-"),
                );
            }

            for skipped in &context.fix_report.skipped {
                println!(
                    "fix_skipped rule={} source={} group={} edits={} file={} span={}..{} reason={} provenance={}",
                    skipped.rule_id,
                    source_label(skipped.source),
                    skipped.group_id,
                    skipped.edit_count,
                    skipped.file,
                    skipped.start,
                    skipped.end,
                    skipped_reason_label(skipped.reason),
                    skipped.provenance.as_deref().unwrap_or("-"),
                );
            }

            let display_root = text_display_root(context.path, context.sarif_root);
            let diagnostics = diagnostics_for_text_display(
                context.diagnostics,
                context.sarif_root,
                display_root.as_path(),
            );
            let diagnostic_refs = diagnostics.iter().collect::<Vec<_>>();
            let rendered = render_check_report(CheckTextReport {
                path: context.path,
                source_root: display_root.as_path(),
                show_run_header: false,
                profile: context.profile,
                changed_only: context.changed_only,
                active_rules: context.effective_rules,
                diagnostics: &diagnostic_refs,
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

fn source_breakdown_selected(report: &FixApplicationReport) -> (usize, usize) {
    let explicit = report
        .selected
        .iter()
        .filter(|selected| selected.source == FixSource::ExplicitFix)
        .count();
    let structured = report.selected.len().saturating_sub(explicit);
    (explicit, structured)
}

fn source_breakdown_skipped(report: &FixApplicationReport) -> (usize, usize) {
    let explicit = report
        .skipped
        .iter()
        .filter(|skipped| skipped.source == FixSource::ExplicitFix)
        .count();
    let structured = report.skipped.len().saturating_sub(explicit);
    (explicit, structured)
}

fn source_label(source: FixSource) -> &'static str {
    match source {
        FixSource::ExplicitFix => "explicit_fix",
        FixSource::StructuredSuggestion => "structured_suggestion",
    }
}

fn skipped_reason_label(reason: SkippedFixReason) -> &'static str {
    match reason {
        SkippedFixReason::SuppressedDiagnostic => "suppressed_diagnostic",
        SkippedFixReason::UnsafeFix => "unsafe_fix",
        SkippedFixReason::MixedFileGroup => "mixed_file_group",
        SkippedFixReason::GroupOverlap => "group_overlap",
        SkippedFixReason::InvalidGroupSpan => "invalid_group_span",
        SkippedFixReason::GroupNoop => "group_noop",
    }
}

fn skipped_reason_breakdown(
    report: &FixApplicationReport,
) -> (usize, usize, usize, usize, usize, usize) {
    let mut suppressed = 0usize;
    let mut unsafe_fix = 0usize;
    let mut mixed_file = 0usize;
    let mut overlap = 0usize;
    let mut invalid_span = 0usize;
    let mut noop = 0usize;

    for skipped in &report.skipped {
        match skipped.reason {
            SkippedFixReason::SuppressedDiagnostic => suppressed += 1,
            SkippedFixReason::UnsafeFix => unsafe_fix += 1,
            SkippedFixReason::MixedFileGroup => mixed_file += 1,
            SkippedFixReason::GroupOverlap => overlap += 1,
            SkippedFixReason::InvalidGroupSpan => invalid_span += 1,
            SkippedFixReason::GroupNoop => noop += 1,
        }
    }

    (
        suppressed,
        unsafe_fix,
        mixed_file,
        overlap,
        invalid_span,
        noop,
    )
}
