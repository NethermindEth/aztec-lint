use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aztec_lint_aztec::{SourceUnit, build_aztec_model, should_activate_aztec};
use aztec_lint_core::config::{RuleOverrides, load_from_dir};
use aztec_lint_core::diagnostics::{
    Confidence, Diagnostic, Severity, normalize_file_path, sort_diagnostics,
};
use aztec_lint_core::noir::build_project_model;
use aztec_lint_core::output::json as json_output;
use aztec_lint_core::output::sarif as sarif_output;
use aztec_lint_core::output::text::{CheckTextReport, render_check_report};
use aztec_lint_core::vcs::changed_files_from_git;
use aztec_lint_rules::RuleEngine;
use aztec_lint_rules::engine::context::RuleContext;
use clap::Args;
use toml::Value as TomlValue;

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

#[derive(Clone, Debug)]
pub(crate) struct LintRun {
    pub effective_rules: usize,
    pub diagnostics: Vec<Diagnostic>,
    pub report_root: PathBuf,
}

pub fn run(args: CheckArgs) -> Result<ExitCode, CliError> {
    let lint_run = collect_lint_run(
        args.path.as_path(),
        &args.profile,
        args.changed_only,
        args.lint.rule_overrides(),
    )?;

    let include_suppressed = suppression_visible(args.lint.format, args.lint.show_suppressed);
    let diagnostics = diagnostics_for_output(
        &lint_run.diagnostics,
        args.lint.min_confidence,
        args.lint.severity_threshold,
        include_suppressed,
    );

    render_result(
        args.lint.format,
        args.path.as_path(),
        &args.profile,
        args.changed_only,
        lint_run.effective_rules,
        &diagnostics,
        lint_run.report_root.as_path(),
    )?;

    let blocking = has_blocking_diagnostics(
        &lint_run.diagnostics,
        args.lint.min_confidence,
        args.lint.severity_threshold,
    );
    Ok(exit_codes::diagnostics_found(blocking))
}

