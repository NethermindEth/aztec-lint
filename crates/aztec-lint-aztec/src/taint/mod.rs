pub mod graph;
pub mod propagate;

pub use graph::{
    DefUseGraph, FunctionGraph, GuardSite, SinkSite, TaintSinkKind, TaintSource, TaintSourceKind,
    build_def_use_graph, build_def_use_graph_with_semantic,
};
pub use propagate::{TaintAnalysis, TaintFlow, TaintOptions, analyze_intra_procedural};
