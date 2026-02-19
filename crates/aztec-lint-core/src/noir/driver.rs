use std::path::Path;

use crate::noir::NoirFrontendError;

#[cfg(feature = "noir-compiler")]
use std::collections::HashMap;
#[cfg(feature = "noir-compiler")]
use std::path::PathBuf;

#[cfg(feature = "noir-compiler")]
use fm::{FileId, FileManager};
#[cfg(feature = "noir-compiler")]
use noirc_driver::{CompileOptions, CrateId, check_crate, file_manager_with_stdlib, prepare_crate};
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
    let mut file_manager = file_manager_with_stdlib(root);
    add_noir_sources_from_root(root, &mut file_manager)?;

    let entry = if entry.is_absolute() {
        entry.to_path_buf()
    } else {
        root.join(entry)
    };

    if file_manager.name_to_id(entry.clone()).is_none() {
        return Err(NoirFrontendError::EntryFileMissing { entry });
    }

    let parsed_files = parse_all_files(&file_manager)?;
    let mut context = Context::new(file_manager, parsed_files);
    let crate_id = prepare_crate(&mut context, &entry);
    let options = CompileOptions::default();

    let (_, diagnostics) = check_crate(&mut context, crate_id, &options).map_err(|issues| {
        NoirFrontendError::CheckDiagnostics {
            messages: issues.into_iter().map(|issue| issue.to_string()).collect(),
        }
    })?;

    let blocking_warnings = diagnostics
        .iter()
        .filter(|diag| diag.is_error())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if !blocking_warnings.is_empty() {
        return Err(NoirFrontendError::CheckDiagnostics {
            messages: blocking_warnings,
        });
    }

    Ok(NoirCheckedProject {
        root: root.to_path_buf(),
        entry,
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
fn add_noir_sources_from_root(
    root: &Path,
    file_manager: &mut FileManager,
) -> Result<(), NoirFrontendError> {
    add_noir_sources_recursively(root, file_manager)
}

#[cfg(feature = "noir-compiler")]
fn add_noir_sources_recursively(
    dir: &Path,
    file_manager: &mut FileManager,
) -> Result<(), NoirFrontendError> {
    let mut entries = std::fs::read_dir(dir)
        .map_err(|source| NoirFrontendError::Io {
            path: dir.to_path_buf(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| NoirFrontendError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            add_noir_sources_recursively(&path, file_manager)?;
            continue;
        }

        let is_noir_file = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("nr"));
        if !is_noir_file {
            continue;
        }

        let source = std::fs::read_to_string(&path).map_err(|source| NoirFrontendError::Io {
            path: path.clone(),
            source,
        })?;
        let _ = file_manager.add_file_with_source_canonical_path(&path, source);
    }

    Ok(())
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
    let mut parser_issues = Vec::<String>::new();

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
                parser_issues.push(format!("{file_label}: {diagnostic}"));
            }
        }

        parsed_files.insert(file_id, (module, diagnostics));
    }

    if !parser_issues.is_empty() {
        return Err(NoirFrontendError::ParserDiagnostics {
            messages: parser_issues,
        });
    }

    Ok(parsed_files)
}

#[cfg(test)]
#[cfg(feature = "noir-compiler")]
mod tests {
    use std::fs;

    use noirc_driver::file_manager_with_stdlib;
    use tempfile::tempdir;

    use super::add_noir_sources_from_root;

    #[test]
    fn source_discovery_order_is_deterministic() {
        let temp = tempdir().expect("temp dir should be created");
        let root = temp.path();
        fs::create_dir_all(root.join("src/nested")).expect("nested dir should be created");
        fs::write(root.join("src/main.nr"), "fn main() {}\n").expect("main source should exist");
        fs::write(root.join("src/zeta.nr"), "fn zeta() {}\n").expect("zeta source should exist");
        fs::write(root.join("src/nested/alpha.nr"), "fn alpha() {}\n")
            .expect("alpha source should exist");
        fs::write(root.join("src/nested/beta.nr"), "fn beta() {}\n")
            .expect("beta source should exist");

        let mut left = file_manager_with_stdlib(root);
        add_noir_sources_from_root(root, &mut left).expect("left discovery should succeed");
        let left_ids = user_file_id_map(root, &left);

        let mut right = file_manager_with_stdlib(root);
        add_noir_sources_from_root(root, &mut right).expect("right discovery should succeed");
        let right_ids = user_file_id_map(root, &right);

        assert_eq!(left_ids, right_ids);
    }

    fn user_file_id_map(
        root: &std::path::Path,
        file_manager: &fm::FileManager,
    ) -> Vec<(String, usize)> {
        let mut mapped = file_manager
            .as_file_map()
            .all_file_ids()
            .filter_map(|file_id| {
                let path = file_manager.path(*file_id)?;
                if !path.starts_with(root) {
                    return None;
                }
                Some((path.display().to_string(), file_id.as_usize()))
            })
            .collect::<Vec<(String, usize)>>();
        mapped.sort_by(|left, right| left.0.cmp(&right.0));
        mapped
    }
}