pub(crate) fn collect_lint_run(
    path: &Path,
    profile: &str,
    changed_only: bool,
    rule_overrides: RuleOverrides,
) -> Result<LintRun, CliError> {
    let loaded = load_from_dir(config_root_for_target(path))?;
    let effective_rules = loaded
        .config
        .effective_rule_levels(profile, &rule_overrides)?;

    let projects = discover_noir_projects(path).map_err(|source| {
        CliError::Runtime(format!(
            "failed to discover Noir projects under '{}': {source}",
            path.display()
        ))
    })?;

    if projects.is_empty() {
        return Err(CliError::Runtime(format!(
            "no Noir project found under '{}'",
            path.display()
        )));
    }
    let report_root = report_root_for_target(path, &projects);

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
        if should_activate_aztec(profile, &sources, &loaded.config.aztec) {
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

    if changed_only {
        let changed = changed_files_from_git(path).map_err(|source| {
            CliError::Runtime(format!(
                "failed to compute changed files for '{}': {source}",
                path.display()
            ))
        })?;
        let changed_files = changed.files_for_root(report_root.as_path());
        retain_changed_only(&mut diagnostics, &changed_files);
    }

    Ok(LintRun {
        effective_rules: effective_rules.len(),
        diagnostics,
        report_root,
    })
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
                source_root: sarif_root,
                show_run_header: true,
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

        for note in &mut diagnostic.notes {
            if let Some(span) = &mut note.span {
                span.file = rebase_file_path(&span.file, project_root, report_root);
            }
        }

        for help in &mut diagnostic.helps {
            if let Some(span) = &mut help.span {
                span.file = rebase_file_path(&span.file, project_root, report_root);
            }
        }

        for suggestion in &mut diagnostic.structured_suggestions {
            suggestion.span.file =
                rebase_file_path(&suggestion.span.file, project_root, report_root);
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

fn retain_changed_only(diagnostics: &mut Vec<Diagnostic>, changed_files: &BTreeSet<String>) {
    let normalized = changed_files
        .iter()
        .map(|file| normalize_file_path(file))
        .collect::<BTreeSet<_>>();

    diagnostics.retain(|diagnostic| {
        normalized.contains(&normalize_file_path(&diagnostic.primary_span.file))
    });
}

fn diagnostics_for_output(
    diagnostics: &[Diagnostic],
    min_confidence: MinConfidence,
    severity_threshold: SeverityThreshold,
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

pub(crate) fn has_blocking_diagnostics(
    diagnostics: &[Diagnostic],
    min_confidence: MinConfidence,
    severity_threshold: SeverityThreshold,
) -> bool {
    diagnostics.iter().any(|diagnostic| {
        !diagnostic.suppressed && passes_thresholds(diagnostic, min_confidence, severity_threshold)
    })
}

pub(crate) fn passes_thresholds(
    diagnostic: &Diagnostic,
    min_confidence: MinConfidence,
    severity_threshold: SeverityThreshold,
) -> bool {
    confidence_rank(diagnostic.confidence) >= min_confidence_rank(min_confidence)
        && severity_rank(diagnostic.severity) >= severity_threshold_rank(severity_threshold)
}

pub(crate) fn suppression_visible(format: OutputFormat, show_suppressed: bool) -> bool {
    match format {
        OutputFormat::Text => show_suppressed,
        OutputFormat::Json | OutputFormat::Sarif => true,
    }
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
            let root = target.parent().unwrap_or(Path::new(".")).to_path_buf();
            append_expanded_project_roots(&root, &mut roots)?;
        } else if target
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("nr"))
            && let Some(root) = nearest_project_root(target.parent().unwrap_or(Path::new(".")))
        {
            append_expanded_project_roots(&root, &mut roots)?;
        }
    } else if let Some(root) = nearest_project_root(target) {
        append_expanded_project_roots(&root, &mut roots)?;
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

fn append_expanded_project_roots(root: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let members = workspace_members(root)?;
    if members.is_empty() {
        out.push(root.to_path_buf());
        return Ok(());
    }

    if select_entry_file(root).is_some() {
        out.push(root.to_path_buf());
    }
    for member in members {
        out.push(member);
    }
    Ok(())
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
        append_expanded_project_roots(dir, out)?;
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

fn workspace_members(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let manifest_path = root.join("Nargo.toml");
    if !manifest_path.is_file() {
        return Ok(Vec::new());
    }

    let manifest = fs::read_to_string(&manifest_path)?;
    let parsed = toml::from_str::<TomlValue>(&manifest).ok();
    let Some(parsed) = parsed else {
        return Ok(Vec::new());
    };
    let Some(workspace) = parsed.get("workspace") else {
        return Ok(Vec::new());
    };
    let Some(members) = workspace.get("members").and_then(TomlValue::as_array) else {
        return Ok(Vec::new());
    };

    let mut resolved = Vec::<PathBuf>::new();
    for member in members {
        let Some(member_path) = member.as_str() else {
            continue;
        };
        let candidate = root.join(member_path);
        if candidate.join("Nargo.toml").is_file() {
            resolved.push(candidate);
        }
    }
    resolved.sort();
    resolved.dedup();
    Ok(resolved)
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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::workspace_members;

    #[test]
    fn reads_workspace_members_from_nargo_manifest() {
        let tmp = tempdir().expect("temp dir should be created");
        let root = tmp.path();
        fs::write(
            root.join("Nargo.toml"),
            "[workspace]\nmembers=[\"a\",\"b\"]\n",
        )
        .expect("workspace manifest should be written");
        for member in ["a", "b"] {
            fs::create_dir_all(root.join(member)).expect("member dir should be created");
            fs::write(
                root.join(member).join("Nargo.toml"),
                format!(
                    "[package]\nname=\"{member}\"\ntype=\"bin\"\nauthors=[\"\"]\n",
                    member = member
                ),
            )
            .expect("member manifest should be written");
        }

        let members = workspace_members(root).expect("workspace parsing should succeed");
        assert_eq!(members.len(), 2);
        assert!(members.iter().any(|path| path.ends_with("a")));
        assert!(members.iter().any(|path| path.ends_with("b")));
    }
}
