use aztec_lint_core::config::AztecConfig;
use aztec_lint_core::diagnostics::normalize_file_path;
use aztec_lint_core::model::{
    AztecModel, ContractModel, EnqueueSite, Entrypoint, EntrypointKind, SemanticSite, Span,
    StorageStruct,
};

use crate::detect::SourceUnit;
use crate::patterns::{
    contains_note_read, contains_note_write, contains_nullifier_emit, contains_public_sink,
    extract_enqueue_target_function, has_attribute, has_external_kind, is_contract_start,
    is_function_start, is_same_contract_enqueue, is_struct_start, looks_like_enqueue,
    normalize_line,
};

pub fn build_aztec_model(sources: &[SourceUnit], config: &AztecConfig) -> AztecModel {
    let mut model = AztecModel::default();

    for source in sources {
        build_for_source(source, config, &mut model);
    }

    model.contracts.sort_by_key(|contract| {
        (
            contract.span.file.clone(),
            contract.span.start,
            contract.name.clone(),
        )
    });
    model
        .contracts
        .dedup_by(|left, right| left.contract_id == right.contract_id);

    model.entrypoints.sort_by_key(|entry| {
        (
            entry.contract_id.clone(),
            entry.function_symbol_id.clone(),
            format!("{:?}", entry.kind),
            entry.span.file.clone(),
            entry.span.start,
        )
    });
    model.entrypoints.dedup_by(|left, right| {
        left.contract_id == right.contract_id
            && left.function_symbol_id == right.function_symbol_id
            && left.kind == right.kind
            && left.span.start == right.span.start
    });

    model.storage_structs.sort_by_key(|item| {
        (
            item.contract_id.clone(),
            item.struct_symbol_id.clone(),
            item.span.file.clone(),
            item.span.start,
        )
    });
    model
        .storage_structs
        .dedup_by(|left, right| left.struct_symbol_id == right.struct_symbol_id);

    sort_sites(&mut model.note_read_sites);
    sort_sites(&mut model.note_write_sites);
    sort_sites(&mut model.nullifier_emit_sites);
    sort_sites(&mut model.public_sinks);

    model.enqueue_sites.sort_by_key(|site| {
        (
            site.source_contract_id.clone(),
            site.source_function_symbol_id.clone(),
            site.target_contract_id.clone(),
            site.target_function_name.clone(),
            site.span.file.clone(),
            site.span.start,
        )
    });
    model.enqueue_sites.dedup();

    model
}

fn build_for_source(source: &SourceUnit, config: &AztecConfig, model: &mut AztecModel) {
    let mut pending_attributes = Vec::<String>::new();
    let mut current_contract: Option<(String, usize)> = None;
    let mut current_function: Option<(String, usize)> = None;
    let mut brace_depth = 0usize;
    let mut offset = 0usize;

    for line in source.text.lines() {
        let trimmed = line.trim();
        let (inline_attributes, code_after_attributes) = split_inline_attributes(trimmed);
        pending_attributes.extend(inline_attributes);

        if current_contract.is_none()
            && pending_attributes
                .iter()
                .any(|attr| has_attribute(attr, &config.contract_attribute))
            && let Some(contract_name) = is_contract_start(code_after_attributes)
        {
            let contract_id = format!("{}::{contract_name}", normalize_file_path(&source.path));
            model.contracts.push(ContractModel {
                contract_id: contract_id.clone(),
                name: contract_name,
                span: line_span(source, offset, line.len()),
            });
            current_contract = Some((contract_id, brace_depth + line.matches('{').count()));
            pending_attributes.clear();
        }

        if let Some((contract_id, _)) = current_contract.clone()
            && pending_attributes
                .iter()
                .any(|attr| has_attribute(attr, &config.storage_attribute))
            && let Some(struct_name) = is_struct_start(code_after_attributes)
        {
            let struct_symbol_id = format!("{contract_id}::struct::{struct_name}");
            model.storage_structs.push(StorageStruct {
                contract_id,
                struct_symbol_id,
                span: line_span(source, offset, line.len()),
            });
            pending_attributes.clear();
        }

        if let Some(function_name) = is_function_start(code_after_attributes) {
            let Some((contract_id, _)) = current_contract.clone() else {
                pending_attributes.clear();
                offset += line.len() + 1;
                brace_depth = update_depth(brace_depth, line);
                continue;
            };

            let function_id = format!("{contract_id}::fn::{function_name}");
            current_function = Some((function_id.clone(), brace_depth + line.matches('{').count()));

            let mut kinds = Vec::<EntrypointKind>::new();
            if pending_attributes
                .iter()
                .any(|attr| has_external_kind(attr, "public", config))
            {
                kinds.push(EntrypointKind::Public);
            }
            if pending_attributes
                .iter()
                .any(|attr| has_external_kind(attr, "private", config))
            {
                kinds.push(EntrypointKind::Private);
            }
            if pending_attributes
                .iter()
                .any(|attr| has_attribute(attr, &config.initializer_attribute))
            {
                kinds.push(EntrypointKind::Initializer);
            }
            if pending_attributes
                .iter()
                .any(|attr| has_attribute(attr, &config.only_self_attribute))
            {
                kinds.push(EntrypointKind::OnlySelf);
            }
            if kinds.is_empty() {
                kinds.push(EntrypointKind::Utility);
            }

            for kind in kinds {
                model.entrypoints.push(Entrypoint {
                    contract_id: contract_id.clone(),
                    function_symbol_id: function_id.clone(),
                    kind,
                    span: line_span(source, offset, line.len()),
                });
            }

            pending_attributes.clear();
        }

        if let (Some((contract_id, _)), Some((function_id, _))) =
            (current_contract.clone(), current_function.clone())
        {
            let span = line_span(source, offset, line.len());
            if contains_note_read(line, config) {
                model.note_read_sites.push(SemanticSite {
                    contract_id: contract_id.clone(),
                    function_symbol_id: function_id.clone(),
                    span: span.clone(),
                });
            }
            if contains_note_write(line) {
                model.note_write_sites.push(SemanticSite {
                    contract_id: contract_id.clone(),
                    function_symbol_id: function_id.clone(),
                    span: span.clone(),
                });
            }
            if contains_nullifier_emit(line, config) {
                model.nullifier_emit_sites.push(SemanticSite {
                    contract_id: contract_id.clone(),
                    function_symbol_id: function_id.clone(),
                    span: span.clone(),
                });
            }
            if contains_public_sink(line) {
                model.public_sinks.push(SemanticSite {
                    contract_id: contract_id.clone(),
                    function_symbol_id: function_id.clone(),
                    span: span.clone(),
                });
            }
            if looks_like_enqueue(line, config) {
                model.enqueue_sites.push(EnqueueSite {
                    source_contract_id: contract_id.clone(),
                    source_function_symbol_id: function_id,
                    target_contract_id: if is_same_contract_enqueue(line) {
                        Some(contract_id)
                    } else {
                        None
                    },
                    target_function_name: extract_enqueue_target_function(line).unwrap_or_default(),
                    span,
                });
            }
        }

        brace_depth = update_depth(brace_depth, line);

        if let Some((_, function_depth)) = current_function.clone()
            && brace_depth < function_depth
        {
            current_function = None;
        }
        if let Some((_, contract_depth)) = current_contract.clone()
            && brace_depth < contract_depth
        {
            current_contract = None;
        }

        if inline_attributes_and_code_should_clear_pending(
            code_after_attributes,
            &pending_attributes,
        ) {
            pending_attributes.clear();
        }

        offset += line.len() + 1;
    }
}

