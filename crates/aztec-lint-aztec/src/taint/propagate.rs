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
        let mut tainted = function
            .sources
            .iter()
            .map(|source| (source.variable.clone(), source.kind))
            .collect::<BTreeMap<_, _>>();
        let mut changed = true;
        let mut steps = 0usize;

        let step_limit = options.max_iterations.max(function.definitions.len() + 1);
        while changed && steps < step_limit {
            changed = false;
            steps += 1;

            for definition in &function.definitions {
                if tainted.contains_key(&definition.variable) {
                    continue;
                }
                let inherited = definition
                    .dependencies
                    .iter()
                    .find_map(|dependency| tainted.get(dependency).copied());
                if let Some(source_kind) = inherited {
                    tainted.insert(definition.variable.clone(), source_kind);
                    changed = true;
                }
            }
        }

        if options.inter_procedural {
            // Reserved for future inter-procedural propagation. Intentionally not enabled in v1.
        }

        analysis.tainted_variables.insert(
            function.function_symbol_id.clone(),
            tainted.keys().cloned().collect(),
        );

        let guard_offsets =
            function
                .guards
                .iter()
                .fold(BTreeMap::<String, u32>::new(), |mut map, guard| {
                    map.entry(guard.variable.clone())
                        .and_modify(|existing| *existing = (*existing).min(guard.span.start))
                        .or_insert(guard.span.start);
                    map
                });

        for sink in &function.sinks {
            record_sink_flows(
                &mut analysis,
                &function.function_symbol_id,
                &tainted,
                &guard_offsets,
                sink,
            );
        }
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

fn record_sink_flows(
    analysis: &mut TaintAnalysis,
    function_symbol_id: &str,
    tainted: &BTreeMap<String, TaintSourceKind>,
    guard_offsets: &BTreeMap<String, u32>,
    sink: &SinkSite,
) {
    for identifier in &sink.identifiers {
        let Some(source_kind) = tainted.get(identifier).copied() else {
            continue;
        };
        if sink.kind == TaintSinkKind::HashOrSerialize
            && is_guarded_before_sink(identifier, sink.sink_span_start(), guard_offsets)
        {
            continue;
        }

        analysis.flows.push(TaintFlow {
            function_symbol_id: function_symbol_id.to_string(),
            variable: identifier.clone(),
            source_kind,
            sink_kind: sink.kind,
            sink_span: sink.span.clone(),
            sink_line: sink.line.clone(),
        });
    }
}

trait SinkSiteSpanExt {
    fn sink_span_start(&self) -> u32;
}

impl SinkSiteSpanExt for SinkSite {
    fn sink_span_start(&self) -> u32 {
        self.span.start
    }
}

fn is_guarded_before_sink(variable: &str, sink_start: u32, guards: &BTreeMap<String, u32>) -> bool {
    guards
        .get(variable)
        .is_some_and(|guard_offset| *guard_offset < sink_start)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use aztec_lint_core::config::AztecConfig;

    use crate::detect::SourceUnit;
    use crate::model_builder::build_aztec_model;
    use crate::taint::graph::{TaintSinkKind, TaintSourceKind, build_def_use_graph};

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
}
