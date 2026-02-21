use std::collections::{BTreeMap, BTreeSet};

use aztec_lint_core::config::AztecConfig;
use aztec_lint_core::diagnostics::normalize_file_path;
use aztec_lint_core::model::{
    AztecModel, CfgEdgeKind, EntrypointKind, ExpressionCategory, GuardKind, SemanticModel, Span,
    StatementCategory,
};

use crate::detect::SourceUnit;
use crate::patterns::{
    contains_note_read, extract_call_name, is_contract_start, is_enqueue_call_name,
    is_function_start, is_nullifier_call_name, is_public_sink_call_name, normalize_line,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TaintSourceKind {
    NoteRead,
    PrivateEntrypointParam,
    SecretState,
    UnconstrainedCall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TaintSinkKind {
    PublicOutput,
    PublicStorageWrite,
    EnqueuePublicCall,
    OracleArgument,
    LogEvent,
    NullifierOrCommitment,
    BranchCondition,
    HashOrSerialize,
    MerkleWitness,
    DebugLog,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineRecord {
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaintSource {
    pub variable: String,
    pub kind: TaintSourceKind,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Definition {
    pub variable: String,
    pub dependencies: BTreeSet<String>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuardSite {
    pub variable: String,
    pub span: Span,
    pub block_id: Option<String>,
    pub covered_nodes: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SinkSite {
    pub kind: TaintSinkKind,
    pub identifiers: BTreeSet<String>,
    pub span: Span,
    pub line: String,
    pub block_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionGraph {
    pub contract_id: String,
    pub function_symbol_id: String,
    pub is_private_entrypoint: bool,
    pub is_public_entrypoint: bool,
    pub semantic_function_symbol_id: Option<String>,
    pub lines: Vec<LineRecord>,
    pub definitions: Vec<Definition>,
    pub sources: Vec<TaintSource>,
    pub guards: Vec<GuardSite>,
    pub sinks: Vec<SinkSite>,
    pub dominators: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DefUseGraph {
    pub functions: Vec<FunctionGraph>,
}

pub fn build_def_use_graph(
    sources: &[SourceUnit],
    model: &AztecModel,
    config: &AztecConfig,
) -> DefUseGraph {
    build_def_use_graph_with_semantic(sources, model, None, config)
}

pub fn build_def_use_graph_with_semantic(
    sources: &[SourceUnit],
    model: &AztecModel,
    semantic: Option<&SemanticModel>,
    config: &AztecConfig,
) -> DefUseGraph {
    if let Some(semantic_model) = semantic
        && semantic_graph_available(semantic_model)
    {
        return build_semantic_def_use_graph(sources, model, semantic_model, config);
    }
    build_fallback_def_use_graph(sources, model, config)
}

fn semantic_graph_available(semantic: &SemanticModel) -> bool {
    !semantic.functions.is_empty()
        && !semantic.dfg_edges.is_empty()
        && !semantic.cfg_blocks.is_empty()
}

fn build_fallback_def_use_graph(
    sources: &[SourceUnit],
    model: &AztecModel,
    config: &AztecConfig,
) -> DefUseGraph {
    let mut graph = DefUseGraph::default();
    let entrypoint_kinds = model.entrypoints.iter().fold(
        BTreeMap::<String, Vec<EntrypointKind>>::new(),
        |mut map, item| {
            map.entry(item.function_symbol_id.clone())
                .or_default()
                .push(item.kind.clone());
            map
        },
    );
    let unconstrained_functions = collect_unconstrained_functions(sources);

    for source in sources {
        graph.functions.extend(build_source_functions(
            source,
            &entrypoint_kinds,
            &unconstrained_functions,
            config,
        ));
    }

    normalize_function_graphs(&mut graph.functions);
    graph
}

fn build_semantic_def_use_graph(
    sources: &[SourceUnit],
    model: &AztecModel,
    semantic: &SemanticModel,
    config: &AztecConfig,
) -> DefUseGraph {
    let source_map = sources
        .iter()
        .map(|source| (normalize_file_path(&source.path), source))
        .collect::<BTreeMap<_, _>>();
    let semantic_names = semantic
        .functions
        .iter()
        .map(|function| (function.symbol_id.clone(), function.name.clone()))
        .collect::<BTreeMap<_, _>>();
    let unconstrained_ids = semantic
        .functions
        .iter()
        .filter(|function| function.is_unconstrained)
        .map(|function| function.symbol_id.clone())
        .collect::<BTreeSet<_>>();
    let entrypoint_meta = match_entrypoints_to_semantic(model, semantic);

    let mut graph = DefUseGraph::default();
    for function in &semantic.functions {
        let normalized_file = normalize_file_path(&function.span.file);
        if !source_map.contains_key(&normalized_file) {
            continue;
        }

        let (contract_id, entrypoint_kinds) = entrypoint_meta
            .get(&function.symbol_id)
            .cloned()
            .unwrap_or_else(|| (format!("{normalized_file}::unknown_contract"), Vec::new()));

        let mut function_graph = FunctionGraph {
            contract_id,
            function_symbol_id: function.symbol_id.clone(),
            is_private_entrypoint: entrypoint_kinds.contains(&EntrypointKind::Private),
            is_public_entrypoint: entrypoint_kinds.contains(&EntrypointKind::Public),
            semantic_function_symbol_id: Some(function.symbol_id.clone()),
            lines: Vec::new(),
            definitions: Vec::new(),
            sources: Vec::new(),
            guards: Vec::new(),
            sinks: Vec::new(),
            dominators: semantic.cfg_dominators(&function.symbol_id),
        };

        add_semantic_definitions(function, semantic, &mut function_graph);
        add_semantic_sources(
            function,
            semantic,
            model,
            &source_map,
            &unconstrained_ids,
            &mut function_graph,
        );
        add_semantic_guards(function, semantic, &mut function_graph);
        add_semantic_sinks(
            function,
            semantic,
            model,
            config,
            &source_map,
            &semantic_names,
            &mut function_graph,
        );

        if !function_graph.definitions.is_empty()
            || !function_graph.sources.is_empty()
            || !function_graph.sinks.is_empty()
        {
            graph.functions.push(function_graph);
        }
    }

    normalize_function_graphs(&mut graph.functions);
    graph
}

fn normalize_function_graphs(functions: &mut [FunctionGraph]) {
    for function in functions.iter_mut() {
        function
            .lines
            .sort_by_key(|line| (line.span.file.clone(), line.span.start, line.text.clone()));
        function.lines.dedup();
        function.definitions.sort_by_key(|item| {
            (
                item.span.file.clone(),
                item.span.start,
                item.variable.clone(),
            )
        });
        function.sources.sort_by_key(|item| {
            (
                item.span.file.clone(),
                item.span.start,
                item.variable.clone(),
            )
        });
        function.guards.sort_by_key(|item| {
            (
                item.span.file.clone(),
                item.span.start,
                item.variable.clone(),
            )
        });
        function.sinks.sort_by_key(|item| {
            (
                item.span.file.clone(),
                item.span.start,
                format!("{:?}", item.kind),
            )
        });
        function.dominators = function
            .dominators
            .iter()
            .map(|(block, dominates)| (block.clone(), dominates.clone()))
            .collect();
    }

    functions.sort_by_key(|function| {
        (
            function.contract_id.clone(),
            function.function_symbol_id.clone(),
        )
    });
}

fn match_entrypoints_to_semantic(
    model: &AztecModel,
    semantic: &SemanticModel,
) -> BTreeMap<String, (String, Vec<EntrypointKind>)> {
    let mut out = BTreeMap::<String, (String, Vec<EntrypointKind>)>::new();

    for entrypoint in &model.entrypoints {
        let normalized_file = normalize_file_path(&entrypoint.span.file);
        let entrypoint_name = function_name_from_symbol_id(&entrypoint.function_symbol_id);
        let mut candidates = semantic
            .functions
            .iter()
            .filter(|function| normalize_file_path(&function.span.file) == normalized_file)
            .filter(|function| spans_overlap(&function.span, &entrypoint.span))
            .collect::<Vec<_>>();
        if let Some(name) = entrypoint_name {
            let named = candidates
                .iter()
                .copied()
                .filter(|function| function.name == name)
                .collect::<Vec<_>>();
            if !named.is_empty() {
                candidates = named;
            }
        }
        let Some(best) = candidates
            .into_iter()
            .min_by_key(|function| function.span.start.abs_diff(entrypoint.span.start))
        else {
            continue;
        };
        let entry = out
            .entry(best.symbol_id.clone())
            .or_insert_with(|| (entrypoint.contract_id.clone(), Vec::new()));
        if entry.0.is_empty() {
            entry.0 = entrypoint.contract_id.clone();
        }
        if !entry.1.contains(&entrypoint.kind) {
            entry.1.push(entrypoint.kind.clone());
        }
    }

    out
}

fn function_name_from_symbol_id(symbol_id: &str) -> Option<&str> {
    symbol_id.rsplit("::").next()
}

fn add_semantic_definitions(
    function: &aztec_lint_core::model::SemanticFunction,
    semantic: &SemanticModel,
    out: &mut FunctionGraph,
) {
    let mut node_spans = BTreeMap::<String, Span>::new();
    for expression in semantic
        .expressions
        .iter()
        .filter(|expression| expression.function_symbol_id == function.symbol_id)
    {
        node_spans.insert(expression.expr_id.clone(), expression.span.clone());
    }
    for statement in semantic
        .statements
        .iter()
        .filter(|statement| statement.function_symbol_id == function.symbol_id)
    {
        node_spans.insert(statement.stmt_id.clone(), statement.span.clone());
    }

    for edge in semantic
        .dfg_edges
        .iter()
        .filter(|edge| edge.function_symbol_id == function.symbol_id)
    {
        let span = node_spans
            .get(&edge.to_node_id)
            .cloned()
            .or_else(|| node_spans.get(&edge.from_node_id).cloned())
            .unwrap_or_else(|| function.span.clone());
        out.definitions.push(Definition {
            variable: edge.to_node_id.clone(),
            dependencies: BTreeSet::from([edge.from_node_id.clone()]),
            span,
        });
    }
}

fn add_semantic_sources(
    function: &aztec_lint_core::model::SemanticFunction,
    semantic: &SemanticModel,
    model: &AztecModel,
    sources: &BTreeMap<String, &SourceUnit>,
    unconstrained_ids: &BTreeSet<String>,
    out: &mut FunctionGraph,
) {
    let all_defs = semantic
        .dfg_edges
        .iter()
        .filter(|edge| edge.function_symbol_id == function.symbol_id)
        .flat_map(|edge| [&edge.from_node_id, &edge.to_node_id])
        .filter(|node_id| node_id.starts_with("def::"))
        .cloned()
        .collect::<BTreeSet<_>>();
    let stmt_defined_defs = semantic
        .dfg_edges
        .iter()
        .filter(|edge| edge.function_symbol_id == function.symbol_id)
        .filter(|edge| {
            edge.from_node_id.starts_with("stmt::") && edge.to_node_id.starts_with("def::")
        })
        .map(|edge| edge.to_node_id.clone())
        .collect::<BTreeSet<_>>();
    let parameter_defs = all_defs
        .difference(&stmt_defined_defs)
        .cloned()
        .collect::<BTreeSet<_>>();
    if out.is_private_entrypoint {
        for definition_id in parameter_defs {
            out.sources.push(TaintSource {
                variable: definition_id,
                kind: TaintSourceKind::PrivateEntrypointParam,
                span: function.span.clone(),
            });
        }
    }

    for site in model.note_read_sites.iter().filter(|site| {
        normalize_file_path(&site.span.file) == normalize_file_path(&function.span.file)
            && site.span.start >= function.span.start
            && site.span.end <= function.span.end
    }) {
        for node_id in sink_nodes_for_span(&function.symbol_id, &site.span, semantic) {
            out.sources.push(TaintSource {
                variable: node_id,
                kind: TaintSourceKind::NoteRead,
                span: site.span.clone(),
            });
        }
    }

    for call_site in semantic.call_sites.iter().filter(|call_site| {
        call_site.function_symbol_id == function.symbol_id
            && unconstrained_ids.contains(&call_site.callee_symbol_id)
    }) {
        out.sources.push(TaintSource {
            variable: call_site.expr_id.clone(),
            kind: TaintSourceKind::UnconstrainedCall,
            span: call_site.span.clone(),
        });
    }

    let normalized_file = normalize_file_path(&function.span.file);
    let Some(source) = sources.get(&normalized_file).copied() else {
        return;
    };
    for expression in semantic
        .expressions
        .iter()
        .filter(|expression| expression.function_symbol_id == function.symbol_id)
        .filter(|expression| expression.category == ExpressionCategory::MemberAccess)
    {
        let Some(text) = span_text(source, &expression.span) else {
            continue;
        };
        if text.contains("self.storage") {
            out.sources.push(TaintSource {
                variable: expression.expr_id.clone(),
                kind: TaintSourceKind::SecretState,
                span: expression.span.clone(),
            });
        }
    }
}

fn add_semantic_guards(
    function: &aztec_lint_core::model::SemanticFunction,
    semantic: &SemanticModel,
    out: &mut FunctionGraph,
) {
    let statement_block = semantic.statement_block_map(&function.symbol_id);
    let reverse_dfg = reverse_adjacency(&function.symbol_id, semantic);

    for guard in semantic
        .guard_nodes
        .iter()
        .filter(|guard| guard.function_symbol_id == function.symbol_id)
        .filter(|guard| {
            matches!(
                guard.kind,
                GuardKind::Assert | GuardKind::Constrain | GuardKind::Range
            )
        })
    {
        let Some(guarded_expr_id) = &guard.guarded_expr_id else {
            continue;
        };
        let mut covered_nodes = reverse_reachable_nodes(guarded_expr_id, &reverse_dfg);
        covered_nodes.insert(guarded_expr_id.clone());

        let block_id = innermost_statement_for_span(&function.symbol_id, &guard.span, semantic)
            .and_then(|statement| statement_block.get(&statement.stmt_id).cloned());

        out.guards.push(GuardSite {
            variable: guarded_expr_id.clone(),
            span: guard.span.clone(),
            block_id,
            covered_nodes,
        });
    }
}

fn add_semantic_sinks(
    function: &aztec_lint_core::model::SemanticFunction,
    semantic: &SemanticModel,
    model: &AztecModel,
    config: &AztecConfig,
    sources: &BTreeMap<String, &SourceUnit>,
    semantic_names: &BTreeMap<String, String>,
    out: &mut FunctionGraph,
) {
    let statement_block = semantic.statement_block_map(&function.symbol_id);
    let expr_to_statement = expression_statement_map(&function.symbol_id, semantic);
    let normalized_file = normalize_file_path(&function.span.file);
    let source = sources.get(&normalized_file).copied();
    let sink_context = SinkContext {
        function_symbol_id: &function.symbol_id,
        semantic,
        statement_block: &statement_block,
        expr_to_statement: &expr_to_statement,
        source,
    };

    for site in model.public_sinks.iter().filter(|site| {
        normalize_file_path(&site.span.file) == normalized_file
            && site.span.start >= function.span.start
            && site.span.end <= function.span.end
    }) {
        push_sink_for_site(TaintSinkKind::PublicOutput, &site.span, &sink_context, out);
    }
    for site in model.nullifier_emit_sites.iter().filter(|site| {
        normalize_file_path(&site.span.file) == normalized_file
            && site.span.start >= function.span.start
            && site.span.end <= function.span.end
    }) {
        push_sink_for_site(
            TaintSinkKind::NullifierOrCommitment,
            &site.span,
            &sink_context,
            out,
        );
    }
    for site in model.enqueue_sites.iter().filter(|site| {
        normalize_file_path(&site.span.file) == normalized_file
            && site.span.start >= function.span.start
            && site.span.end <= function.span.end
    }) {
        push_sink_for_site(
            TaintSinkKind::EnqueuePublicCall,
            &site.span,
            &sink_context,
            out,
        );
    }

    for block in semantic
        .cfg_blocks
        .iter()
        .filter(|block| block.function_symbol_id == function.symbol_id)
    {
        let has_branch = semantic.cfg_edges.iter().any(|edge| {
            edge.function_symbol_id == function.symbol_id
                && edge.from_block_id == block.block_id
                && matches!(
                    edge.kind,
                    CfgEdgeKind::TrueBranch | CfgEdgeKind::FalseBranch
                )
        });
        if !has_branch {
            continue;
        }
        for statement_id in &block.statement_ids {
            let span = semantic
                .statements
                .iter()
                .find(|statement| {
                    statement.function_symbol_id == function.symbol_id
                        && statement.stmt_id == *statement_id
                })
                .map(|statement| statement.span.clone())
                .unwrap_or_else(|| function.span.clone());
            out.sinks.push(SinkSite {
                kind: TaintSinkKind::BranchCondition,
                identifiers: BTreeSet::from([statement_id.clone()]),
                span: span.clone(),
                line: source
                    .and_then(|unit| span_text(unit, &span))
                    .unwrap_or("")
                    .to_string(),
                block_id: Some(block.block_id.clone()),
            });
        }
    }

    let mut covered_calls = BTreeSet::<String>::new();
    for call_site in semantic
        .call_sites
        .iter()
        .filter(|call_site| call_site.function_symbol_id == function.symbol_id)
    {
        covered_calls.insert(call_site.expr_id.clone());
        let call_name = semantic_names
            .get(&call_site.callee_symbol_id)
            .cloned()
            .or_else(|| {
                source
                    .and_then(|unit| span_text(unit, &call_site.span))
                    .and_then(extract_call_name)
            });
        let Some(call_name) = call_name else {
            continue;
        };
        for sink_kind in sink_kinds_for_call_name(&call_name, config) {
            push_call_sink(
                sink_kind,
                &call_site.expr_id,
                &call_site.span,
                &statement_block,
                &expr_to_statement,
                source,
                out,
            );
        }
    }

    for expression in semantic
        .expressions
        .iter()
        .filter(|expression| expression.function_symbol_id == function.symbol_id)
        .filter(|expression| expression.category == ExpressionCategory::Call)
    {
        if covered_calls.contains(&expression.expr_id) {
            continue;
        }
        let Some(call_name) = source
            .and_then(|unit| span_text(unit, &expression.span))
            .and_then(extract_call_name)
        else {
            continue;
        };
        for sink_kind in sink_kinds_for_call_name(&call_name, config) {
            push_call_sink(
                sink_kind,
                &expression.expr_id,
                &expression.span,
                &statement_block,
                &expr_to_statement,
                source,
                out,
            );
        }
    }

    for statement in semantic
        .statements
        .iter()
        .filter(|statement| statement.function_symbol_id == function.symbol_id)
        .filter(|statement| {
            matches!(
                statement.category,
                StatementCategory::Assign | StatementCategory::Expression
            )
        })
    {
        let Some(text) = source.and_then(|unit| span_text(unit, &statement.span)) else {
            continue;
        };
        if text.contains("self.storage")
            && (text.contains('=') || text.contains(".write(") || text.contains(".set("))
        {
            out.sinks.push(SinkSite {
                kind: TaintSinkKind::PublicStorageWrite,
                identifiers: BTreeSet::from([statement.stmt_id.clone()]),
                span: statement.span.clone(),
                line: text.to_string(),
                block_id: statement_block.get(&statement.stmt_id).cloned(),
            });
        }
    }
}

fn sink_kinds_for_call_name(name: &str, config: &AztecConfig) -> Vec<TaintSinkKind> {
    let mut kinds = Vec::<TaintSinkKind>::new();
    let lower = name.to_ascii_lowercase();

    if is_public_sink_call_name(name) {
        kinds.push(TaintSinkKind::PublicOutput);
    }
    if is_nullifier_call_name(name, config) {
        kinds.push(TaintSinkKind::NullifierOrCommitment);
    }
    if is_enqueue_call_name(name, config) {
        kinds.push(TaintSinkKind::EnqueuePublicCall);
    }
    if lower == "debug_log" {
        kinds.push(TaintSinkKind::DebugLog);
        kinds.push(TaintSinkKind::LogEvent);
    }
    if lower.contains("oracle") {
        kinds.push(TaintSinkKind::OracleArgument);
    }
    if matches!(
        lower.as_str(),
        "hash" | "pedersen" | "serialize" | "to_bytes"
    ) || lower.ends_with("_hash")
        || lower.contains("serialize")
    {
        kinds.push(TaintSinkKind::HashOrSerialize);
    }
    if lower.contains("merkle") || lower.contains("witness") {
        kinds.push(TaintSinkKind::MerkleWitness);
    }

    kinds
}

fn push_call_sink(
    kind: TaintSinkKind,
    expr_id: &str,
    span: &Span,
    statement_block: &BTreeMap<String, String>,
    expr_to_statement: &BTreeMap<String, String>,
    source: Option<&SourceUnit>,
    out: &mut FunctionGraph,
) {
    let block_id = expr_to_statement
        .get(expr_id)
        .and_then(|statement_id| statement_block.get(statement_id))
        .cloned();
    out.sinks.push(SinkSite {
        kind,
        identifiers: BTreeSet::from([expr_id.to_string()]),
        span: span.clone(),
        line: source
            .and_then(|unit| span_text(unit, span))
            .unwrap_or("")
            .to_string(),
        block_id,
    });
}

fn push_sink_for_site(
    kind: TaintSinkKind,
    span: &Span,
    context: &SinkContext<'_>,
    out: &mut FunctionGraph,
) {
    let identifiers = sink_nodes_for_span(context.function_symbol_id, span, context.semantic);
    if identifiers.is_empty() {
        return;
    }
    let block_id = identifiers.iter().find_map(|node_id| {
        if node_id.starts_with("stmt::") {
            return context.statement_block.get(node_id).cloned();
        }
        context
            .expr_to_statement
            .get(node_id)
            .and_then(|statement_id| context.statement_block.get(statement_id))
            .cloned()
    });
    out.sinks.push(SinkSite {
        kind,
        identifiers,
        span: span.clone(),
        line: context
            .source
            .and_then(|unit| span_text(unit, span))
            .unwrap_or("")
            .to_string(),
        block_id,
    });
}

struct SinkContext<'a> {
    function_symbol_id: &'a str,
    semantic: &'a SemanticModel,
    statement_block: &'a BTreeMap<String, String>,
    expr_to_statement: &'a BTreeMap<String, String>,
    source: Option<&'a SourceUnit>,
}

fn sink_nodes_for_span(
    function_symbol_id: &str,
    span: &Span,
    semantic: &SemanticModel,
) -> BTreeSet<String> {
    let mut nodes = BTreeSet::<String>::new();

    for call_site in semantic.call_sites.iter().filter(|call_site| {
        call_site.function_symbol_id == function_symbol_id && spans_overlap(span, &call_site.span)
    }) {
        nodes.insert(call_site.expr_id.clone());
    }
    for expression in semantic.expressions.iter().filter(|expression| {
        expression.function_symbol_id == function_symbol_id
            && expression.category == ExpressionCategory::Call
            && spans_overlap(span, &expression.span)
    }) {
        nodes.insert(expression.expr_id.clone());
    }
    for statement in semantic.statements.iter().filter(|statement| {
        statement.function_symbol_id == function_symbol_id
            && statement.category == StatementCategory::Return
            && spans_overlap(span, &statement.span)
    }) {
        nodes.insert(statement.stmt_id.clone());
    }

    if nodes.is_empty()
        && let Some(statement) = innermost_statement_for_span(function_symbol_id, span, semantic)
    {
        nodes.insert(statement.stmt_id.clone());
    }

    nodes
}

fn expression_statement_map(
    function_symbol_id: &str,
    semantic: &SemanticModel,
) -> BTreeMap<String, String> {
    semantic
        .dfg_edges
        .iter()
        .filter(|edge| edge.function_symbol_id == function_symbol_id)
        .filter(|edge| {
            edge.from_node_id.starts_with("expr::") && edge.to_node_id.starts_with("stmt::")
        })
        .fold(BTreeMap::<String, String>::new(), |mut map, edge| {
            map.entry(edge.from_node_id.clone())
                .or_insert_with(|| edge.to_node_id.clone());
            map
        })
}

fn reverse_adjacency(
    function_symbol_id: &str,
    semantic: &SemanticModel,
) -> BTreeMap<String, BTreeSet<String>> {
    semantic
        .dfg_edges
        .iter()
        .filter(|edge| edge.function_symbol_id == function_symbol_id)
        .fold(
            BTreeMap::<String, BTreeSet<String>>::new(),
            |mut map, edge| {
                map.entry(edge.to_node_id.clone())
                    .or_default()
                    .insert(edge.from_node_id.clone());
                map
            },
        )
}

fn reverse_reachable_nodes(
    start_node: &str,
    reverse_adjacency: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut visited = BTreeSet::<String>::new();
    let mut queue = vec![start_node.to_string()];
    while let Some(node_id) = queue.pop() {
        if !visited.insert(node_id.clone()) {
            continue;
        }
        if let Some(parents) = reverse_adjacency.get(&node_id) {
            queue.extend(parents.iter().cloned());
        }
    }
    visited
}

fn spans_overlap(left: &Span, right: &Span) -> bool {
    normalize_file_path(&left.file) == normalize_file_path(&right.file)
        && left.start < right.end
        && right.start < left.end
}

fn innermost_statement_for_span<'a>(
    function_symbol_id: &str,
    span: &Span,
    semantic: &'a SemanticModel,
) -> Option<&'a aztec_lint_core::model::SemanticStatement> {
    semantic
        .statements
        .iter()
        .filter(|statement| statement.function_symbol_id == function_symbol_id)
        .filter(|statement| spans_overlap(&statement.span, span))
        .min_by_key(|statement| statement.span.end.saturating_sub(statement.span.start))
}

fn span_text<'a>(source: &'a SourceUnit, span: &Span) -> Option<&'a str> {
    let start = usize::try_from(span.start).ok()?;
    let end = usize::try_from(span.end).ok()?;
    if start >= end || end > source.text.len() {
        return None;
    }
    source.text.get(start..end)
}

fn build_source_functions(
    source: &SourceUnit,
    entrypoint_kinds: &BTreeMap<String, Vec<EntrypointKind>>,
    unconstrained_functions: &BTreeSet<String>,
    config: &AztecConfig,
) -> Vec<FunctionGraph> {
    let mut functions = Vec::<FunctionGraph>::new();

    let normalized_file = normalize_file_path(&source.path);
    let mut current_contract: Option<(String, usize)> = None;
    let mut current_function: Option<FunctionBuilder> = None;
    let mut brace_depth = 0usize;
    let mut offset = 0usize;

    for line in source.text.lines() {
        let trimmed = line.trim();
        let (_, code_after_attributes) = split_inline_attributes(trimmed);

        if current_contract.is_none()
            && let Some(contract_name) = is_contract_start(code_after_attributes)
        {
            let contract_id = format!("{normalized_file}::{contract_name}");
            current_contract = Some((contract_id, brace_depth + line.matches('{').count()));
        }

        if let Some((contract_id, _)) = current_contract.clone()
            && let Some(function_name) = is_function_start(code_after_attributes)
        {
            let function_symbol_id = format!("{contract_id}::fn::{function_name}");
            let kinds = entrypoint_kinds
                .get(&function_symbol_id)
                .cloned()
                .unwrap_or_default();
            let is_private_entrypoint = kinds.contains(&EntrypointKind::Private);
            let is_public_entrypoint = kinds.contains(&EntrypointKind::Public);
            let span = line_span(source, offset, line.len());
            let mut builder = FunctionBuilder {
                function: FunctionGraph {
                    contract_id: contract_id.clone(),
                    function_symbol_id: function_symbol_id.clone(),
                    is_private_entrypoint,
                    is_public_entrypoint,
                    semantic_function_symbol_id: None,
                    lines: vec![LineRecord {
                        text: normalize_line(line).to_string(),
                        span: span.clone(),
                    }],
                    definitions: Vec::new(),
                    sources: Vec::new(),
                    guards: Vec::new(),
                    sinks: Vec::new(),
                    dominators: BTreeMap::new(),
                },
                end_depth: brace_depth + line.matches('{').count(),
            };

            if is_private_entrypoint {
                for param in parse_params(code_after_attributes) {
                    builder.function.sources.push(TaintSource {
                        variable: param,
                        kind: TaintSourceKind::PrivateEntrypointParam,
                        span: span.clone(),
                    });
                }
            }

            current_function = Some(builder);
        }

        if let Some(builder) = current_function.as_mut() {
            analyze_line(
                source,
                line,
                offset,
                builder,
                unconstrained_functions,
                config,
            );
        }

        brace_depth = update_depth(brace_depth, line);

        if let Some(builder) = current_function.take() {
            if brace_depth < builder.end_depth {
                functions.push(builder.function);
            } else {
                current_function = Some(builder);
            }
        }

        if let Some((_, contract_depth)) = current_contract.clone()
            && brace_depth < contract_depth
        {
            current_contract = None;
        }

        offset += line.len() + 1;
    }

    functions
}

struct FunctionBuilder {
    function: FunctionGraph,
    end_depth: usize,
}

fn analyze_line(
    source: &SourceUnit,
    line: &str,
    offset: usize,
    builder: &mut FunctionBuilder,
    unconstrained_functions: &BTreeSet<String>,
    config: &AztecConfig,
) {
    let normalized = normalize_line(line);
    if normalized.is_empty() {
        return;
    }

    let span = line_span(source, offset, line.len());
    builder.function.lines.push(LineRecord {
        text: normalized.to_string(),
        span: span.clone(),
    });

    if let Some((name, rhs, name_start)) = parse_let_binding(normalized) {
        let dependencies = extract_identifiers(rhs)
            .into_iter()
            .filter(|identifier| identifier != &name)
            .collect::<BTreeSet<_>>();
        builder.function.definitions.push(Definition {
            variable: name.clone(),
            dependencies,
            span: Span::new(
                span.file.clone(),
                span.start + u32::try_from(name_start).unwrap_or(0),
                span.start + u32::try_from(name_start + name.len()).unwrap_or(0),
                span.line,
                span.col + u32::try_from(name_start).unwrap_or(0),
            ),
        });

        if contains_note_read(normalized, config) {
            builder.function.sources.push(TaintSource {
                variable: name.clone(),
                kind: TaintSourceKind::NoteRead,
                span: span.clone(),
            });
        }
        if rhs.contains("self.storage") || rhs.contains("secret") {
            builder.function.sources.push(TaintSource {
                variable: name.clone(),
                kind: TaintSourceKind::SecretState,
                span: span.clone(),
            });
        }
        if unconstrained_functions
            .iter()
            .any(|function| rhs.contains(&format!("{function}(")))
        {
            builder.function.sources.push(TaintSource {
                variable: name,
                kind: TaintSourceKind::UnconstrainedCall,
                span: span.clone(),
            });
        }
    }

    if is_guard_line(normalized) {
        for variable in extract_identifiers(normalized) {
            builder.function.guards.push(GuardSite {
                variable,
                span: span.clone(),
                block_id: None,
                covered_nodes: BTreeSet::new(),
            });
        }
    }

    for sink_kind in detect_sink_kinds(normalized) {
        builder.function.sinks.push(SinkSite {
            kind: sink_kind,
            identifiers: sink_identifiers(sink_kind, normalized),
            span: span.clone(),
            line: normalized.to_string(),
            block_id: None,
        });
    }
}

fn collect_unconstrained_functions(sources: &[SourceUnit]) -> BTreeSet<String> {
    let mut names = BTreeSet::<String>::new();

    for source in sources {
        for line in source.text.lines() {
            let normalized = normalize_line(line);
            if let Some(index) = normalized.find("unconstrained fn ") {
                let tail = &normalized[index + "unconstrained fn ".len()..];
                if let Some(name) = extract_first_identifier(tail) {
                    names.insert(name);
                }
            }
        }
    }

    names
}

fn parse_let_binding(line: &str) -> Option<(String, &str, usize)> {
    let bytes = line.as_bytes();
    let marker = line.find("let ")?;
    if marker > 0 && is_ident_continue(bytes[marker - 1]) {
        return None;
    }

    let mut cursor = marker + "let ".len();
    if line[cursor..].starts_with("mut ") {
        cursor += "mut ".len();
    }

    let name_start = cursor;
    while cursor < bytes.len() && is_ident_continue(bytes[cursor]) {
        cursor += 1;
    }
    if name_start == cursor {
        return None;
    }
    let name = line[name_start..cursor].to_string();
    let rhs_start = line[cursor..].find('=')? + cursor + 1;

    Some((name, line[rhs_start..].trim(), name_start))
}

fn parse_params(line: &str) -> Vec<String> {
    let Some(open) = line.find('(') else {
        return Vec::new();
    };
    let Some(close) = line[open + 1..].find(')') else {
        return Vec::new();
    };
    let body = &line[open + 1..open + 1 + close];
    body.split(',')
        .filter_map(|segment| {
            let name = segment
                .split(':')
                .next()?
                .trim()
                .trim_start_matches("mut ")
                .trim();
            if name.is_empty() || name == "self" {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
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

fn detect_sink_kinds(line: &str) -> Vec<TaintSinkKind> {
    let mut sinks = Vec::<TaintSinkKind>::new();

    let trimmed = line.trim_start();
    if trimmed.starts_with("if ")
        || trimmed.starts_with("if(")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("match ")
    {
        sinks.push(TaintSinkKind::BranchCondition);
    }
    if line.contains("emit(") || line.contains("return ") {
        sinks.push(TaintSinkKind::PublicOutput);
    }
    if line.contains("debug_log(") {
        sinks.push(TaintSinkKind::DebugLog);
        sinks.push(TaintSinkKind::LogEvent);
    }
    if line.contains("self.storage")
        && (line.contains("=") || line.contains(".write(") || line.contains(".set("))
    {
        sinks.push(TaintSinkKind::PublicStorageWrite);
    }
    if line.contains("enqueue(") || line.contains("enqueue_self") {
        sinks.push(TaintSinkKind::EnqueuePublicCall);
    }
    if line.contains("oracle") && line.contains('(') {
        sinks.push(TaintSinkKind::OracleArgument);
    }
    if line.contains("emit_nullifier(") || line.contains("nullify(") || line.contains(".insert(") {
        sinks.push(TaintSinkKind::NullifierOrCommitment);
    }
    if line.contains("hash(")
        || line.contains("pedersen(")
        || line.contains("serialize(")
        || line.contains("to_bytes(")
    {
        sinks.push(TaintSinkKind::HashOrSerialize);
    }
    if line.contains("merkle") || line.contains("witness") {
        sinks.push(TaintSinkKind::MerkleWitness);
    }

    sinks
}

fn sink_identifiers(kind: TaintSinkKind, line: &str) -> BTreeSet<String> {
    match kind {
        TaintSinkKind::HashOrSerialize => {
            let segment = line
                .split_once('=')
                .map_or(line, |(_, rhs)| rhs)
                .trim_start();
            extract_identifiers(segment)
        }
        TaintSinkKind::BranchCondition => {
            let segment = line
                .split_once('{')
                .map_or(line, |(head, _)| head)
                .trim_end();
            extract_identifiers(segment)
        }
        _ => extract_identifiers(line),
    }
}

fn is_guard_line(line: &str) -> bool {
    (line.contains("assert(") || line.contains("constrain(") || line.contains("range"))
        && (line.contains('<') || line.contains("<="))
}

fn extract_identifiers(line: &str) -> BTreeSet<String> {
    let bytes = line.as_bytes();
    let mut cursor = 0usize;
    let mut identifiers = BTreeSet::<String>::new();

    while cursor < bytes.len() {
        if !is_ident_start(bytes[cursor]) {
            cursor += 1;
            continue;
        }
        let start = cursor;
        cursor += 1;
        while cursor < bytes.len() && is_ident_continue(bytes[cursor]) {
            cursor += 1;
        }
        let candidate = line[start..cursor].to_string();
        if !is_keyword(&candidate) {
            identifiers.insert(candidate);
        }
    }

    identifiers
}

fn extract_first_identifier(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if !is_ident_start(bytes[cursor]) {
            cursor += 1;
            continue;
        }
        let start = cursor;
        cursor += 1;
        while cursor < bytes.len() && is_ident_continue(bytes[cursor]) {
            cursor += 1;
        }
        let candidate = &line[start..cursor];
        if !is_keyword(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_keyword(token: &str) -> bool {
    matches!(
        token,
        "let"
            | "mut"
            | "fn"
            | "if"
            | "while"
            | "match"
            | "return"
            | "assert"
            | "constrain"
            | "self"
            | "true"
            | "false"
            | "pub"
            | "contract"
    )
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::config::AztecConfig;
    use aztec_lint_core::model::{
        AztecModel, CallSite, CfgBlock, CfgEdge, CfgEdgeKind, DfgEdge, DfgEdgeKind, Entrypoint,
        EntrypointKind, ExpressionCategory, GuardKind, GuardNode, SemanticExpression,
        SemanticFunction, SemanticModel, SemanticSite, SemanticStatement, Span, StatementCategory,
        TypeCategory,
    };

    use crate::detect::SourceUnit;
    use crate::model_builder::build_aztec_model;

    use super::{
        TaintSinkKind, TaintSourceKind, build_def_use_graph, build_def_use_graph_with_semantic,
    };

    fn span_for(source: &str, needle: &str) -> Span {
        let start = source.find(needle).expect("needle must exist in source");
        let end = start + needle.len();
        Span::new(
            "src/main.nr".to_string(),
            u32::try_from(start).expect("span start fits in u32"),
            u32::try_from(end).expect("span end fits in u32"),
            1,
            1,
        )
    }

    #[test]
    fn captures_private_params_and_note_read_sources() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        let notes = self.notes.get_notes();
        let combined = secret + notes;
        emit(combined);
    }
}
"#;
        let sources = vec![SourceUnit::new("src/main.nr", source)];
        let model = build_aztec_model(&sources, &AztecConfig::default());
        let graph = build_def_use_graph(&sources, &model, &AztecConfig::default());

        let function = &graph.functions[0];
        assert!(
            function
                .sources
                .iter()
                .any(|item| item.kind == TaintSourceKind::PrivateEntrypointParam)
        );
        assert!(
            function
                .sources
                .iter()
                .any(|item| item.kind == TaintSourceKind::NoteRead)
        );
        assert!(
            function
                .sinks
                .iter()
                .any(|item| item.kind == TaintSinkKind::PublicOutput)
        );
    }

    #[test]
    fn parses_mut_private_params_as_sources() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(mut secret: Field) {
        emit(secret);
    }
}
"#;
        let sources = vec![SourceUnit::new("src/main.nr", source)];
        let model = build_aztec_model(&sources, &AztecConfig::default());
        let graph = build_def_use_graph(&sources, &model, &AztecConfig::default());

        let function = &graph.functions[0];
        assert!(function.sources.iter().any(|item| {
            item.kind == TaintSourceKind::PrivateEntrypointParam && item.variable == "secret"
        }));
    }

    #[test]
    fn semantic_graph_builds_typed_sources_and_sinks() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        assert(secret < 100);
        let notes = self.notes.get_notes();
        let digest = hash(notes);
        emit(digest);
    }
}
"#;
        let sources = vec![SourceUnit::new("src/main.nr", source)];
        let call_notes_span = span_for(source, "self.notes.get_notes()");
        let call_hash_span = span_for(source, "hash(notes)");
        let call_emit_span = span_for(source, "emit(digest)");
        let guard_span = span_for(source, "assert(secret < 100)");
        let function_span = Span::new(
            "src/main.nr".to_string(),
            0,
            u32::try_from(source.len()).expect("source length fits in u32"),
            1,
            1,
        );
        let entry_span = function_span.clone();

        let model = AztecModel {
            entrypoints: vec![Entrypoint {
                contract_id: "src/main.nr::C".to_string(),
                function_symbol_id: "src/main.nr::C::fn::bridge".to_string(),
                kind: EntrypointKind::Private,
                span: entry_span.clone(),
            }],
            note_read_sites: vec![SemanticSite {
                contract_id: "src/main.nr::C".to_string(),
                function_symbol_id: "src/main.nr::C::fn::bridge".to_string(),
                span: call_notes_span.clone(),
            }],
            public_sinks: vec![SemanticSite {
                contract_id: "src/main.nr::C".to_string(),
                function_symbol_id: "src/main.nr::C::fn::bridge".to_string(),
                span: call_emit_span.clone(),
            }],
            ..AztecModel::default()
        };
        let semantic = SemanticModel {
            functions: vec![
                SemanticFunction {
                    symbol_id: "fn::bridge".to_string(),
                    name: "bridge".to_string(),
                    module_symbol_id: "module::root".to_string(),
                    return_type_repr: "()".to_string(),
                    return_type_category: TypeCategory::Unknown,
                    parameter_types: vec!["Field".to_string()],
                    is_entrypoint: true,
                    is_unconstrained: false,
                    span: function_span.clone(),
                },
                SemanticFunction {
                    symbol_id: "fn::get_notes".to_string(),
                    name: "get_notes".to_string(),
                    module_symbol_id: "module::root".to_string(),
                    return_type_repr: "Field".to_string(),
                    return_type_category: TypeCategory::Field,
                    parameter_types: vec![],
                    is_entrypoint: false,
                    is_unconstrained: false,
                    span: call_notes_span.clone(),
                },
                SemanticFunction {
                    symbol_id: "fn::hash".to_string(),
                    name: "hash".to_string(),
                    module_symbol_id: "module::root".to_string(),
                    return_type_repr: "Field".to_string(),
                    return_type_category: TypeCategory::Field,
                    parameter_types: vec!["Field".to_string()],
                    is_entrypoint: false,
                    is_unconstrained: false,
                    span: call_hash_span.clone(),
                },
                SemanticFunction {
                    symbol_id: "fn::emit".to_string(),
                    name: "emit".to_string(),
                    module_symbol_id: "module::root".to_string(),
                    return_type_repr: "()".to_string(),
                    return_type_category: TypeCategory::Unknown,
                    parameter_types: vec!["Field".to_string()],
                    is_entrypoint: false,
                    is_unconstrained: false,
                    span: call_emit_span.clone(),
                },
            ],
            expressions: vec![
                SemanticExpression {
                    expr_id: "expr::notes_call".to_string(),
                    function_symbol_id: "fn::bridge".to_string(),
                    category: ExpressionCategory::Call,
                    type_category: TypeCategory::Field,
                    type_repr: "Field".to_string(),
                    span: call_notes_span.clone(),
                },
                SemanticExpression {
                    expr_id: "expr::hash_call".to_string(),
                    function_symbol_id: "fn::bridge".to_string(),
                    category: ExpressionCategory::Call,
                    type_category: TypeCategory::Field,
                    type_repr: "Field".to_string(),
                    span: call_hash_span.clone(),
                },
                SemanticExpression {
                    expr_id: "expr::emit_call".to_string(),
                    function_symbol_id: "fn::bridge".to_string(),
                    category: ExpressionCategory::Call,
                    type_category: TypeCategory::Unknown,
                    type_repr: "()".to_string(),
                    span: call_emit_span.clone(),
                },
                SemanticExpression {
                    expr_id: "expr::guard_secret".to_string(),
                    function_symbol_id: "fn::bridge".to_string(),
                    category: ExpressionCategory::Identifier,
                    type_category: TypeCategory::Field,
                    type_repr: "Field".to_string(),
                    span: guard_span.clone(),
                },
            ],
            statements: vec![
                SemanticStatement {
                    stmt_id: "stmt::guard".to_string(),
                    function_symbol_id: "fn::bridge".to_string(),
                    category: StatementCategory::Assert,
                    span: guard_span.clone(),
                },
                SemanticStatement {
                    stmt_id: "stmt::notes".to_string(),
                    function_symbol_id: "fn::bridge".to_string(),
                    category: StatementCategory::Let,
                    span: call_notes_span.clone(),
                },
                SemanticStatement {
                    stmt_id: "stmt::hash".to_string(),
                    function_symbol_id: "fn::bridge".to_string(),
                    category: StatementCategory::Let,
                    span: call_hash_span.clone(),
                },
                SemanticStatement {
                    stmt_id: "stmt::emit".to_string(),
                    function_symbol_id: "fn::bridge".to_string(),
                    category: StatementCategory::Expression,
                    span: call_emit_span.clone(),
                },
            ],
            cfg_blocks: vec![
                CfgBlock {
                    function_symbol_id: "fn::bridge".to_string(),
                    block_id: "bb::bridge::guard".to_string(),
                    statement_ids: vec!["stmt::guard".to_string()],
                },
                CfgBlock {
                    function_symbol_id: "fn::bridge".to_string(),
                    block_id: "bb::bridge::notes".to_string(),
                    statement_ids: vec!["stmt::notes".to_string()],
                },
                CfgBlock {
                    function_symbol_id: "fn::bridge".to_string(),
                    block_id: "bb::bridge::hash".to_string(),
                    statement_ids: vec!["stmt::hash".to_string()],
                },
                CfgBlock {
                    function_symbol_id: "fn::bridge".to_string(),
                    block_id: "bb::bridge::emit".to_string(),
                    statement_ids: vec!["stmt::emit".to_string()],
                },
            ],
            cfg_edges: vec![
                CfgEdge {
                    function_symbol_id: "fn::bridge".to_string(),
                    from_block_id: "bb::bridge::guard".to_string(),
                    to_block_id: "bb::bridge::notes".to_string(),
                    kind: CfgEdgeKind::Unconditional,
                },
                CfgEdge {
                    function_symbol_id: "fn::bridge".to_string(),
                    from_block_id: "bb::bridge::notes".to_string(),
                    to_block_id: "bb::bridge::hash".to_string(),
                    kind: CfgEdgeKind::Unconditional,
                },
                CfgEdge {
                    function_symbol_id: "fn::bridge".to_string(),
                    from_block_id: "bb::bridge::hash".to_string(),
                    to_block_id: "bb::bridge::emit".to_string(),
                    kind: CfgEdgeKind::Unconditional,
                },
            ],
            dfg_edges: vec![
                DfgEdge {
                    function_symbol_id: "fn::bridge".to_string(),
                    from_node_id: "expr::guard_secret".to_string(),
                    to_node_id: "stmt::guard".to_string(),
                    kind: DfgEdgeKind::DefUse,
                },
                DfgEdge {
                    function_symbol_id: "fn::bridge".to_string(),
                    from_node_id: "expr::notes_call".to_string(),
                    to_node_id: "stmt::notes".to_string(),
                    kind: DfgEdgeKind::DefUse,
                },
                DfgEdge {
                    function_symbol_id: "fn::bridge".to_string(),
                    from_node_id: "stmt::notes".to_string(),
                    to_node_id: "def::notes".to_string(),
                    kind: DfgEdgeKind::DefUse,
                },
                DfgEdge {
                    function_symbol_id: "fn::bridge".to_string(),
                    from_node_id: "def::notes".to_string(),
                    to_node_id: "expr::hash_call".to_string(),
                    kind: DfgEdgeKind::UseDef,
                },
                DfgEdge {
                    function_symbol_id: "fn::bridge".to_string(),
                    from_node_id: "expr::hash_call".to_string(),
                    to_node_id: "stmt::hash".to_string(),
                    kind: DfgEdgeKind::DefUse,
                },
                DfgEdge {
                    function_symbol_id: "fn::bridge".to_string(),
                    from_node_id: "stmt::hash".to_string(),
                    to_node_id: "def::digest".to_string(),
                    kind: DfgEdgeKind::DefUse,
                },
                DfgEdge {
                    function_symbol_id: "fn::bridge".to_string(),
                    from_node_id: "def::digest".to_string(),
                    to_node_id: "expr::emit_call".to_string(),
                    kind: DfgEdgeKind::UseDef,
                },
                DfgEdge {
                    function_symbol_id: "fn::bridge".to_string(),
                    from_node_id: "expr::emit_call".to_string(),
                    to_node_id: "stmt::emit".to_string(),
                    kind: DfgEdgeKind::DefUse,
                },
            ],
            call_sites: vec![
                CallSite {
                    call_site_id: "call::notes".to_string(),
                    function_symbol_id: "fn::bridge".to_string(),
                    callee_symbol_id: "fn::get_notes".to_string(),
                    expr_id: "expr::notes_call".to_string(),
                    span: call_notes_span.clone(),
                },
                CallSite {
                    call_site_id: "call::hash".to_string(),
                    function_symbol_id: "fn::bridge".to_string(),
                    callee_symbol_id: "fn::hash".to_string(),
                    expr_id: "expr::hash_call".to_string(),
                    span: call_hash_span.clone(),
                },
                CallSite {
                    call_site_id: "call::emit".to_string(),
                    function_symbol_id: "fn::bridge".to_string(),
                    callee_symbol_id: "fn::emit".to_string(),
                    expr_id: "expr::emit_call".to_string(),
                    span: call_emit_span.clone(),
                },
            ],
            guard_nodes: vec![GuardNode {
                guard_id: "guard::secret".to_string(),
                function_symbol_id: "fn::bridge".to_string(),
                kind: GuardKind::Assert,
                guarded_expr_id: Some("expr::guard_secret".to_string()),
                span: guard_span.clone(),
            }],
        };

        let graph = build_def_use_graph_with_semantic(
            &sources,
            &model,
            Some(&semantic),
            &AztecConfig::default(),
        );
        let function = graph
            .functions
            .iter()
            .find(|function| function.function_symbol_id == "fn::bridge")
            .expect("semantic function must be present");

        assert!(
            function
                .sources
                .iter()
                .any(|source| source.kind == TaintSourceKind::NoteRead)
        );
        assert!(
            function
                .sinks
                .iter()
                .any(|sink| sink.kind == TaintSinkKind::HashOrSerialize)
        );
        assert!(
            function
                .guards
                .iter()
                .any(|guard| !guard.covered_nodes.is_empty())
        );
    }
}
