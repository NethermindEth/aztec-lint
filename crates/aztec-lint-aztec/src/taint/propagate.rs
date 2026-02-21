use std::collections::{BTreeMap, BTreeSet};

use super::graph::{DefUseGraph, TaintSinkKind, TaintSourceKind};
use crate::taint::graph::SinkSite;
use aztec_lint_core::model::Span;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaintFlow {
    pub function_symbol_id: String,
    pub variable: String,
    pub source_kind: TaintSourceKind,
    pub sink_kind: TaintSinkKind,
    pub sink_span: Span,
    pub sink_line: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TaintAnalysis {
    pub tainted_variables: BTreeMap<String, BTreeSet<String>>,
    pub flows: Vec<TaintFlow>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaintOptions {
    pub inter_procedural: bool,
    pub max_iterations: usize,
}

impl Default for TaintOptions {
    fn default() -> Self {
        Self {
            inter_procedural: false,
            max_iterations: 256,
        }
    }
}

pub fn analyze_intra_procedural(graph: &DefUseGraph) -> TaintAnalysis {
    analyze_with_options(graph, &TaintOptions::default())
}

pub fn analyze_with_options(graph: &DefUseGraph, options: &TaintOptions) -> TaintAnalysis {
    let mut analysis = TaintAnalysis::default();

    for function in &graph.functions {
        let adjacency = build_adjacency(function);
        let step_limit = options.max_iterations.max(function.definitions.len() + 1);
        let mut tainted_nodes = BTreeSet::<String>::new();

        if options.inter_procedural {
            // Reserved for future inter-procedural propagation. Intentionally not enabled in v1.
        }

        for source in &function.sources {
            let reachable = reachable_from_source(&source.variable, &adjacency, step_limit);
            tainted_nodes.extend(reachable.iter().cloned());

            for sink in &function.sinks {
                if reachable.is_disjoint(&sink.identifiers) {
                    continue;
                }
                if sink.kind == TaintSinkKind::HashOrSerialize
                    && is_source_guarded_before_sink(
                        &source.variable,
                        sink,
                        &function.guards,
                        &function.dominators,
                    )
                {
                    continue;
                }
                analysis.flows.push(TaintFlow {
                    function_symbol_id: function.function_symbol_id.clone(),
                    variable: source.variable.clone(),
                    source_kind: source.kind,
                    sink_kind: sink.kind,
                    sink_span: sink.span.clone(),
                    sink_line: sink.line.clone(),
                });
            }
        }

        analysis
            .tainted_variables
            .insert(function.function_symbol_id.clone(), tainted_nodes);
    }

    analysis.flows.sort_by_key(|flow| {
        (
            flow.function_symbol_id.clone(),
            flow.sink_span.file.clone(),
            flow.sink_span.start,
            format!("{:?}", flow.sink_kind),
            flow.variable.clone(),
        )
    });
    analysis.flows.dedup();

    analysis
}

fn build_adjacency(function: &super::graph::FunctionGraph) -> BTreeMap<String, BTreeSet<String>> {
    function.definitions.iter().fold(
        BTreeMap::<String, BTreeSet<String>>::new(),
        |mut map, def| {
            for dependency in &def.dependencies {
                map.entry(dependency.clone())
                    .or_default()
                    .insert(def.variable.clone());
            }
            map
        },
    )
}

fn reachable_from_source(
    source: &str,
    adjacency: &BTreeMap<String, BTreeSet<String>>,
    step_limit: usize,
) -> BTreeSet<String> {
    let mut visited = BTreeSet::<String>::new();
    let mut frontier = vec![source.to_string()];
    let mut steps = 0usize;

    while let Some(node_id) = frontier.pop() {
        if !visited.insert(node_id.clone()) {
            continue;
        }
        if steps >= step_limit {
            continue;
        }
        steps += 1;

        if let Some(next_nodes) = adjacency.get(&node_id) {
            frontier.extend(next_nodes.iter().cloned());
        }
    }

    visited
}

fn is_source_guarded_before_sink(
    source_variable: &str,
    sink: &SinkSite,
    guards: &[super::graph::GuardSite],
    dominators: &BTreeMap<String, BTreeSet<String>>,
) -> bool {
    guards.iter().any(|guard| {
        let source_is_guarded = if guard.covered_nodes.is_empty() {
            guard.variable == source_variable
        } else {
            guard.covered_nodes.contains(source_variable)
        };
        if !source_is_guarded {
            return false;
        }
        guard_dominates_sink(guard, sink, dominators)
    })
}

fn guard_dominates_sink(
    guard: &super::graph::GuardSite,
    sink: &SinkSite,
    dominators: &BTreeMap<String, BTreeSet<String>>,
) -> bool {
    let Some(sink_block_id) = sink.block_id.as_deref() else {
        return guard.span.start < sink.span.start;
    };
    let Some(guard_block_id) = guard.block_id.as_deref() else {
        return guard.span.start < sink.span.start;
    };
    if sink_block_id == guard_block_id {
        return guard.span.start < sink.span.start;
    }
    dominators
        .get(sink_block_id)
        .is_some_and(|set| set.contains(guard_block_id))
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use aztec_lint_core::config::AztecConfig;
    use aztec_lint_core::model::Span;

    use crate::detect::SourceUnit;
    use crate::model_builder::build_aztec_model;
    use crate::taint::graph::{
        DefUseGraph, Definition, FunctionGraph, GuardSite, SinkSite, TaintSinkKind, TaintSource,
        TaintSourceKind, build_def_use_graph,
    };

    use super::analyze_intra_procedural;

    #[test]
    fn source_only_without_sink_has_no_flows() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge() {
        let notes = self.notes.get_notes();
    }
}
"#;
        let sources = vec![SourceUnit::new("src/main.nr", source)];
        let model = build_aztec_model(&sources, &AztecConfig::default());
        let graph = build_def_use_graph(&sources, &model, &AztecConfig::default());
        let analysis = analyze_intra_procedural(&graph);

        assert!(analysis.flows.is_empty());
    }

    #[test]
    fn source_to_sink_produces_flow() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge() {
        let notes = self.notes.get_notes();
        emit(notes);
    }
}
"#;
        let sources = vec![SourceUnit::new("src/main.nr", source)];
        let model = build_aztec_model(&sources, &AztecConfig::default());
        let graph = build_def_use_graph(&sources, &model, &AztecConfig::default());
        let analysis = analyze_intra_procedural(&graph);

        assert!(
            analysis
                .flows
                .iter()
                .any(|flow| flow.source_kind == TaintSourceKind::NoteRead
                    && flow.sink_kind == TaintSinkKind::PublicOutput)
        );
    }

    #[test]
    fn guard_sanitizes_hash_sink_flow() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        assert(secret < 100);
        let digest = hash(secret);
        emit(digest);
    }
}
"#;
        let sources = vec![SourceUnit::new("src/main.nr", source)];
        let model = build_aztec_model(&sources, &AztecConfig::default());
        let graph = build_def_use_graph(&sources, &model, &AztecConfig::default());
        let analysis = analyze_intra_procedural(&graph);

        assert!(
            !analysis
                .flows
                .iter()
                .any(|flow| flow.sink_kind == TaintSinkKind::HashOrSerialize)
        );
    }

    #[test]
    fn guard_does_not_sanitize_public_output_flow() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        assert(secret < 100);
        emit(secret);
    }
}
"#;
        let sources = vec![SourceUnit::new("src/main.nr", source)];
        let model = build_aztec_model(&sources, &AztecConfig::default());
        let graph = build_def_use_graph(&sources, &model, &AztecConfig::default());
        let analysis = analyze_intra_procedural(&graph);

        assert!(
            analysis
                .flows
                .iter()
                .any(|flow| flow.sink_kind == TaintSinkKind::PublicOutput)
        );
    }

    #[test]
    fn performance_smoke_stays_bounded() {
        let mut body = String::from(
            "#[aztec]\npub contract C {\n#[external(\"private\")]\nfn bridge(secret: Field) {\n",
        );
        for idx in 0..200 {
            if idx == 0 {
                body.push_str("let v0 = secret;\n");
            } else {
                body.push_str(&format!("let v{idx} = v{};\n", idx - 1));
            }
        }
        body.push_str("emit(v199);\n}\n}\n");

        let sources = vec![SourceUnit::new("src/main.nr", body)];
        let model = build_aztec_model(&sources, &AztecConfig::default());
        let graph = build_def_use_graph(&sources, &model, &AztecConfig::default());

        let started = Instant::now();
        let analysis = analyze_intra_procedural(&graph);
        assert!(started.elapsed().as_millis() < 1000);
        assert!(!analysis.flows.is_empty());
    }

    #[test]
    fn cfg_dominance_sanitizes_hash_even_with_later_span() {
        let graph = DefUseGraph {
            functions: vec![FunctionGraph {
                contract_id: "src/main.nr::C".to_string(),
                function_symbol_id: "fn::bridge".to_string(),
                is_private_entrypoint: true,
                is_public_entrypoint: false,
                semantic_function_symbol_id: Some("fn::bridge".to_string()),
                lines: vec![],
                definitions: vec![Definition {
                    variable: "expr::hash".to_string(),
                    dependencies: std::collections::BTreeSet::from(["def::secret".to_string()]),
                    span: Span::new("src/main.nr".to_string(), 100, 110, 1, 1),
                }],
                sources: vec![TaintSource {
                    variable: "def::secret".to_string(),
                    kind: TaintSourceKind::PrivateEntrypointParam,
                    span: Span::new("src/main.nr".to_string(), 10, 20, 1, 1),
                }],
                guards: vec![GuardSite {
                    variable: "expr::guard_secret".to_string(),
                    span: Span::new("src/main.nr".to_string(), 300, 310, 1, 1),
                    block_id: Some("bb::guard".to_string()),
                    covered_nodes: std::collections::BTreeSet::from(["def::secret".to_string()]),
                }],
                sinks: vec![SinkSite {
                    kind: TaintSinkKind::HashOrSerialize,
                    identifiers: std::collections::BTreeSet::from(["expr::hash".to_string()]),
                    span: Span::new("src/main.nr".to_string(), 100, 110, 1, 1),
                    line: "hash(secret)".to_string(),
                    block_id: Some("bb::hash".to_string()),
                }],
                dominators: std::collections::BTreeMap::from([(
                    "bb::hash".to_string(),
                    std::collections::BTreeSet::from([
                        "bb::guard".to_string(),
                        "bb::hash".to_string(),
                    ]),
                )]),
            }],
        };

        let analysis = analyze_intra_procedural(&graph);
        assert!(analysis.flows.is_empty());
    }
}
