use std::path::Path;

use crate::noir::NoirFrontendError;
#[cfg(feature = "noir-compiler")]
use crate::output::ansi::{Colorizer, Stream};

#[cfg(feature = "noir-compiler")]
use std::collections::HashMap;
#[cfg(feature = "noir-compiler")]
use std::fmt::Write as _;
#[cfg(feature = "noir-compiler")]
use std::path::PathBuf;

#[cfg(feature = "noir-compiler")]
use fm::{FileId, FileManager};
#[cfg(feature = "noir-compiler")]
use nargo::{insert_all_files_for_workspace_into_file_manager, prepare_dependencies};
#[cfg(feature = "noir-compiler")]
use nargo_toml::{PackageSelection, find_file_manifest, resolve_workspace_from_toml};
#[cfg(feature = "noir-compiler")]
use noirc_driver::{CompileOptions, CrateId, check_crate, prepare_crate};
#[cfg(feature = "noir-compiler")]
use noirc_errors::{CustomDiagnostic, reporter::line_and_column_from_span};
#[cfg(feature = "noir-compiler")]
use noirc_frontend::{hir::Context, parse_program, parser::ParserError};

#[cfg(feature = "noir-compiler")]
pub struct NoirCheckedProject {
    root: PathBuf,
    entry: PathBuf,
    crate_id: CrateId,
    context: Context<'static, 'static>,
}

#[cfg(not(feature = "noir-compiler"))]
pub struct NoirCheckedProject;

#[cfg(feature = "noir-compiler")]
impl NoirCheckedProject {
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn entry(&self) -> &Path {
        &self.entry
    }

    pub fn crate_id(&self) -> CrateId {
        self.crate_id
    }

    pub fn context(&self) -> &Context<'static, 'static> {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut Context<'static, 'static> {
        &mut self.context
    }
}

#[cfg(feature = "noir-compiler")]
pub fn load_and_check_project(
    root: &Path,
    entry: &Path,
) -> Result<NoirCheckedProject, NoirFrontendError> {
    let project_root = canonicalize_best_effort(root);
    let requested_entry = if entry.is_absolute() {
        entry.to_path_buf()
    } else {
        root.join(entry)
    };
    let (workspace, package) = resolve_workspace_and_package(root, &requested_entry)?;

    let mut file_manager = workspace.new_file_manager();
    insert_all_files_for_workspace_into_file_manager(&workspace, &mut file_manager);

    if file_manager
        .name_to_id(package.entry_path.clone())
        .is_none()
    {
        return Err(NoirFrontendError::EntryFileMissing {
            entry: package.entry_path,
        });
    }

    let parsed_files = parse_all_files(&file_manager)?;
    let mut context = Context::new(file_manager, parsed_files);
    let crate_id = prepare_crate(&mut context, &package.entry_path);
    context.required_unstable_features.insert(
        crate_id,
        package.compiler_required_unstable_features.clone(),
    );
    prepare_dependencies(&mut context, crate_id, &package.dependencies);
    let options = CompileOptions::default();

    let (_, diagnostics) = check_crate(&mut context, crate_id, &options).map_err(|issues| {
        let blocking = issues
            .iter()
            .filter(|diagnostic| diagnostic.is_error())
            .cloned()
            .collect::<Vec<_>>();
        let emitted = if blocking.is_empty() {
            issues
        } else {
            blocking
        };
        emit_diagnostics(&context.file_manager, &emitted);
        NoirFrontendError::CheckDiagnostics {
            count: emitted.len(),
        }
    })?;

    let blocking_warnings = diagnostics
        .iter()
        .filter(|diag| diag.is_error())
        .cloned()
        .collect::<Vec<_>>();
    if !blocking_warnings.is_empty() {
        emit_diagnostics(&context.file_manager, &blocking_warnings);
        return Err(NoirFrontendError::CheckDiagnostics {
            count: blocking_warnings.len(),
        });
    }

    // Keep compiler warnings visible in text mode, similar to cargo/clippy behavior.
    let compiler_warnings = diagnostics
        .iter()
        .filter(|diag| diag.is_warning())
        .filter(|diag| {
            context
                .file_manager
                .path(diag.file)
                .is_some_and(|path| canonicalize_best_effort(path).starts_with(&project_root))
        })
        .cloned()
        .collect::<Vec<_>>();
    if !compiler_warnings.is_empty() {
        emit_diagnostics(&context.file_manager, &compiler_warnings);
    }

    Ok(NoirCheckedProject {
        root: canonicalize_best_effort(&package.root_dir),
        entry: canonicalize_best_effort(&package.entry_path),
        crate_id,
        context,
    })
}

#[cfg(not(feature = "noir-compiler"))]
pub fn load_and_check_project(
    _root: &Path,
    _entry: &Path,
) -> Result<NoirCheckedProject, NoirFrontendError> {
    Err(NoirFrontendError::CompilerFeatureDisabled)
}

#[cfg(feature = "noir-compiler")]
fn resolve_workspace_and_package(
    root: &Path,
    entry: &Path,
) -> Result<(nargo::workspace::Workspace, nargo::package::Package), NoirFrontendError> {
    let manifest = find_manifest_path(root)?;
    let workspace = resolve_workspace_from_toml(&manifest, PackageSelection::DefaultOrAll, None)
        .map_err(|source| {
            NoirFrontendError::Internal(format!(
                "failed to resolve Nargo workspace from '{}': {source}",
                manifest.display()
            ))
        })?;

    let requested_root = canonicalize_best_effort(root);
    let requested_entry = canonicalize_best_effort(entry);

    let selected = workspace
        .members
        .iter()
        .find(|package| canonicalize_best_effort(&package.root_dir) == requested_root)
        .or_else(|| {
            workspace
                .members
                .iter()
                .find(|package| canonicalize_best_effort(&package.entry_path) == requested_entry)
        })
        .or_else(|| {
            if workspace.members.len() == 1 {
                workspace.members.first()
            } else {
                None
            }
        })
        .cloned()
        .ok_or_else(|| {
            let available = workspace
                .members
                .iter()
                .map(|package| package.root_dir.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            NoirFrontendError::Internal(format!(
                "failed to select package for root '{}' and entry '{}' from workspace members [{available}]",
                root.display(),
                entry.display()
            ))
        })?;

    Ok((workspace, selected))
}

#[cfg(feature = "noir-compiler")]
fn find_manifest_path(path: &Path) -> Result<PathBuf, NoirFrontendError> {
    if path.is_file()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "Nargo.toml")
    {
        return Ok(path.to_path_buf());
    }

    if let Some(manifest) = find_file_manifest(path) {
        return Ok(manifest);
    }

    Err(NoirFrontendError::Internal(format!(
        "no Nargo.toml found for '{}'",
        path.display()
    )))
}

#[cfg(feature = "noir-compiler")]
fn canonicalize_best_effort(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(feature = "noir-compiler")]
fn parse_all_files(
    file_manager: &FileManager,
) -> Result<HashMap<FileId, (noirc_frontend::ParsedModule, Vec<ParserError>)>, NoirFrontendError> {
    let mut file_ids = file_manager
        .as_file_map()
        .all_file_ids()
        .copied()
        .collect::<Vec<_>>();
    file_ids.sort_by_key(FileId::as_usize);

    let mut parsed_files = HashMap::new();
    let mut parser_issues = Vec::<CustomDiagnostic>::new();

    for file_id in file_ids {
        let Some(source) = file_manager.fetch_file(file_id) else {
            continue;
        };
        let (module, diagnostics) = parse_program(source, file_id);

        let file_label = file_manager
            .path(file_id)
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| format!("file#{}", file_id.as_usize()));
        for diagnostic in &diagnostics {
            if !diagnostic.is_warning() {
                let mut converted = CustomDiagnostic::from(diagnostic);
                if converted.message.is_empty() {
                    converted.message = format!("{file_label}: {diagnostic}");
                }
                parser_issues.push(converted);
            }
        }

        parsed_files.insert(file_id, (module, diagnostics));
    }

