use std::path::Path;

use crate::model::{ProjectModel, SemanticModel};
use crate::noir::NoirFrontendError;

#[cfg(feature = "noir-compiler")]
use crate::model::CallEdge;
#[cfg(feature = "noir-compiler")]
use crate::model::{ModuleEdge, SymbolKind, SymbolRef, TypeRef};
#[cfg(feature = "noir-compiler")]
use crate::noir::call_graph::call_edges_from_semantic;
#[cfg(feature = "noir-compiler")]
use crate::noir::driver::{NoirCheckedProject, load_and_check_project};
#[cfg(feature = "noir-compiler")]
use crate::noir::semantic_builder::extract_semantic_model;
#[cfg(feature = "noir-compiler")]
use crate::noir::span_mapper::SpanMapper;
#[cfg(feature = "noir-compiler")]
use noirc_frontend::hir::def_map::LocalModuleId;
#[cfg(feature = "noir-compiler")]
use noirc_frontend::parser::ItemKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSemanticBundle {
    pub project: ProjectModel,
}

impl ProjectSemanticBundle {
    pub fn project_model(&self) -> &ProjectModel {
        &self.project
    }

    pub fn semantic_model(&self) -> &SemanticModel {
        &self.project.semantic
    }

    pub fn into_project_model(self) -> ProjectModel {
        self.project
    }
}

#[cfg(feature = "noir-compiler")]
pub fn build_project_model(root: &Path, entry: &Path) -> Result<ProjectModel, NoirFrontendError> {
    Ok(build_project_semantic_bundle(root, entry)?.into_project_model())
}

#[cfg(feature = "noir-compiler")]
pub fn build_project_semantic_bundle(
    root: &Path,
    entry: &Path,
) -> Result<ProjectSemanticBundle, NoirFrontendError> {
    let checked = load_and_check_project(root, entry)?;
    build_bundle_from_checked(&checked)
}

#[cfg(not(feature = "noir-compiler"))]
pub fn build_project_model(_root: &Path, _entry: &Path) -> Result<ProjectModel, NoirFrontendError> {
    Err(NoirFrontendError::CompilerFeatureDisabled)
}

#[cfg(not(feature = "noir-compiler"))]
pub fn build_project_semantic_bundle(
    _root: &Path,
    _entry: &Path,
) -> Result<ProjectSemanticBundle, NoirFrontendError> {
    Err(NoirFrontendError::CompilerFeatureDisabled)
}

#[cfg(feature = "noir-compiler")]
fn build_bundle_from_checked(
    checked: &NoirCheckedProject,
) -> Result<ProjectSemanticBundle, NoirFrontendError> {
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

    model.semantic = extract_semantic_model(context, checked.crate_id(), &span_mapper);
    model.call_graph =
        filter_edges_to_user_symbols(call_edges_from_semantic(&model.semantic), &model.symbols);

    model.normalize();

    Ok(ProjectSemanticBundle { project: model })
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

    use super::{build_project_model, build_project_semantic_bundle};

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

    #[test]
    fn semantic_bundle_extracts_nodes_for_minimal_fixture() {
        let root = fixture_root();
        let bundle = run_with_large_stack(move || {
            build_project_semantic_bundle(&root, &root.join("src/main.nr"))
                .expect("fixture should build semantic bundle")
        });

        let semantic = bundle.semantic_model();
        assert!(!semantic.functions.is_empty());
        assert!(!semantic.expressions.is_empty());
        assert!(!semantic.statements.is_empty());
        assert!(!semantic.cfg_blocks.is_empty());
        assert!(!semantic.cfg_edges.is_empty());
        assert!(!semantic.dfg_edges.is_empty());
        assert!(!semantic.call_sites.is_empty());
    }

    #[test]
    fn semantic_bundle_captures_definition_to_identifier_use_edges() {
        let root = fixture_root();
        let bundle = run_with_large_stack(move || {
            build_project_semantic_bundle(&root, &root.join("src/main.nr"))
                .expect("fixture should build semantic bundle")
        });

        let semantic = bundle.semantic_model();
        assert!(
            semantic.dfg_edges.iter().any(|edge| {
                edge.from_node_id.starts_with("def::") && edge.to_node_id.starts_with("expr::")
            }),
            "expected at least one def->expr use edge in semantic DFG"
        );
    }
}
