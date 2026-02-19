use serde::{Deserialize, Serialize};

use crate::model::Span;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AztecModel {
    pub contracts: Vec<ContractModel>,
    pub entrypoints: Vec<Entrypoint>,
    pub storage_structs: Vec<StorageStruct>,
    pub note_read_sites: Vec<SemanticSite>,
    pub note_write_sites: Vec<SemanticSite>,
    pub nullifier_emit_sites: Vec<SemanticSite>,
    pub public_sinks: Vec<SemanticSite>,
    pub enqueue_sites: Vec<EnqueueSite>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContractModel {
    pub contract_id: String,
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Entrypoint {
    pub contract_id: String,
    pub function_symbol_id: String,
    pub kind: EntrypointKind,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntrypointKind {
    Public,
    Private,
    Initializer,
    OnlySelf,
    Utility,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StorageStruct {
    pub contract_id: String,
    pub struct_symbol_id: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SemanticSite {
    pub contract_id: String,
    pub function_symbol_id: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnqueueSite {
    pub source_contract_id: String,
    pub source_function_symbol_id: String,
    pub target_contract_id: Option<String>,
    pub target_function_name: String,
    pub span: Span,
}
