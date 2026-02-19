use std::path::Path;

use crate::model::ProjectModel;
use crate::noir::NoirFrontendError;

#[cfg(feature = "noir-compiler")]
use crate::model::CallEdge;
#[cfg(feature = "noir-compiler")]
use crate::model::{ModuleEdge, SymbolKind, SymbolRef, TypeRef};
#[cfg(feature = "noir-compiler")]
use crate::noir::call_graph::extract_best_effort_call_edges;
#[cfg(feature = "noir-compiler")]
use crate::noir::driver::{NoirCheckedProject, load_and_check_project};
#[cfg(feature = "noir-compiler")]
use crate::noir::span_mapper::SpanMapper;
#[cfg(feature = "noir-compiler")]
use noirc_frontend::hir::def_map::LocalModuleId;
#[cfg(feature = "noir-compiler")]
use noirc_frontend::parser::ItemKind;

#[cfg(feature = "noir-compiler")]
pub fn build_project_model(root: &Path, entry: &Path) -> Result<ProjectModel, NoirFrontendError> {
    let checked = load_and_check_project(root, entry)?;
    build_from_checked(&checked)
}

#[cfg(not(feature = "noir-compiler"))]
pub fn build_project_model(_root: &Path, _entry: &Path) -> Result<ProjectModel, NoirFrontendError> {
    Err(NoirFrontendError::CompilerFeatureDisabled)
}

#[cfg(feature = "noir-compiler")]
fn build_from_checked(checked: &NoirCheckedProject) -> Result<ProjectModel, NoirFrontendError> {
    let context = checked.context();
    let Some(def_map) = context.def_map(&checked.crate_id()) else {
        return Err(NoirFrontendError::Internal(
            "crate def map missing after successful check".to_string(),
        ));
    };

    let span_mapper = SpanMapper::new(checked.root(), &context.file_manager);
    let mut model = ProjectModel::default();

    let mut user_files = def_map
        .file_ids()
        .into_iter()
        .filter(|file_id| span_mapper.is_user_file(*file_id))
        .collect::<Vec<_>>();
    user_files.sort_by_key(|file_id| file_id.as_usize());

    model.ast_ids = user_files
        .iter()
        .map(|file_id| span_mapper.normalize_file_path(*file_id))
        .collect();

    for (module_index, module) in def_map.modules().iter() {
        if !span_mapper.is_user_file(module.location.file) {
            continue;
        }
        let local_id = LocalModuleId::new(module_index);
        let current_module_name = module_name(def_map, local_id, module.parent);
        let module_symbol_id = format!("module::{current_module_name}");

        model.symbols.push(SymbolRef {
            symbol_id: module_symbol_id.clone(),
            name: current_module_name.clone(),
            kind: SymbolKind::Module,
            span: span_mapper.map_location(module.location),
        });

        let mut child_modules = module
            .children
            .values()
            .copied()
            .filter(|child| span_mapper.is_user_file(def_map[*child].location.file))
            .collect::<Vec<_>>();
        child_modules.sort_by_key(|child| module_name(def_map, *child, def_map[*child].parent));
        for child in child_modules {
            model.module_graph.push(ModuleEdge {
                from_module: current_module_name.clone(),
                to_module: module_name(def_map, child, def_map[child].parent),
            });
        }

        let mut value_defs = module.value_definitions().collect::<Vec<_>>();
        value_defs.sort_by_key(|definition| format!("{definition:?}"));
        for definition in value_defs {
            if let Some(function_id) = definition.as_function() {
                let Some(function_meta) = context.def_interner.try_function_meta(&function_id)
                else {
                    continue;
                };
                if !span_mapper.is_user_file(function_meta.location.file) {
                    continue;
                }

                let symbol_id = format!("fn::{function_id}");
                model.symbols.push(SymbolRef {
                    symbol_id: symbol_id.clone(),
                    name: context.def_interner.function_name(&function_id).to_string(),
                    kind: SymbolKind::Function,
                    span: span_mapper.map_location(function_meta.location),
                });
                model.type_refs.push(TypeRef {
                    symbol_id,
                    type_repr: format!("function/arity:{}", function_meta.parameters.len()),
                });
            } else if let Some(global_id) = definition.as_global() {
                let global = context.def_interner.get_global(global_id);
                if !span_mapper.is_user_file(global.location.file) {
                    continue;
                }

                let symbol_id = format!("global::{global_id:?}");
                model.symbols.push(SymbolRef {
                    symbol_id: symbol_id.clone(),
                    name: global.ident.to_string(),
                    kind: SymbolKind::Global,
                    span: span_mapper.map_location(global.location),
                });
                model.type_refs.push(TypeRef {
                    symbol_id,
                    type_repr: "global".to_string(),
                });
            }
        }

        let mut type_defs = module.type_definitions().collect::<Vec<_>>();
        type_defs.sort_by_key(|definition| format!("{definition:?}"));
        for definition in type_defs {
            if let Some(type_id) = definition.as_type() {
                let data_type = context.def_interner.get_type(type_id);
                let data_type = data_type.borrow();
                if !span_mapper.is_user_file(data_type.location.file) {
                    continue;
                }

                let symbol_id = format!("type::{type_id:?}");
                model.symbols.push(SymbolRef {
                    symbol_id: symbol_id.clone(),
                    name: data_type.name.to_string(),
                    kind: SymbolKind::Struct,
                    span: span_mapper.map_location(data_type.location),
                });
                model.type_refs.push(TypeRef {
                    symbol_id,
                    type_repr: data_type.to_string(),
                });
            } else if let Some(alias_id) = definition.as_type_alias() {
                let alias = context.def_interner.get_type_alias(alias_id);
                let alias = alias.borrow();
                if !span_mapper.is_user_file(alias.location.file) {
                    continue;
                }

                let symbol_id = format!("alias::{alias_id:?}");
                model.symbols.push(SymbolRef {
                    symbol_id: symbol_id.clone(),
                    name: alias.name.to_string(),
                    kind: SymbolKind::Unknown,
                    span: span_mapper.map_location(alias.location),
                });
                model.type_refs.push(TypeRef {
                    symbol_id,
                    type_repr: format!("alias:{}", alias.name),
                });
            }
        }
    }

    for file_id in user_files {
        let (parsed_module, _) = context.parsed_file_results(file_id);
        for item in parsed_module.items {
            if let ItemKind::Import(use_tree, _) = item.kind {
                let span = span_mapper.map_location(item.location);
                let symbol_id = format!("import::{}:{}:{}", span.file, span.start, span.end);
                model.symbols.push(SymbolRef {
                    symbol_id,
                    name: use_tree.to_string(),
                    kind: SymbolKind::Import,
                    span,
                });
            }
        }
    }

    model.call_graph = filter_edges_to_user_symbols(
        extract_best_effort_call_edges(context, checked.crate_id(), &span_mapper),
        &model.symbols,
    );

    model.ast_ids.sort();
    model.ast_ids.dedup();
    model.symbols.sort_by_key(|symbol| {
        (
            symbol.span.file.clone(),
            symbol.span.start,
            symbol.span.end,
            symbol.name.clone(),
            symbol.symbol_id.clone(),
        )
    });
    model
        .symbols
        .dedup_by(|left, right| left.symbol_id == right.symbol_id);
    model
        .type_refs
        .sort_by_key(|type_ref| (type_ref.symbol_id.clone(), type_ref.type_repr.clone()));
    model.type_refs.dedup();
    model
        .module_graph
        .sort_by_key(|edge| (edge.from_module.clone(), edge.to_module.clone()));
    model.module_graph.dedup();

    Ok(model)
}

