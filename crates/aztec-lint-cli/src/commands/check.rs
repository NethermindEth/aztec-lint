use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aztec_lint_aztec::{SourceUnit, build_aztec_model, should_activate_aztec};
use aztec_lint_core::config::load_from_dir;
use aztec_lint_core::diagnostics::{
    Confidence, Diagnostic, Severity, normalize_file_path, sort_diagnostics,
};
use aztec_lint_core::noir::build_project_model;
use aztec_lint_core::output::json as json_output;
use aztec_lint_core::output::sarif as sarif_output;
use aztec_lint_core::output::text::{CheckTextReport, render_check_report};
use aztec_lint_rules::RuleEngine;
use aztec_lint_rules::engine::context::RuleContext;
use clap::Args;

use crate::cli::{CliError, CommonLintFlags, MinConfidence, OutputFormat, SeverityThreshold};
use crate::exit_codes;

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

#[derive(Clone, Debug)]
struct NoirProject {
    root: PathBuf,
    entry: PathBuf,
}

pub fn run(args: CheckArgs) -> Result<ExitCode, CliError> {
    let loaded = load_from_dir(config_root_for_target(args.path.as_path()))?;
    let effective_rules = loaded
        .config
        .effective_rule_levels(&args.profile, &args.lint.rule_overrides())?;

    let projects = discover_noir_projects(args.path.as_path()).map_err(|source| {
        CliError::Runtime(format!(
            "failed to discover Noir projects under '{}': {source}",
            args.path.display()
        ))
    })?;

    if projects.is_empty() {
        return Err(CliError::Runtime(format!(
            "no Noir project found under '{}'",
            args.path.display()
        )));
    }
    let report_root = report_root_for_target(args.path.as_path(), &projects);

    let engine = RuleEngine::new();
    let mut diagnostics = Vec::<Diagnostic>::new();

    for project in projects {
        let model = build_project_model(&project.root, &project.entry).map_err(|source| {
            CliError::Runtime(format!(
                "failed to build Noir model for '{}' (entry '{}'): {source}",
                project.root.display(),
                project.entry.display()
            ))
        })?;
        let mut context =
            RuleContext::from_project_root(&project.root, &model).map_err(|source| {
                CliError::Runtime(format!(
                    "failed to read Noir sources for '{}': {source}",
                    project.root.display()
                ))
            })?;
        context.set_aztec_config(loaded.config.aztec.clone());

        let sources = context
            .files()
            .iter()
            .map(|file| SourceUnit::new(file.path().to_string(), file.text().to_string()))
            .collect::<Vec<_>>();
        if should_activate_aztec(&args.profile, &sources, &loaded.config.aztec) {
            let aztec_model = build_aztec_model(&sources, &loaded.config.aztec);
            context.set_aztec_model(aztec_model);
        }

        let mut project_diagnostics = engine.run(&context, &effective_rules);
        rebase_diagnostic_paths(
            &mut project_diagnostics,
            project.root.as_path(),
            report_root.as_path(),
        );
        diagnostics.extend(project_diagnostics);
    }

    sort_diagnostics(&mut diagnostics);

    let filtered = filter_diagnostics(
        &diagnostics,
        args.lint.min_confidence,
        args.lint.severity_threshold,
    );

    render_result(
        args.lint.format,
        args.path.as_path(),
        &args.profile,
        args.changed_only,
        effective_rules.len(),
        &filtered,
        report_root.as_path(),
    )?;

    Ok(exit_codes::diagnostics_found(!filtered.is_empty()))
}

pub(crate) fn config_root_for_target(path: &Path) -> &Path {
    if path.exists() && path.is_file() {
        return path.parent().unwrap_or(Path::new("."));
    }
    path
}

fn render_result(
    format: OutputFormat,
    path: &Path,
    profile: &str,
    changed_only: bool,
    effective_rules: usize,
    diagnostics: &[&Diagnostic],
    sarif_root: &Path,
) -> Result<(), CliError> {
    match format {
        OutputFormat::Text => {
            let rendered = render_check_report(CheckTextReport {
                path,
                profile,
                changed_only,
                active_rules: effective_rules,
                diagnostics,
            });
            print!("{rendered}");
            Ok(())
        }
        OutputFormat::Json => {
            let rendered = json_output::render_diagnostics(diagnostics).map_err(|source| {
                CliError::Runtime(format!("failed to serialize diagnostics as JSON: {source}"))
            })?;
            println!("{rendered}");
            Ok(())
        }
        OutputFormat::Sarif => {
            let rendered =
                sarif_output::render_diagnostics(sarif_root, diagnostics).map_err(|source| {
                    CliError::Runtime(format!(
                        "failed to serialize diagnostics as SARIF: {source}"
                    ))
                })?;
            println!("{rendered}");
            Ok(())
        }
    }
}

