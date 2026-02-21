use std::path::Path;

use crate::config::DeprecatedPathConfig;
use crate::noir::NoirFrontendError;
#[cfg(feature = "noir-compiler")]
use crate::output::ansi::{Colorizer, Stream};

#[cfg(feature = "noir-compiler")]
use std::collections::{HashMap, HashSet};
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
    load_and_check_project_with_options(root, entry, DeprecatedPathConfig::default())
}

#[cfg(feature = "noir-compiler")]
pub fn load_and_check_project_with_options(
    root: &Path,
    entry: &Path,
    deprecated_path: DeprecatedPathConfig,
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
    let compiler_warnings =
        filter_deprecated_path_warnings(&context.file_manager, compiler_warnings, deprecated_path);
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

#[cfg(not(feature = "noir-compiler"))]
pub fn load_and_check_project_with_options(
    _root: &Path,
    _entry: &Path,
    _deprecated_path: DeprecatedPathConfig,
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
        normalize_diagnostic_messages(file_manager, diagnostic);
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
fn normalize_diagnostic_messages(_file_manager: &FileManager, _diagnostic: &mut CustomDiagnostic) {}

#[cfg(feature = "noir-compiler")]
fn filter_deprecated_path_warnings(
    file_manager: &FileManager,
    warnings: Vec<CustomDiagnostic>,
    config: DeprecatedPathConfig,
) -> Vec<CustomDiagnostic> {
    let mut filtered = Vec::with_capacity(warnings.len());
    let mut dedup = HashSet::<DeprecatedPathDedupKey>::new();

    for mut warning in warnings {
        let Some(context) = deprecated_path_context(file_manager, &warning) else {
            filtered.push(warning);
            continue;
        };

        let has_safe_absolute_rewrite =
            config.try_absolute_root && warning_has_absolute_replacement_hint(&warning);
        if has_safe_absolute_rewrite && !context.blocked_by_local_binding {
            filtered.push(warning);
            continue;
        }

        if context.blocked_by_local_binding {
            let key = DeprecatedPathDedupKey::new(
                &context,
                DeprecatedPathSuppressionReason::BlockedByLocalBinding,
            );
            if !dedup.insert(key) {
                continue;
            }

            if config.warn_on_blocked {
                annotate_blocked_deprecated_path(&mut warning);
                filtered.push(warning);
            } else if config.verbose_blocked_notes {
                downgrade_to_info(&mut warning);
                annotate_blocked_deprecated_path(&mut warning);
                filtered.push(warning);
            }
            continue;
        }

        let key = DeprecatedPathDedupKey::new(
            &context,
            DeprecatedPathSuppressionReason::NoVerifiedReplacement,
        );
        if !dedup.insert(key) {
            continue;
        }

        if config.verbose_blocked_notes {
            downgrade_to_info(&mut warning);
            annotate_unfixable_deprecated_path(&mut warning);
            filtered.push(warning);
        }
    }

    filtered
}

#[cfg(feature = "noir-compiler")]
fn warning_has_absolute_replacement_hint(warning: &CustomDiagnostic) -> bool {
    warning
        .secondaries
        .iter()
        .any(|secondary| secondary.message.contains("Please use `::aztec` instead"))
}

#[cfg(feature = "noir-compiler")]
fn deprecated_path_context(
    file_manager: &FileManager,
    warning: &CustomDiagnostic,
) -> Option<DeprecatedPathContext> {
    let secondary = warning
        .secondaries
        .iter()
        .find(|secondary| secondary.message.contains("Please use `::aztec` instead"))?;
    let location = secondary.location;

    let source = file_manager.fetch_file(location.file)?;
    let offset = usize::try_from(location.span.start()).unwrap_or(source.len());
    if !statement_contains_dep_aztec_path(source, offset) {
        return None;
    }

    Some(DeprecatedPathContext {
        file_id: location.file,
        statement_start: statement_start(source, offset),
        statement_end: statement_end(source, offset),
        blocked_by_local_binding: scope_binds_aztec(source, offset),
    })
}

#[cfg(feature = "noir-compiler")]
fn statement_contains_dep_aztec_path(source: &str, offset: usize) -> bool {
    let start = statement_start(source, offset);
    let end = statement_end(source, offset);
    source
        .get(start..end)
        .is_some_and(|statement| statement.contains("dep::aztec::"))
}

#[cfg(feature = "noir-compiler")]
fn downgrade_to_info(warning: &mut CustomDiagnostic) {
    use noirc_errors::DiagnosticKind;

    warning.kind = DiagnosticKind::Info;
}

#[cfg(feature = "noir-compiler")]
fn annotate_blocked_deprecated_path(warning: &mut CustomDiagnostic) {
    if !warning
        .notes
        .iter()
        .any(|note| note.contains("blocked by local binding `aztec`"))
    {
        warning.notes.push(
            "deprecated path migration blocked by local binding `aztec`; rename/alias the local binding or keep `dep::aztec::...`".to_string(),
        );
    }
}

#[cfg(feature = "noir-compiler")]
fn annotate_unfixable_deprecated_path(warning: &mut CustomDiagnostic) {
    if !warning
        .notes
        .iter()
        .any(|note| note.contains("no verified safe replacement"))
    {
        warning.notes.push(
            "deprecated path migration skipped: no verified safe replacement candidate".to_string(),
        );
    }
}

#[cfg(feature = "noir-compiler")]
#[derive(Clone, Copy, Debug)]
struct DeprecatedPathContext {
    file_id: FileId,
    statement_start: usize,
    statement_end: usize,
    blocked_by_local_binding: bool,
}

#[cfg(feature = "noir-compiler")]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum DeprecatedPathSuppressionReason {
    BlockedByLocalBinding,
    NoVerifiedReplacement,
}

#[cfg(feature = "noir-compiler")]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct DeprecatedPathDedupKey {
    file_id: usize,
    statement_start: usize,
    statement_end: usize,
    reason: DeprecatedPathSuppressionReason,
}