    if !parser_issues.is_empty() {
        emit_diagnostics(file_manager, &parser_issues);
        return Err(NoirFrontendError::ParserDiagnostics {
            count: parser_issues.len(),
        });
    }

    Ok(parsed_files)
}

#[cfg(feature = "noir-compiler")]
fn emit_diagnostics(file_manager: &FileManager, diagnostics: &[CustomDiagnostic]) {
    let mut normalized = diagnostics.to_vec();
    let colors = Colorizer::for_stream(Stream::Stderr);
    for diagnostic in &mut normalized {
        normalize_diagnostic_messages(diagnostic);
    }
    let mut rendered = String::new();
    for diagnostic in &normalized {
        render_compiler_diagnostic(&mut rendered, file_manager, diagnostic, colors);
        let _ = writeln!(rendered);
    }
    eprint!("{rendered}");
}

#[cfg(feature = "noir-compiler")]
fn render_compiler_diagnostic(
    output: &mut String,
    file_manager: &FileManager,
    diagnostic: &CustomDiagnostic,
    colors: Colorizer,
) {
    let severity = diagnostic_kind_label(diagnostic);
    let severity = match severity {
        "error" => colors.error(severity),
        "warning" => colors.warning(severity),
        "note" => colors.note(severity),
        _ => severity.to_string(),
    };
    let accent_arrow = colors.accent("-->");
    let accent_bar = colors.accent("|");

    let _ = writeln!(output, "{}: {}", severity, diagnostic.message);

    let (location, label_message) = diagnostic
        .secondaries
        .first()
        .map(|label| (label.location, Some(label.message.as_str())))
        .unwrap_or_else(|| {
            (
                noirc_errors::Location::new(noirc_errors::Span::single_char(0), diagnostic.file),
                None,
            )
        });

    let file_id = location.file;
    let file_path = file_manager
        .path(file_id)
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| format!("file#{}", file_id.as_usize()));

    let line_col = file_manager.fetch_file(file_id).map(|source| {
        let (line, col) = line_and_column_from_span(source, &location.span);
        (line.max(1), col.max(1), source)
    });

    let (line, col, source) = line_col
        .map(|(line, col, source)| (line, col, Some(source)))
        .unwrap_or((1, 1, None));

    let _ = writeln!(output, "  {} {}:{line}:{col}", accent_arrow, file_path);
    let _ = writeln!(output, "   {accent_bar}");

    let line_no = line.to_string();
    let gutter_width = line_no.len();
    if let Some(source) = source {
        if let Some(line_text) = source.lines().nth(line.saturating_sub(1) as usize) {
            let _ = writeln!(output, " {line_no:>gutter_width$} {accent_bar} {line_text}");
            let marker = marker_line(line_text, col, location.span.start(), location.span.end());
            let marker = match diagnostic_kind_label(diagnostic) {
                "warning" => colors.warning(&marker),
                "error" => colors.error(&marker),
                _ => colors.note(&marker),
            };
            if let Some(message) = label_message.filter(|message| !message.trim().is_empty()) {
                let _ = writeln!(
                    output,
                    " {:>gutter_width$} {accent_bar} {marker} {message}",
                    ""
                );
            } else {
                let _ = writeln!(output, " {:>gutter_width$} {accent_bar} {marker}", "");
            }
        } else {
            let marker = match diagnostic_kind_label(diagnostic) {
                "warning" => colors.warning("^"),
                "error" => colors.error("^"),
                _ => colors.note("^"),
            };
            let _ = writeln!(
                output,
                " {line_no:>gutter_width$} {accent_bar} <source unavailable>"
            );
            let _ = writeln!(output, " {:>gutter_width$} {accent_bar} {marker}", "");
        }
    } else {
        let marker = match diagnostic_kind_label(diagnostic) {
            "warning" => colors.warning("^"),
            "error" => colors.error("^"),
            _ => colors.note("^"),
        };
        let _ = writeln!(
            output,
            " {line_no:>gutter_width$} {accent_bar} <source unavailable>"
        );
        let _ = writeln!(output, " {:>gutter_width$} {accent_bar} {marker}", "");
    }
    let _ = writeln!(output, "   {accent_bar}");

    let note_label = colors.note("note");
    for secondary in diagnostic.secondaries.iter().skip(1) {
        if !secondary.message.trim().is_empty() {
            let _ = writeln!(output, "   = {note_label}: {}", secondary.message);
        }
    }
    for note in &diagnostic.notes {
        if !note.trim().is_empty() {
            let _ = writeln!(output, "   = {note_label}: {note}");
        }
    }
}