fn report_root_for_target(path: &Path, projects: &[NoirProject]) -> PathBuf {
    if path.is_file()
        && let Some(project) = projects.first()
    {
        return project.root.clone();
    }

    config_root_for_target(path)
        .canonicalize()
        .unwrap_or_else(|_| config_root_for_target(path).to_path_buf())
}

fn rebase_diagnostic_paths(
    diagnostics: &mut [Diagnostic],
    project_root: &Path,
    report_root: &Path,
) {
    for diagnostic in diagnostics {
        diagnostic.primary_span.file =
            rebase_file_path(&diagnostic.primary_span.file, project_root, report_root);

        for span in &mut diagnostic.secondary_spans {
            span.file = rebase_file_path(&span.file, project_root, report_root);
        }

        for fix in &mut diagnostic.fixes {
            fix.span.file = rebase_file_path(&fix.span.file, project_root, report_root);
        }
    }
}

fn rebase_file_path(file: &str, project_root: &Path, report_root: &Path) -> String {
    let file_path = Path::new(file);
    let absolute_path = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        project_root.join(file_path)
    };
    let rebased = absolute_path
        .strip_prefix(report_root)
        .unwrap_or(absolute_path.as_path());
    normalize_file_path(&rebased.to_string_lossy())
}

fn filter_diagnostics(
    diagnostics: &[Diagnostic],
    min_confidence: MinConfidence,
    severity_threshold: SeverityThreshold,
) -> Vec<&Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| !diagnostic.suppressed)
        .filter(|diagnostic| {
            confidence_rank(diagnostic.confidence) >= min_confidence_rank(min_confidence)
        })
        .filter(|diagnostic| {
            severity_rank(diagnostic.severity) >= severity_threshold_rank(severity_threshold)
        })
        .collect()
}

fn confidence_rank(confidence: Confidence) -> u8 {
    match confidence {
        Confidence::Low => 1,
        Confidence::Medium => 2,
        Confidence::High => 3,
    }
}

fn min_confidence_rank(confidence: MinConfidence) -> u8 {
    match confidence {
        MinConfidence::Low => 1,
        MinConfidence::Medium => 2,
        MinConfidence::High => 3,
    }
}

fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Warning => 1,
        Severity::Error => 2,
    }
}

fn severity_threshold_rank(threshold: SeverityThreshold) -> u8 {
    match threshold {
        SeverityThreshold::Warning => 1,
        SeverityThreshold::Error => 2,
    }
}

fn discover_noir_projects(target: &Path) -> std::io::Result<Vec<NoirProject>> {
    let mut roots = Vec::<PathBuf>::new();

    if target.is_file() {
        if target
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "Nargo.toml")
        {
            roots.push(target.parent().unwrap_or(Path::new(".")).to_path_buf());
        } else if target
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("nr"))
            && let Some(root) = nearest_project_root(target.parent().unwrap_or(Path::new(".")))
        {
            roots.push(root);
        }
    } else if let Some(root) = nearest_project_root(target) {
        roots.push(root);
    } else {
        collect_project_roots(target, &mut roots)?;
    }

    roots.sort();
    roots.dedup();

    let canonical_roots = roots
        .into_iter()
        .filter_map(|root| root.canonicalize().ok())
        .collect::<Vec<_>>();

    Ok(canonical_roots
        .into_iter()
        .filter_map(|root| {
            select_entry_file(&root)
                .and_then(|entry| entry.canonicalize().ok())
                .map(|entry| NoirProject { root, entry })
        })
        .collect())
}

fn nearest_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(path) = current {
        if path.join("Nargo.toml").is_file() {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}

fn collect_project_roots(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.join("Nargo.toml").is_file() {
        out.push(dir.to_path_buf());
        return Ok(());
    }

    let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_project_roots(&path, out)?;
        }
    }

    Ok(())
}

fn select_entry_file(root: &Path) -> Option<PathBuf> {
    let main = root.join("src/main.nr");
    if main.is_file() {
        return Some(main);
    }

    let lib = root.join("src/lib.nr");
    if lib.is_file() {
        return Some(lib);
    }

    let mut candidates = Vec::<PathBuf>::new();
    collect_noir_sources(&root.join("src"), &mut candidates).ok()?;
    candidates.sort();
    candidates.into_iter().next()
}

fn collect_noir_sources(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(());
    }

    let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_noir_sources(&path, out)?;
            continue;
        }
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("nr"))
        {
            out.push(path);
        }
    }

    Ok(())
}
