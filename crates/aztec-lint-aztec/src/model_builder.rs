use aztec_lint_core::config::AztecConfig;
use aztec_lint_core::diagnostics::normalize_file_path;
use aztec_lint_core::model::{
    AztecModel, ContractModel, EnqueueSite, Entrypoint, EntrypointKind, ExpressionCategory,
    SemanticModel, SemanticSite, Span, StatementCategory, StorageStruct,
};

use crate::detect::SourceUnit;
use crate::patterns::{
    contains_note_read, contains_note_write, contains_nullifier_emit, contains_public_sink,
    extract_call_name, extract_enqueue_target_function, has_attribute, has_external_kind,
    is_contract_start, is_enqueue_call_name, is_function_start, is_note_getter_call_name,
    is_note_write_call_name, is_nullifier_call_name, is_public_sink_call_name,
    is_same_contract_enqueue, is_struct_start, looks_like_enqueue, normalize_line,
};

pub fn build_aztec_model(sources: &[SourceUnit], config: &AztecConfig) -> AztecModel {
    build_aztec_model_with_semantic(sources, config, None)
}

pub fn build_aztec_model_with_semantic(
    sources: &[SourceUnit],
    config: &AztecConfig,
    semantic: Option<&SemanticModel>,
) -> AztecModel {
    let mut model = AztecModel::default();

    for source in sources {
        build_for_source(source, config, semantic, &mut model);
    }

    model.normalize();
    model
}