#[cfg(feature = "noir-compiler")]
fn filter_edges_to_user_symbols(edges: Vec<CallEdge>, symbols: &[SymbolRef]) -> Vec<CallEdge> {
    let known_symbols = symbols
        .iter()
        .map(|symbol| symbol.symbol_id.clone())
        .collect::<std::collections::HashSet<_>>();
    edges
        .into_iter()
        .filter(|edge| {
            known_symbols.contains(&edge.caller_symbol_id)
                && known_symbols.contains(&edge.callee_symbol_id)
        })
        .collect()
}

#[cfg(feature = "noir-compiler")]
fn module_name(
    def_map: &noirc_frontend::hir::def_map::CrateDefMap,
    id: LocalModuleId,
    parent: Option<LocalModuleId>,
) -> String {
    let raw = def_map.get_module_path(id, parent);
    if raw.is_empty() {
        "<root>".to_string()
    } else {
        raw
    }
}

#[cfg(test)]
#[cfg(feature = "noir-compiler")]
mod tests {
    use std::path::PathBuf;
    use std::thread;

    use serde_json::to_vec;

    use super::build_project_model;

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/noir_core/minimal")
            .canonicalize()
            .expect("fixture path should exist")
    }

    fn run_with_large_stack<T: Send + 'static>(job: impl FnOnce() -> T + Send + 'static) -> T {
        thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(job)
            .expect("thread should spawn")
            .join()
            .expect("thread should complete")
    }

    #[test]
    fn compiles_minimal_noir_fixture_into_project_model() {
        let root = fixture_root();
        let model = run_with_large_stack(move || {
            build_project_model(&root, &root.join("src/main.nr"))
                .expect("fixture should build through noirc frontend")
        });

        assert!(!model.ast_ids.is_empty());
        assert!(model.ast_ids.iter().any(|id| id.ends_with("src/main.nr")));
        assert!(
            model
                .symbols
                .iter()
                .any(|symbol| symbol.kind == crate::model::SymbolKind::Function)
        );
    }

    #[test]
    fn project_model_serialization_is_deterministic_across_runs() {
        let root = fixture_root();
        let first = run_with_large_stack({
            let root = root.clone();
            move || {
                build_project_model(&root, &root.join("src/main.nr"))
                    .expect("first run should pass")
            }
        });
        let second = run_with_large_stack(move || {
            build_project_model(&root, &root.join("src/main.nr")).expect("second run should pass")
        });

        let first_json = to_vec(&first).expect("serialization should succeed");
        let second_json = to_vec(&second).expect("serialization should succeed");
        assert_eq!(first_json, second_json);
    }
}
