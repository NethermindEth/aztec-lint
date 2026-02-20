use std::path::Path;

use crate::noir::NoirFrontendError;

#[cfg(feature = "noir-compiler")]
use std::collections::HashMap;
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
use noirc_errors::{CustomDiagnostic, reporter::report_all};
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
    for diagnostic in &mut normalized {
        normalize_diagnostic_messages(diagnostic);
    }
    let file_map = file_manager.as_file_map();
    let _ = report_all(file_map, &normalized, false, false);
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