fn split_inline_attributes(line: &str) -> (Vec<String>, &str) {
    let mut attrs = Vec::<String>::new();
    let mut remaining = line.trim_start();

    while remaining.starts_with("#[") {
        let Some(close) = remaining.find(']') else {
            break;
        };
        attrs.push(remaining[..=close].trim().to_string());
        remaining = remaining[close + 1..].trim_start();
    }

    (attrs, remaining)
}

fn inline_attributes_and_code_should_clear_pending(
    code_after_attributes: &str,
    pending_attributes: &[String],
) -> bool {
    !pending_attributes.is_empty() && !normalize_line(code_after_attributes).is_empty()
}

fn line_span(source: &SourceUnit, offset: usize, line_len: usize) -> Span {
    let line = source.text[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    Span::new(
        normalize_file_path(&source.path),
        u32::try_from(offset).unwrap_or(u32::MAX),
        u32::try_from(offset + line_len).unwrap_or(u32::MAX),
        u32::try_from(line).unwrap_or(u32::MAX),
        1,
    )
}

fn update_depth(current: usize, line: &str) -> usize {
    let opens = line.bytes().filter(|byte| *byte == b'{').count();
    let closes = line.bytes().filter(|byte| *byte == b'}').count();
    current.saturating_add(opens).saturating_sub(closes)
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

#[cfg(test)]
mod tests {
    use aztec_lint_core::config::AztecConfig;
    use aztec_lint_core::model::EntrypointKind;

    use crate::detect::SourceUnit;

    use super::build_aztec_model;

    #[test]
    fn builds_model_with_contract_entrypoints_and_enqueue_sites() {
        let config = AztecConfig::default();
        let sources = vec![SourceUnit::new(
            "src/main.nr",
            r#"
#[aztec]
pub contract Token {
    #[external("private")]
    fn bridge() {
        let notes = self.notes.get_notes();
        self.enqueue(Contract::at(self.context.this_address()).mint_public(notes));
    }

    #[external("public")]
    fn mint_public(value: Field) {
        emit(value);
    }
}
"#,
        )];

        let model = build_aztec_model(&sources, &config);

        assert_eq!(model.contracts.len(), 1);
        assert!(!model.entrypoints.is_empty());
        assert!(
            model
                .entrypoints
                .iter()
                .any(|entry| entry.kind == EntrypointKind::Private)
        );
        assert!(!model.note_read_sites.is_empty());
        assert!(!model.enqueue_sites.is_empty());
    }

    #[test]
    fn parses_same_line_attributes_and_storage_structs() {
        let config = AztecConfig::default();
        let sources = vec![SourceUnit::new(
            "src/main.nr",
            r#"
#[aztec] pub contract Vault {
    #[storage] pub struct Storage {
        value: Field,
    }

    #[external("public")] #[only_self] fn mint_public(value: Field) {
        emit(value);
    }
}
"#,
        )];

        let model = build_aztec_model(&sources, &config);

        assert_eq!(model.contracts.len(), 1);
        assert_eq!(model.storage_structs.len(), 1);
        assert!(
            model
                .entrypoints
                .iter()
                .any(|entry| entry.kind == EntrypointKind::Public)
        );
        assert!(
            model
                .entrypoints
                .iter()
                .any(|entry| entry.kind == EntrypointKind::OnlySelf)
        );
    }

    #[test]
    fn keeps_attribute_context_when_line_has_trailing_comment() {
        let config = AztecConfig::default();
        let sources = vec![SourceUnit::new(
            "src/main.nr",
            r#"
#[aztec] // comment
pub contract Vault {
    #[external("private")] // comment
    fn bridge() {
        let notes = self.notes.get_notes();
    }
}
"#,
        )];

        let model = build_aztec_model(&sources, &config);
        assert_eq!(model.contracts.len(), 1);
        assert!(
            model
                .entrypoints
                .iter()
                .any(|entry| entry.kind == EntrypointKind::Private)
        );
    }
}
