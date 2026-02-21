use std::collections::{BTreeMap, BTreeSet};

use aztec_lint_aztec::SourceUnit;
use aztec_lint_aztec::taint::{
    DefUseGraph, FunctionGraph, TaintSinkKind, analyze_intra_procedural,
    build_def_use_graph_with_semantic,
};
use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::model::{SemanticModel, StatementCategory};
use aztec_lint_core::policy::SOUNDNESS;

use crate::Rule;
use crate::engine::context::RuleContext;

pub struct Aztec022MerkleWitnessRule;

impl Rule for Aztec022MerkleWitnessRule {
    fn id(&self) -> &'static str {
        "AZTEC022"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(model) = ctx.aztec_model() else {
            return;
        };

        let config = ctx.aztec_config();
        let sources = ctx
            .files()
            .iter()
            .map(|file| SourceUnit::new(file.path().to_string(), file.text().to_string()))
            .collect::<Vec<_>>();
        let graph =
            build_def_use_graph_with_semantic(&sources, model, Some(ctx.semantic_model()), &config);
        let analysis = analyze_intra_procedural(&graph);

        let tainted_merkle_functions = analysis
            .flows
            .iter()
            .filter(|flow| flow.sink_kind == TaintSinkKind::MerkleWitness)
            .map(|flow| flow.function_symbol_id.clone())
            .collect::<BTreeSet<_>>();

        for function in &graph.functions {
            let has_merkle_sink = function
                .sinks
                .iter()
                .any(|sink| sink.kind == TaintSinkKind::MerkleWitness);
            if !has_merkle_sink {
                continue;
            }

            let has_verification = has_semantic_merkle_verification(function, &graph, ctx);
            if has_verification {
                continue;
            }

            if !tainted_merkle_functions.contains(&function.function_symbol_id)
                && !function.is_private_entrypoint
            {
                continue;
            }

            for sink in &function.sinks {
                if sink.kind != TaintSinkKind::MerkleWitness {
                    continue;
                }
                out.push(ctx.diagnostic(
                    self.id(),
                    SOUNDNESS,
                    "Merkle witness usage appears unchecked or weakly verified",
                    sink.span.clone(),
                ));
            }
        }
    }
}

fn has_semantic_merkle_verification(
    function: &FunctionGraph,
    graph: &DefUseGraph,
    ctx: &RuleContext<'_>,
) -> bool {
    let semantic = ctx.semantic_model();
    if semantic.call_sites.is_empty() || semantic.dfg_edges.is_empty() {
        return has_text_verification_fallback(function);
    }

    let function_symbol_id = &function.function_symbol_id;
    let callee_names = semantic
        .functions
        .iter()
        .map(|item| (item.symbol_id.as_str(), item.name.to_ascii_lowercase()))
        .collect::<BTreeMap<_, _>>();

    if semantic.call_sites.iter().any(|call_site| {
        call_site.function_symbol_id == *function_symbol_id
            && callee_names
                .get(call_site.callee_symbol_id.as_str())
                .is_some_and(|name| name == "verify_merkle" || name == "check_membership")
    }) {
        return true;
    }

    let statement_assert_targets = semantic
        .statements
        .iter()
        .filter(|statement| statement.function_symbol_id == *function_symbol_id)
        .filter(|statement| {
            matches!(
                statement.category,
                StatementCategory::Assert | StatementCategory::Constrain
            )
        })
        .map(|statement| statement.stmt_id.clone())
        .collect::<BTreeSet<_>>();
    let guard_targets = semantic
        .guard_nodes
        .iter()
        .filter(|guard| guard.function_symbol_id == *function_symbol_id)
        .filter_map(|guard| guard.guarded_expr_id.clone())
        .collect::<BTreeSet<_>>();
    let mut verification_targets = statement_assert_targets;
    verification_targets.extend(guard_targets);
    if verification_targets.is_empty() {
        return false;
    }

    let adjacency = dfg_adjacency(function_symbol_id, semantic);
    let mut merkle_seed_nodes = function
        .sinks
        .iter()
        .filter(|sink| sink.kind == TaintSinkKind::MerkleWitness)
        .flat_map(|sink| sink.identifiers.iter().cloned())
        .collect::<BTreeSet<_>>();
    if merkle_seed_nodes.is_empty() {
        merkle_seed_nodes.extend(
            graph
                .functions
                .iter()
                .filter(|item| item.function_symbol_id == *function_symbol_id)
                .flat_map(|item| {
                    item.sinks
                        .iter()
                        .filter(|sink| sink.kind == TaintSinkKind::MerkleWitness)
                        .flat_map(|sink| sink.identifiers.iter().cloned())
                        .collect::<Vec<_>>()
                }),
        );
    }
    if merkle_seed_nodes.is_empty() {
        return false;
    }

    let reachable = reachable_nodes(&merkle_seed_nodes, &adjacency);
    !reachable.is_disjoint(&verification_targets)
}

fn dfg_adjacency(
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
                map.entry(edge.from_node_id.clone())
                    .or_default()
                    .insert(edge.to_node_id.clone());
                map
            },
        )
}

fn reachable_nodes(
    seeds: &BTreeSet<String>,
    adjacency: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut visited = BTreeSet::<String>::new();
    let mut queue = seeds.iter().cloned().collect::<Vec<_>>();
    while let Some(node_id) = queue.pop() {
        if !visited.insert(node_id.clone()) {
            continue;
        }
        if let Some(next_nodes) = adjacency.get(&node_id) {
            queue.extend(next_nodes.iter().cloned());
        }
    }
    visited
}

fn has_text_verification_fallback(function: &FunctionGraph) -> bool {
    function.lines.iter().any(|line| {
        line.text.contains("verify_merkle")
            || line.text.contains("check_membership")
            || (line.text.contains("assert(")
                && (line.text.contains("root")
                    || line.text.contains("path")
                    || line.text.contains("witness")))
    })
}
