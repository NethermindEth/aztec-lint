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

impl AztecModel {
    pub fn normalize(&mut self) {
        self.contracts.sort_by_key(|contract| {
            (
                contract.span.file.clone(),
                contract.span.start,
                contract.name.clone(),
            )
        });
        self.contracts
            .dedup_by(|left, right| left.contract_id == right.contract_id);

        self.entrypoints.sort_by_key(|entry| {
            (
                entry.contract_id.clone(),
                entry.function_symbol_id.clone(),
                format!("{:?}", entry.kind),
                entry.span.file.clone(),
                entry.span.start,
            )
        });
        self.entrypoints.dedup_by(|left, right| {
            left.contract_id == right.contract_id
                && left.function_symbol_id == right.function_symbol_id
                && left.kind == right.kind
                && left.span.start == right.span.start
        });

        self.storage_structs.sort_by_key(|item| {
            (
                item.contract_id.clone(),
                item.struct_symbol_id.clone(),
                item.span.file.clone(),
                item.span.start,
            )
        });
        self.storage_structs
            .dedup_by(|left, right| left.struct_symbol_id == right.struct_symbol_id);

        sort_sites(&mut self.note_read_sites);
        sort_sites(&mut self.note_write_sites);
        sort_sites(&mut self.nullifier_emit_sites);
        sort_sites(&mut self.public_sinks);

        self.enqueue_sites.sort_by_key(|site| {
            (
                site.source_contract_id.clone(),
                site.source_function_symbol_id.clone(),
                site.target_contract_id.clone(),
                site.target_function_name.clone(),
                site.span.file.clone(),
                site.span.start,
            )
        });
        self.enqueue_sites.dedup();
    }
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

fn sort_sites(items: &mut Vec<SemanticSite>) {
    items.sort_by_key(|site| {
        (
            site.contract_id.clone(),
            site.function_symbol_id.clone(),
            site.span.file.clone(),
            site.span.start,
        )
    });
    items.dedup();
}