fn build_for_source(
    source: &SourceUnit,
    config: &AztecConfig,
    semantic: Option<&SemanticModel>,
    model: &mut AztecModel,
) {
    let mut pending_attributes = Vec::<String>::new();
    let mut current_contract: Option<(String, usize)> = None;
    let mut current_function: Option<ActiveFunction> = None;
    let mut function_scopes = Vec::<FunctionScope>::new();
    let mut brace_depth = 0usize;
    let mut offset = 0usize;
    let mut fallback_note_read_sites = Vec::<SemanticSite>::new();
    let mut fallback_note_write_sites = Vec::<SemanticSite>::new();
    let mut fallback_nullifier_emit_sites = Vec::<SemanticSite>::new();
    let mut fallback_public_sinks = Vec::<SemanticSite>::new();
    let mut fallback_enqueue_sites = Vec::<EnqueueSite>::new();

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
            current_function = Some(ActiveFunction {
                scope: FunctionScope {
                    contract_id: contract_id.clone(),
                    function_symbol_id: function_id.clone(),
                    function_name: function_name.clone(),
                    start_offset: offset,
                    end_offset: source.text.len(),
                },
                end_depth: brace_depth + line.matches('{').count(),
            });

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

        if let (Some((contract_id, _)), Some(function)) =
            (current_contract.clone(), current_function.as_ref())
        {
            let span = line_span(source, offset, line.len());
            if contains_note_read(line, config) {
                fallback_note_read_sites.push(SemanticSite {
                    contract_id: contract_id.clone(),
                    function_symbol_id: function.scope.function_symbol_id.clone(),
                    span: span.clone(),
                });
            }
            if contains_note_write(line) {
                fallback_note_write_sites.push(SemanticSite {
                    contract_id: contract_id.clone(),
                    function_symbol_id: function.scope.function_symbol_id.clone(),
                    span: span.clone(),
                });
            }
            if contains_nullifier_emit(line, config) {
                fallback_nullifier_emit_sites.push(SemanticSite {
                    contract_id: contract_id.clone(),
                    function_symbol_id: function.scope.function_symbol_id.clone(),
                    span: span.clone(),
                });
            }
            if contains_public_sink(line) {
                fallback_public_sinks.push(SemanticSite {
                    contract_id: contract_id.clone(),
                    function_symbol_id: function.scope.function_symbol_id.clone(),
                    span: span.clone(),
                });
            }
            if looks_like_enqueue(line, config) {
                fallback_enqueue_sites.push(EnqueueSite {
                    source_contract_id: contract_id.clone(),
                    source_function_symbol_id: function.scope.function_symbol_id.clone(),
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

        if let Some(active) = current_function.clone()
            && brace_depth < active.end_depth
        {
            function_scopes.push(FunctionScope {
                end_offset: offset + line.len(),
                ..active.scope
            });
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

    if let Some(active) = current_function.take() {
        function_scopes.push(active.scope);
    }

    let used_semantic = semantic
        .filter(|semantic_model| semantic_model_available(semantic_model))
        .is_some_and(|semantic_model| {
            add_semantic_sites_for_source(source, config, semantic_model, &function_scopes, model)
        });

    if !used_semantic {
        model.note_read_sites.extend(fallback_note_read_sites);
        model.note_write_sites.extend(fallback_note_write_sites);
        model
            .nullifier_emit_sites
            .extend(fallback_nullifier_emit_sites);
        model.public_sinks.extend(fallback_public_sinks);
        model.enqueue_sites.extend(fallback_enqueue_sites);
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

#[derive(Clone, Debug)]
struct FunctionScope {
    contract_id: String,
    function_symbol_id: String,
    function_name: String,
    start_offset: usize,
    end_offset: usize,
}

#[derive(Clone, Debug)]
struct ActiveFunction {
    scope: FunctionScope,
    end_depth: usize,
}

fn semantic_model_available(semantic: &SemanticModel) -> bool {
    !semantic.call_sites.is_empty()
        || semantic
            .expressions
            .iter()
            .any(|expression| expression.category == ExpressionCategory::Call)
        || semantic
            .statements
            .iter()
            .any(|statement| statement.category == StatementCategory::Return)
}

fn add_semantic_sites_for_source(
    source: &SourceUnit,
    config: &AztecConfig,
    semantic: &SemanticModel,
    function_scopes: &[FunctionScope],
    model: &mut AztecModel,
) -> bool {
    let normalized_file = normalize_file_path(&source.path);
    let semantic_names = semantic
        .functions
        .iter()
        .map(|function| (function.symbol_id.clone(), function.name.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut scope_by_semantic_symbol = std::collections::BTreeMap::<String, FunctionScope>::new();
    for scope in function_scopes {
        if let Some(semantic_symbol_id) = match_semantic_function(scope, semantic, &normalized_file)
        {
            scope_by_semantic_symbol.insert(semantic_symbol_id, scope.clone());
        }
    }
    if scope_by_semantic_symbol.is_empty() {
        return false;
    }
    let mut saw_semantic_signal = false;
    let mut covered_calls = std::collections::BTreeSet::<(String, u32, u32)>::new();

    for call_site in semantic.call_sites.iter().filter(|call_site| {
        normalize_file_path(&call_site.span.file) == normalized_file
            && scope_by_semantic_symbol.contains_key(&call_site.function_symbol_id)
    }) {
        saw_semantic_signal = true;
        let Some(scope) = scope_by_semantic_symbol.get(&call_site.function_symbol_id) else {
            continue;
        };
        covered_calls.insert((
            call_site.function_symbol_id.clone(),
            call_site.span.start,
            call_site.span.end,
        ));
        let Some(call_text) = span_text(source, &call_site.span) else {
            continue;
        };
        let callee_name = semantic_names
            .get(&call_site.callee_symbol_id)
            .cloned()
            .or_else(|| extract_call_name(call_text));
        let Some(callee_name) = callee_name else {
            continue;
        };

        let site = SemanticSite {
            contract_id: scope.contract_id.clone(),
            function_symbol_id: scope.function_symbol_id.clone(),
            span: call_site.span.clone(),
        };

        if is_note_getter_call_name(&callee_name, config) {
            model.note_read_sites.push(site.clone());
        }
        if is_note_write_call_name(&callee_name)
            && (call_text.contains("deliver(") || call_text.contains("ONCHAIN_CONSTRAINED"))
        {
            model.note_write_sites.push(site.clone());
        }
        if is_nullifier_call_name(&callee_name, config) {
            model.nullifier_emit_sites.push(site.clone());
        }
        if is_public_sink_call_name(&callee_name) {
            model.public_sinks.push(site.clone());
        }
        if is_enqueue_call_name(&callee_name, config) {
            model.enqueue_sites.push(EnqueueSite {
                source_contract_id: scope.contract_id.clone(),
                source_function_symbol_id: scope.function_symbol_id.clone(),
                target_contract_id: if is_same_contract_enqueue(call_text) {
                    Some(scope.contract_id.clone())
                } else {
                    None
                },
                target_function_name: extract_enqueue_target_function(call_text)
                    .unwrap_or_default(),
                span: call_site.span.clone(),
            });
        }
    }

    for expression in semantic.expressions.iter().filter(|expression| {
        expression.category == ExpressionCategory::Call
            && normalize_file_path(&expression.span.file) == normalized_file
            && scope_by_semantic_symbol.contains_key(&expression.function_symbol_id)
    }) {
        saw_semantic_signal = true;
        if covered_calls.contains(&(
            expression.function_symbol_id.clone(),
            expression.span.start,
            expression.span.end,
        )) {
            continue;
        }
        let Some(scope) = scope_by_semantic_symbol.get(&expression.function_symbol_id) else {
            continue;
        };
        let Some(call_text) = span_text(source, &expression.span) else {
            continue;
        };
        let Some(callee_name) = extract_call_name(call_text) else {
            continue;
        };

        let site = SemanticSite {
            contract_id: scope.contract_id.clone(),
            function_symbol_id: scope.function_symbol_id.clone(),
            span: expression.span.clone(),
        };

        if is_note_getter_call_name(&callee_name, config) {
            model.note_read_sites.push(site.clone());
        }
        if is_note_write_call_name(&callee_name)
            && (call_text.contains("deliver(") || call_text.contains("ONCHAIN_CONSTRAINED"))
        {
            model.note_write_sites.push(site.clone());
        }
        if is_nullifier_call_name(&callee_name, config) {
            model.nullifier_emit_sites.push(site.clone());
        }
        if is_public_sink_call_name(&callee_name) {
            model.public_sinks.push(site.clone());
        }
        if is_enqueue_call_name(&callee_name, config) {
            model.enqueue_sites.push(EnqueueSite {
                source_contract_id: scope.contract_id.clone(),
                source_function_symbol_id: scope.function_symbol_id.clone(),
                target_contract_id: if is_same_contract_enqueue(call_text) {
                    Some(scope.contract_id.clone())
                } else {
                    None
                },
                target_function_name: extract_enqueue_target_function(call_text)
                    .unwrap_or_default(),
                span: expression.span.clone(),
            });
        }
    }

    for statement in semantic.statements.iter().filter(|statement| {
        statement.category == StatementCategory::Return
            && normalize_file_path(&statement.span.file) == normalized_file
            && scope_by_semantic_symbol.contains_key(&statement.function_symbol_id)
    }) {
        saw_semantic_signal = true;
        let Some(scope) = scope_by_semantic_symbol.get(&statement.function_symbol_id) else {
            continue;
        };
        model.public_sinks.push(SemanticSite {
            contract_id: scope.contract_id.clone(),
            function_symbol_id: scope.function_symbol_id.clone(),
            span: statement.span.clone(),
        });
    }

    saw_semantic_signal
}

fn match_semantic_function(
    scope: &FunctionScope,
    semantic: &SemanticModel,
    normalized_file: &str,
) -> Option<String> {
    let mut candidates = semantic
        .functions
        .iter()
        .filter(|function| function.name == scope.function_name)
        .filter(|function| normalize_file_path(&function.span.file) == normalized_file)
        .filter_map(|function| {
            let start = usize::try_from(function.span.start).ok()?;
            Some((function.symbol_id.clone(), start))
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }

    candidates.sort_by_key(|(_, start)| *start);
    if let Some((symbol_id, _)) = candidates
        .iter()
        .find(|(_, start)| *start >= scope.start_offset && *start <= scope.end_offset)
    {
        return Some(symbol_id.clone());
    }

    candidates
        .into_iter()
        .min_by_key(|(_, start)| start.abs_diff(scope.start_offset))
        .map(|(symbol_id, _)| symbol_id)
}

fn span_text<'a>(source: &'a SourceUnit, span: &Span) -> Option<&'a str> {
    let start = usize::try_from(span.start).ok()?;
    let end = usize::try_from(span.end).ok()?;
    if start >= end || end > source.text.len() {
        return None;
    }
    source.text.get(start..end)
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::config::AztecConfig;
    use aztec_lint_core::model::EntrypointKind;
    use aztec_lint_core::model::{
        CallSite, ExpressionCategory, GuardNode, ProjectModel, SemanticExpression,
        SemanticFunction, SemanticModel, SemanticStatement, Span, StatementCategory, TypeCategory,
    };

    use crate::detect::SourceUnit;

    use super::{build_aztec_model, build_aztec_model_with_semantic};

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

    #[test]
    fn prefers_semantic_call_site_classification_when_available() {
        let config = AztecConfig::default();
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge() {
        let notes = alias_read();
        emit(notes);
    }
}
"#;
        let alias_start = source.find("alias_read(").expect("alias call should exist");
        let alias_end = alias_start + "alias_read()".len();
        let emit_start = source.find("emit(notes)").expect("emit call should exist");
        let emit_end = emit_start + "emit(notes)".len();
        let bridge_start = source.find("bridge").expect("bridge fn should exist");

        let semantic = SemanticModel {
            functions: vec![
                SemanticFunction {
                    symbol_id: "fn::bridge".to_string(),
                    name: "bridge".to_string(),
                    module_symbol_id: "module::main".to_string(),
                    return_type_repr: "()".to_string(),
                    return_type_category: TypeCategory::Unknown,
                    parameter_types: Vec::new(),
                    is_entrypoint: false,
                    is_unconstrained: false,
                    span: Span::new(
                        "src/main.nr",
                        u32::try_from(bridge_start).unwrap_or(u32::MAX),
                        u32::try_from(bridge_start + "bridge".len()).unwrap_or(u32::MAX),
                        5,
                        8,
                    ),
                },
                SemanticFunction {
                    symbol_id: "fn::get_notes_runtime".to_string(),
                    name: "get_notes".to_string(),
                    module_symbol_id: "module::deps".to_string(),
                    return_type_repr: "Field".to_string(),
                    return_type_category: TypeCategory::Field,
                    parameter_types: Vec::new(),
                    is_entrypoint: false,
                    is_unconstrained: false,
                    span: Span::new("deps.nr", 0, 0, 1, 1),
                },
                SemanticFunction {
                    symbol_id: "fn::emit_runtime".to_string(),
                    name: "emit".to_string(),
                    module_symbol_id: "module::deps".to_string(),
                    return_type_repr: "()".to_string(),
                    return_type_category: TypeCategory::Unknown,
                    parameter_types: vec!["Field".to_string()],
                    is_entrypoint: false,
                    is_unconstrained: false,
                    span: Span::new("deps.nr", 0, 0, 1, 1),
                },
            ],
            call_sites: vec![
                CallSite {
                    call_site_id: "call::read".to_string(),
                    function_symbol_id: "fn::bridge".to_string(),
                    callee_symbol_id: "fn::get_notes_runtime".to_string(),
                    expr_id: "expr::read".to_string(),
                    span: Span::new(
                        "src/main.nr",
                        u32::try_from(alias_start).unwrap_or(u32::MAX),
                        u32::try_from(alias_end).unwrap_or(u32::MAX),
                        6,
                        20,
                    ),
                },
                CallSite {
                    call_site_id: "call::emit".to_string(),
                    function_symbol_id: "fn::bridge".to_string(),
                    callee_symbol_id: "fn::emit_runtime".to_string(),
                    expr_id: "expr::emit".to_string(),
                    span: Span::new(
                        "src/main.nr",
                        u32::try_from(emit_start).unwrap_or(u32::MAX),
                        u32::try_from(emit_end).unwrap_or(u32::MAX),
                        7,
                        9,
                    ),
                },
            ],
            expressions: vec![SemanticExpression {
                expr_id: "expr::emit".to_string(),
                function_symbol_id: "fn::bridge".to_string(),
                category: ExpressionCategory::Call,
                type_category: TypeCategory::Unknown,
                type_repr: "()".to_string(),
                span: Span::new(
                    "src/main.nr",
                    u32::try_from(emit_start).unwrap_or(u32::MAX),
                    u32::try_from(emit_end).unwrap_or(u32::MAX),
                    7,
                    9,
                ),
            }],
            statements: vec![SemanticStatement {
                stmt_id: "stmt::noop".to_string(),
                function_symbol_id: "fn::bridge".to_string(),
                category: StatementCategory::Expression,
                span: Span::new(
                    "src/main.nr",
                    u32::try_from(emit_start).unwrap_or(u32::MAX),
                    u32::try_from(emit_end).unwrap_or(u32::MAX),
                    7,
                    9,
                ),
            }],
            guard_nodes: vec![GuardNode {
                guard_id: "guard::noop".to_string(),
                function_symbol_id: "fn::bridge".to_string(),
                kind: aztec_lint_core::model::GuardKind::Assert,
                guarded_expr_id: None,
                span: Span::new("src/main.nr", 0, 0, 1, 1),
            }],
            ..SemanticModel::default()
        };
        let mut project = ProjectModel {
            semantic,
            ..ProjectModel::default()
        };
        project.normalize();

        let model = build_aztec_model_with_semantic(
            &[SourceUnit::new("src/main.nr", source)],
            &config,
            Some(&project.semantic),
        );

        assert_eq!(model.contracts.len(), 1);
        assert_eq!(model.note_read_sites.len(), 1);
        assert_eq!(model.public_sinks.len(), 1);
    }

    #[test]
    fn classifies_semantic_call_expressions_when_callee_resolution_is_missing() {
        let config = AztecConfig::default();
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge() {
        let notes = self.notes.get_notes();
    }
}
"#;
        let bridge_start = source.find("bridge").expect("bridge fn should exist");
        let call_start = source
            .find("self.notes.get_notes()")
            .expect("call should exist");
        let call_end = call_start + "self.notes.get_notes()".len();

        let semantic = SemanticModel {
            functions: vec![SemanticFunction {
                symbol_id: "fn::bridge".to_string(),
                name: "bridge".to_string(),
                module_symbol_id: "module::main".to_string(),
                return_type_repr: "()".to_string(),
                return_type_category: TypeCategory::Unknown,
                parameter_types: Vec::new(),
                is_entrypoint: false,
                is_unconstrained: false,
                span: Span::new(
                    "src/main.nr",
                    u32::try_from(bridge_start).unwrap_or(u32::MAX),
                    u32::try_from(bridge_start + "bridge".len()).unwrap_or(u32::MAX),
                    5,
                    8,
                ),
            }],
            expressions: vec![SemanticExpression {
                expr_id: "expr::call::notes".to_string(),
                function_symbol_id: "fn::bridge".to_string(),
                category: ExpressionCategory::Call,
                type_category: TypeCategory::Unknown,
                type_repr: "Field".to_string(),
                span: Span::new(
                    "src/main.nr",
                    u32::try_from(call_start).unwrap_or(u32::MAX),
                    u32::try_from(call_end).unwrap_or(u32::MAX),
                    6,
                    20,
                ),
            }],
            statements: vec![SemanticStatement {
                stmt_id: "stmt::call".to_string(),
                function_symbol_id: "fn::bridge".to_string(),
                category: StatementCategory::Expression,
                span: Span::new(
                    "src/main.nr",
                    u32::try_from(call_start).unwrap_or(u32::MAX),
                    u32::try_from(call_end).unwrap_or(u32::MAX),
                    6,
                    20,
                ),
            }],
            ..SemanticModel::default()
        };
        let mut project = ProjectModel {
            semantic,
            ..ProjectModel::default()
        };
        project.normalize();

        let model = build_aztec_model_with_semantic(
            &[SourceUnit::new("src/main.nr", source)],
            &config,
            Some(&project.semantic),
        );

        assert_eq!(model.contracts.len(), 1);
        assert_eq!(model.note_read_sites.len(), 1);
    }
}