#[cfg(feature = "noir-compiler")]
impl DeprecatedPathDedupKey {
    fn new(context: &DeprecatedPathContext, reason: DeprecatedPathSuppressionReason) -> Self {
        Self {
            file_id: context.file_id.as_usize(),
            statement_start: context.statement_start,
            statement_end: context.statement_end,
            reason,
        }
    }
}

#[cfg(feature = "noir-compiler")]
fn scope_binds_aztec(source: &str, offset: usize) -> bool {
    if offset > source.len() {
        return false;
    }
    let prefix = &source[..offset];
    for line in prefix.lines() {
        let code = strip_line_comment(line).trim();
        if code.is_empty() {
            continue;
        }
        if binds_aztec_in_use_statement(code)
            || code.starts_with("let aztec")
            || code.starts_with("let mut aztec")
            || code.starts_with("mod aztec")
            || code.starts_with("pub mod aztec")
            || code.starts_with("struct aztec")
            || code.starts_with("enum aztec")
            || code.starts_with("trait aztec")
            || code.starts_with("type aztec")
            || code.contains("(aztec:")
            || code.contains(", aztec:")
        {
            return true;
        }
    }
    false
}

#[cfg(feature = "noir-compiler")]
fn binds_aztec_in_use_statement(statement: &str) -> bool {
    let compact = statement.trim();
    if !compact.starts_with("use ") && !compact.starts_with("pub use ") {
        return false;
    }
    let use_tail = compact.split_once("use ").map_or(compact, |(_, tail)| tail);
    let use_tail = use_tail.trim_end_matches(';').trim();
    if use_tail.contains(" as aztec") {
        return true;
    }
    let trimmed_tail = use_tail.trim_end();
    if trimmed_tail.ends_with("::aztec") {
        return true;
    }
    if let Some(open) = trimmed_tail.find('{') {
        let Some(close_rel) = trimmed_tail[open + 1..].find('}') else {
            return false;
        };
        let close = open + 1 + close_rel;
        let grouped = &trimmed_tail[open + 1..close];
        return grouped
            .split(',')
            .any(|leaf| leaf.trim() == "aztec" || leaf.trim_end().ends_with(" as aztec"));
    }
    false
}

#[cfg(feature = "noir-compiler")]
fn strip_line_comment(line: &str) -> &str {
    line.split_once("//").map_or(line, |(code, _)| code)
}

#[cfg(feature = "noir-compiler")]
fn statement_start(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())]
        .rfind([';', '{', '}'])
        .map_or(0, |idx| idx + 1)
}

#[cfg(feature = "noir-compiler")]
fn statement_end(source: &str, offset: usize) -> usize {
    let bytes = source.as_bytes();
    let mut cursor = offset.min(source.len());
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    while cursor < bytes.len() {
        match bytes[cursor] {
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b'{' => brace_depth += 1,
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            b';' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                return cursor + 1;
            }
            b'\n' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                return cursor;
            }
            _ => {}
        }
        cursor += 1;
    }
    source.len()
}