#[cfg(feature = "noir-compiler")]
fn marker_line(line_text: &str, col: u32, span_start: u32, span_end: u32) -> String {
    let line_width = line_text.chars().count();
    let col_zero = usize::try_from(col.saturating_sub(1)).unwrap_or(0);
    let start = col_zero.min(line_width);
    let span_len = usize::try_from(span_end.saturating_sub(span_start)).unwrap_or(1);
    let marker_len = span_len.max(1);

    format!("{}{}", " ".repeat(start), "^".repeat(marker_len))
}

#[cfg(feature = "noir-compiler")]
fn diagnostic_kind_label(diagnostic: &CustomDiagnostic) -> &'static str {
    use noirc_errors::DiagnosticKind;

    match diagnostic.kind {
        DiagnosticKind::Error | DiagnosticKind::Bug => "error",
        DiagnosticKind::Warning => "warning",
        DiagnosticKind::Info => "note",
    }
}

#[cfg(feature = "noir-compiler")]
fn normalize_diagnostic_messages(diagnostic: &mut CustomDiagnostic) {
    diagnostic.message = normalize_message(&diagnostic.message);
    for secondary in &mut diagnostic.secondaries {
        secondary.message = normalize_message(&secondary.message);
    }
    for note in &mut diagnostic.notes {
        *note = normalize_message(note);
    }
}

#[cfg(feature = "noir-compiler")]
fn normalize_message(message: &str) -> String {
    message.replace(
        "Please use `::aztec` instead",
        "Please use `aztec::` instead",
    )
}

#[cfg(test)]
#[cfg(feature = "noir-compiler")]
mod tests {
    use noirc_errors::{CustomDiagnostic, Location, Span};

    use super::normalize_diagnostic_messages;

    #[test]
    fn normalizes_aztec_deprecation_hint_message() {
        let mut diagnostic = CustomDiagnostic::from_message("primary", fm::FileId::dummy());
        let location = Location::new(Span::single_char(0), fm::FileId::dummy());
        diagnostic.add_secondary("Please use `::aztec` instead".to_string(), location);
        diagnostic
            .notes
            .push("note: Please use `::aztec` instead".to_string());

        normalize_diagnostic_messages(&mut diagnostic);

        assert!(
            diagnostic
                .secondaries
                .iter()
                .any(|secondary| secondary.message == "Please use `aztec::` instead")
        );
        assert!(
            diagnostic
                .notes
                .iter()
                .any(|note| note.contains("Please use `aztec::` instead"))
        );
    }
}
