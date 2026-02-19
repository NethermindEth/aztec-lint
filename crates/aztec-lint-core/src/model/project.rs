use serde::{Deserialize, Serialize};

use crate::model::Span;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectModel {
    pub ast_ids: Vec<String>,
    pub symbols: Vec<SymbolRef>,
    pub type_refs: Vec<TypeRef>,
    pub call_graph: Vec<CallEdge>,
    pub module_graph: Vec<ModuleEdge>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SymbolRef {
    pub symbol_id: String,
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Struct,
    Module,
    Import,
    Local,
    Global,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TypeRef {
    pub symbol_id: String,
    pub type_repr: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CallEdge {
    pub caller_symbol_id: String,
    pub callee_symbol_id: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModuleEdge {
    pub from_module: String,
    pub to_module: String,
}