#[cfg(test)]
#[cfg(feature = "noir-compiler")]
mod tests {
    use std::path::Path;

    use fm::FileManager;
    use noirc_errors::{CustomDiagnostic, DiagnosticKind, Location, Span};

    use super::filter_deprecated_path_warnings;
    use crate::config::DeprecatedPathConfig;

    #[test]
    fn keeps_actionable_deprecated_path_warning_with_absolute_rewrite_hint() {
        let mut files = FileManager::new(Path::new("."));
        let source = "use dep::aztec::protocol_types::A;\n";
        let file_id = files
            .add_file_with_source(Path::new("src/main.nr"), source.to_string())
            .expect("file should be added");
        let warning = deprecated_path_warning(file_id, source, "dep::aztec::");

        let filtered =
            filter_deprecated_path_warnings(&files, vec![warning], DeprecatedPathConfig::default());

        assert_eq!(filtered.len(), 1);
        assert!(
            filtered[0]
                .secondaries
                .iter()
                .any(|secondary| secondary.message.contains("Please use `::aztec` instead"))
        );
        assert!(
            !filtered[0]
                .secondaries
                .iter()
                .any(|secondary| secondary.message.contains("Please use `aztec::` instead"))
        );
    }

    #[test]
    fn suppresses_blocked_deprecated_path_warning_by_default() {
        let mut files = FileManager::new(Path::new("."));
        let source = "use aztec::macros::aztec;\nuse dep::aztec::protocol_types::A;\n";
        let file_id = files
            .add_file_with_source(Path::new("src/main.nr"), source.to_string())
            .expect("file should be added");
        let warning = deprecated_path_warning(file_id, source, "dep::aztec::");

        let filtered =
            filter_deprecated_path_warnings(&files, vec![warning], DeprecatedPathConfig::default());

        assert!(filtered.is_empty());
    }

    #[test]
    fn emits_single_info_for_blocked_deprecated_path_in_verbose_mode() {
        let mut files = FileManager::new(Path::new("."));
        let source =
            "use aztec::macros::aztec;\nuse dep::aztec::{protocol_types::A, protocol_types::B};\n";
        let file_id = files
            .add_file_with_source(Path::new("src/main.nr"), source.to_string())
            .expect("file should be added");
        let warning_a = deprecated_path_warning(file_id, source, "dep::aztec::{");
        let warning_b = deprecated_path_warning(file_id, source, "protocol_types::B");
        let config = DeprecatedPathConfig {
            warn_on_blocked: false,
            try_absolute_root: true,
            verbose_blocked_notes: true,
        };

        let filtered = filter_deprecated_path_warnings(&files, vec![warning_a, warning_b], config);

        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].is_info());
        assert!(
            filtered[0]
                .notes
                .iter()
                .any(|note| note.contains("blocked by local binding `aztec`"))
        );
    }

    #[test]
    fn suppresses_warning_when_absolute_rewrite_attempt_is_disabled() {
        let mut files = FileManager::new(Path::new("."));
        let source = "use dep::aztec::protocol_types::A;\n";
        let file_id = files
            .add_file_with_source(Path::new("src/main.nr"), source.to_string())
            .expect("file should be added");
        let warning = deprecated_path_warning(file_id, source, "dep::aztec::");
        let config = DeprecatedPathConfig {
            warn_on_blocked: false,
            try_absolute_root: false,
            verbose_blocked_notes: false,
        };

        let filtered = filter_deprecated_path_warnings(&files, vec![warning], config);

        assert!(filtered.is_empty());
    }

    fn deprecated_path_warning(
        file_id: fm::FileId,
        source: &str,
        marker: &str,
    ) -> CustomDiagnostic {
        let marker_offset = source.find(marker).expect("marker should exist");
        let dep_offset = source[marker_offset..]
            .find("dep::aztec::")
            .map(|relative| marker_offset + relative)
            .or_else(|| source.find("dep::aztec::"))
            .expect("dep::aztec path should exist");
        let location = Location::new(
            Span::from(
                u32::try_from(dep_offset).expect("fits")
                    ..u32::try_from(dep_offset + "dep::aztec::".len()).expect("fits"),
            ),
            file_id,
        );
        let mut diagnostic = CustomDiagnostic::simple_warning(
            "`dep::aztec` path is deprecated".to_string(),
            "Please use `::aztec` instead".to_string(),
            location,
        );
        diagnostic.kind = DiagnosticKind::Warning;
        diagnostic
    }
}
