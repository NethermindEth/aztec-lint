use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::model::Span;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SemanticModel {
    pub functions: Vec<SemanticFunction>,
    pub expressions: Vec<SemanticExpression>,
    pub statements: Vec<SemanticStatement>,
    pub cfg_blocks: Vec<CfgBlock>,
    pub cfg_edges: Vec<CfgEdge>,
    pub dfg_edges: Vec<DfgEdge>,
    pub call_sites: Vec<CallSite>,
    pub guard_nodes: Vec<GuardNode>,
}

impl SemanticModel {
    pub fn normalize(&mut self) {
        self.functions.sort_by_key(|item| {
            (
                item.symbol_id.clone(),
                item.name.clone(),
                item.module_symbol_id.clone(),
                item.return_type_repr.clone(),
                item.return_type_category,
                item.parameter_types.clone(),
                item.is_entrypoint,
                item.is_unconstrained,
                item.span.file.clone(),
                item.span.start,
                item.span.end,
            )
        });
        self.functions
            .dedup_by(|left, right| left.symbol_id == right.symbol_id);

        self.expressions.sort_by_key(|item| {
            (
                item.expr_id.clone(),
                item.function_symbol_id.clone(),
                item.category,
                item.type_category,
                item.type_repr.clone(),
                item.span.file.clone(),
                item.span.start,
                item.span.end,
            )
        });
        self.expressions
            .dedup_by(|left, right| left.expr_id == right.expr_id);

        self.statements.sort_by_key(|item| {
            (
                item.stmt_id.clone(),
                item.function_symbol_id.clone(),
                item.category,
                item.span.file.clone(),
                item.span.start,
                item.span.end,
            )
        });
        self.statements
            .dedup_by(|left, right| left.stmt_id == right.stmt_id);

        self.cfg_blocks.sort_by_key(|item| {
            (
                item.function_symbol_id.clone(),
                item.block_id.clone(),
                item.statement_ids.clone(),
            )
        });
        self.cfg_blocks.dedup_by(|left, right| {
            left.function_symbol_id == right.function_symbol_id && left.block_id == right.block_id
        });

        self.cfg_edges.sort_by_key(|item| {
            (
                item.function_symbol_id.clone(),
                item.from_block_id.clone(),
                item.to_block_id.clone(),
                item.kind,
            )
        });
        self.cfg_edges.dedup();

        self.dfg_edges.sort_by_key(|item| {
            (
                item.function_symbol_id.clone(),
                item.from_node_id.clone(),
                item.to_node_id.clone(),
                item.kind,
            )
        });
        self.dfg_edges.dedup();

        self.call_sites.sort_by_key(|item| {
            (
                item.call_site_id.clone(),
                item.function_symbol_id.clone(),
                item.callee_symbol_id.clone(),
                item.expr_id.clone(),
                item.span.file.clone(),
                item.span.start,
                item.span.end,
            )
        });
        self.call_sites
            .dedup_by(|left, right| left.call_site_id == right.call_site_id);

        self.guard_nodes.sort_by_key(|item| {
            (
                item.guard_id.clone(),
                item.function_symbol_id.clone(),
                item.kind,
                item.guarded_expr_id.clone(),
                item.span.file.clone(),
                item.span.start,
                item.span.end,
            )
        });
        self.guard_nodes
            .dedup_by(|left, right| left.guard_id == right.guard_id);
    }

    pub fn statement_block_map(&self, function_symbol_id: &str) -> BTreeMap<String, String> {
        self.cfg_blocks
            .iter()
            .filter(|block| block.function_symbol_id == function_symbol_id)
            .fold(BTreeMap::<String, String>::new(), |mut out, block| {
                for statement_id in &block.statement_ids {
                    out.insert(statement_id.clone(), block.block_id.clone());
                }
                out
            })
    }

    pub fn cfg_dominators(&self, function_symbol_id: &str) -> BTreeMap<String, BTreeSet<String>> {
        let blocks = self
            .cfg_blocks
            .iter()
            .filter(|block| block.function_symbol_id == function_symbol_id)
            .map(|block| block.block_id.clone())
            .collect::<BTreeSet<_>>();
        if blocks.is_empty() {
            return BTreeMap::new();
        }

        let mut predecessors = BTreeMap::<String, BTreeSet<String>>::new();
        for block_id in &blocks {
            predecessors.insert(block_id.clone(), BTreeSet::new());
        }
        for edge in self
            .cfg_edges
            .iter()
            .filter(|edge| edge.function_symbol_id == function_symbol_id)
        {
            if blocks.contains(&edge.from_block_id) && blocks.contains(&edge.to_block_id) {
                predecessors
                    .entry(edge.to_block_id.clone())
                    .or_default()
                    .insert(edge.from_block_id.clone());
            }
        }

        let entry_blocks = blocks
            .iter()
            .filter(|block_id| {
                predecessors
                    .get(*block_id)
                    .is_none_or(|predecessors| predecessors.is_empty())
            })
            .cloned()
            .collect::<BTreeSet<_>>();

        let mut dominators = BTreeMap::<String, BTreeSet<String>>::new();
        for block_id in &blocks {
            if entry_blocks.contains(block_id) {
                dominators.insert(block_id.clone(), BTreeSet::from([block_id.clone()]));
            } else {
                dominators.insert(block_id.clone(), blocks.clone());
            }
        }

        loop {
            let mut changed = false;
            for block_id in &blocks {
                if entry_blocks.contains(block_id) {
                    continue;
                }
                let predecessors = predecessors.get(block_id).cloned().unwrap_or_default();
                if predecessors.is_empty() {
                    let singleton = BTreeSet::from([block_id.clone()]);
                    if dominators.get(block_id) != Some(&singleton) {
                        dominators.insert(block_id.clone(), singleton);
                        changed = true;
                    }
                    continue;
                }

                let mut iter = predecessors.into_iter();
                let Some(first) = iter.next() else {
                    continue;
                };
                let mut next = dominators.get(&first).cloned().unwrap_or_default();
                for predecessor in iter {
                    let predecessor_dominators =
                        dominators.get(&predecessor).cloned().unwrap_or_default();
                    next = next
                        .intersection(&predecessor_dominators)
                        .cloned()
                        .collect::<BTreeSet<_>>();
                }
                next.insert(block_id.clone());

                if dominators.get(block_id) != Some(&next) {
                    dominators.insert(block_id.clone(), next);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        dominators
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SemanticFunction {
    pub symbol_id: String,
    pub name: String,
    pub module_symbol_id: String,
    pub return_type_repr: String,
    pub return_type_category: TypeCategory,
    pub parameter_types: Vec<String>,
    pub is_entrypoint: bool,
    pub is_unconstrained: bool,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SemanticExpression {
    pub expr_id: String,
    pub function_symbol_id: String,
    pub category: ExpressionCategory,
    pub type_category: TypeCategory,
    pub type_repr: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SemanticStatement {
    pub stmt_id: String,
    pub function_symbol_id: String,
    pub category: StatementCategory,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CfgBlock {
    pub function_symbol_id: String,
    pub block_id: String,
    pub statement_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CfgEdge {
    pub function_symbol_id: String,
    pub from_block_id: String,
    pub to_block_id: String,
    pub kind: CfgEdgeKind,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CfgEdgeKind {
    Unconditional,
    TrueBranch,
    FalseBranch,
    LoopBack,
    Exceptional,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct DfgEdge {
    pub function_symbol_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub kind: DfgEdgeKind,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DfgEdgeKind {
    DefUse,
    UseDef,
    Phi,
    Argument,
    Return,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CallSite {
    pub call_site_id: String,
    pub function_symbol_id: String,
    pub callee_symbol_id: String,
    pub expr_id: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct GuardNode {
    pub guard_id: String,
    pub function_symbol_id: String,
    pub kind: GuardKind,
    pub guarded_expr_id: Option<String>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardKind {
    Assert,
    Constrain,
    Range,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpressionCategory {
    Literal,
    Identifier,
    UnaryOp,
    BinaryOp,
    Call,
    Index,
    MemberAccess,
    Cast,
    Block,
    Tuple,
    Array,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatementCategory {
    Let,
    Assign,
    Expression,
    For,
    While,
    Loop,
    Break,
    Continue,
    Assert,
    Constrain,
    Return,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeCategory {
    Bool,
    Integer,
    Field,
    Array,
    Tuple,
    Struct,
    Function,
    Generic,
    Unknown,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::{from_str, to_vec};

    use super::{
        CallSite, CfgBlock, CfgEdge, CfgEdgeKind, DfgEdge, DfgEdgeKind, ExpressionCategory,
        GuardKind, GuardNode, SemanticExpression, SemanticFunction, SemanticModel,
        SemanticStatement, StatementCategory, TypeCategory,
    };
    use crate::model::Span;

    fn span(file: &str, start: u32) -> Span {
        Span::new(file.to_string(), start, start + 1, 1, 1)
    }

    fn sample_model() -> SemanticModel {
        SemanticModel {
            functions: vec![
                SemanticFunction {
                    symbol_id: "fn::b".to_string(),
                    name: "b".to_string(),
                    module_symbol_id: "module::z".to_string(),
                    return_type_repr: "Field".to_string(),
                    return_type_category: TypeCategory::Field,
                    parameter_types: vec![
                        "u32".to_string(),
                        "Field".to_string(),
                        "u32".to_string(),
                    ],
                    is_entrypoint: false,
                    is_unconstrained: false,
                    span: span("src/main.nr", 20),
                },
                SemanticFunction {
                    symbol_id: "fn::a".to_string(),
                    name: "a".to_string(),
                    module_symbol_id: "module::z".to_string(),
                    return_type_repr: "Field".to_string(),
                    return_type_category: TypeCategory::Field,
                    parameter_types: vec!["Field".to_string()],
                    is_entrypoint: true,
                    is_unconstrained: false,
                    span: span("src/main.nr", 10),
                },
                SemanticFunction {
                    symbol_id: "fn::a".to_string(),
                    name: "a".to_string(),
                    module_symbol_id: "module::z".to_string(),
                    return_type_repr: "Field".to_string(),
                    return_type_category: TypeCategory::Field,
                    parameter_types: vec!["Field".to_string()],
                    is_entrypoint: true,
                    is_unconstrained: false,
                    span: span("src/main.nr", 10),
                },
            ],
            expressions: vec![
                SemanticExpression {
                    expr_id: "expr::2".to_string(),
                    function_symbol_id: "fn::a".to_string(),
                    category: ExpressionCategory::Call,
                    type_category: TypeCategory::Field,
                    type_repr: "Field".to_string(),
                    span: span("src/main.nr", 30),
                },
                SemanticExpression {
                    expr_id: "expr::1".to_string(),
                    function_symbol_id: "fn::a".to_string(),
                    category: ExpressionCategory::Literal,
                    type_category: TypeCategory::Field,
                    type_repr: "Field".to_string(),
                    span: span("src/main.nr", 29),
                },
                SemanticExpression {
                    expr_id: "expr::1".to_string(),
                    function_symbol_id: "fn::a".to_string(),
                    category: ExpressionCategory::Literal,
                    type_category: TypeCategory::Field,
                    type_repr: "Field".to_string(),
                    span: span("src/main.nr", 29),
                },
            ],
            statements: vec![
                SemanticStatement {
                    stmt_id: "stmt::2".to_string(),
                    function_symbol_id: "fn::a".to_string(),
                    category: StatementCategory::Expression,
                    span: span("src/main.nr", 41),
                },
                SemanticStatement {
                    stmt_id: "stmt::1".to_string(),
                    function_symbol_id: "fn::a".to_string(),
                    category: StatementCategory::Let,
                    span: span("src/main.nr", 40),
                },
                SemanticStatement {
                    stmt_id: "stmt::1".to_string(),
                    function_symbol_id: "fn::a".to_string(),
                    category: StatementCategory::Let,
                    span: span("src/main.nr", 40),
                },
            ],
            cfg_blocks: vec![
                CfgBlock {
                    function_symbol_id: "fn::a".to_string(),
                    block_id: "bb1".to_string(),
                    statement_ids: vec![
                        "stmt::2".to_string(),
                        "stmt::1".to_string(),
                        "stmt::1".to_string(),
                    ],
                },
                CfgBlock {
                    function_symbol_id: "fn::a".to_string(),
                    block_id: "bb0".to_string(),
                    statement_ids: vec!["stmt::0".to_string()],
                },
                CfgBlock {
                    function_symbol_id: "fn::a".to_string(),
                    block_id: "bb0".to_string(),
                    statement_ids: vec!["stmt::0".to_string()],
                },
            ],
            cfg_edges: vec![
                CfgEdge {
                    function_symbol_id: "fn::a".to_string(),
                    from_block_id: "bb1".to_string(),
                    to_block_id: "bb2".to_string(),
                    kind: CfgEdgeKind::TrueBranch,
                },
                CfgEdge {
                    function_symbol_id: "fn::a".to_string(),
                    from_block_id: "bb1".to_string(),
                    to_block_id: "bb2".to_string(),
                    kind: CfgEdgeKind::TrueBranch,
                },
            ],
            dfg_edges: vec![
                DfgEdge {
                    function_symbol_id: "fn::a".to_string(),
                    from_node_id: "expr::1".to_string(),
                    to_node_id: "stmt::1".to_string(),
                    kind: DfgEdgeKind::DefUse,
                },
                DfgEdge {
                    function_symbol_id: "fn::a".to_string(),
                    from_node_id: "expr::1".to_string(),
                    to_node_id: "stmt::1".to_string(),
                    kind: DfgEdgeKind::DefUse,
                },
            ],
            call_sites: vec![
                CallSite {
                    call_site_id: "call::1".to_string(),
                    function_symbol_id: "fn::a".to_string(),
                    callee_symbol_id: "fn::b".to_string(),
                    expr_id: "expr::2".to_string(),
                    span: span("src/main.nr", 50),
                },
                CallSite {
                    call_site_id: "call::1".to_string(),
                    function_symbol_id: "fn::a".to_string(),
                    callee_symbol_id: "fn::b".to_string(),
                    expr_id: "expr::2".to_string(),
                    span: span("src/main.nr", 50),
                },
            ],
            guard_nodes: vec![
                GuardNode {
                    guard_id: "guard::2".to_string(),
                    function_symbol_id: "fn::a".to_string(),
                    kind: GuardKind::Range,
                    guarded_expr_id: Some("expr::2".to_string()),
                    span: span("src/main.nr", 61),
                },
                GuardNode {
                    guard_id: "guard::1".to_string(),
                    function_symbol_id: "fn::a".to_string(),
                    kind: GuardKind::Assert,
                    guarded_expr_id: Some("expr::1".to_string()),
                    span: span("src/main.nr", 60),
                },
                GuardNode {
                    guard_id: "guard::1".to_string(),
                    function_symbol_id: "fn::a".to_string(),
                    kind: GuardKind::Assert,
                    guarded_expr_id: Some("expr::1".to_string()),
                    span: span("src/main.nr", 60),
                },
            ],
        }
    }

    #[test]
    fn normalize_sorts_and_dedups_semantic_collections() {
        let mut model = sample_model();
        model.normalize();

        assert_eq!(model.functions.len(), 2);
        assert_eq!(model.functions[0].symbol_id, "fn::a");
        assert_eq!(model.functions[1].symbol_id, "fn::b");
        assert_eq!(
            model.functions[1].parameter_types,
            vec!["u32".to_string(), "Field".to_string(), "u32".to_string()]
        );

        assert_eq!(model.expressions.len(), 2);
        assert_eq!(model.expressions[0].expr_id, "expr::1");
        assert_eq!(model.expressions[1].expr_id, "expr::2");

        assert_eq!(model.statements.len(), 2);
        assert_eq!(model.statements[0].stmt_id, "stmt::1");
        assert_eq!(model.statements[1].stmt_id, "stmt::2");

        assert_eq!(model.cfg_blocks.len(), 2);
        assert_eq!(model.cfg_blocks[0].block_id, "bb0");
        assert_eq!(model.cfg_blocks[1].block_id, "bb1");
        assert_eq!(
            model.cfg_blocks[1].statement_ids,
            vec![
                "stmt::2".to_string(),
                "stmt::1".to_string(),
                "stmt::1".to_string()
            ]
        );

        assert_eq!(model.cfg_edges.len(), 1);
        assert_eq!(model.dfg_edges.len(), 1);
        assert_eq!(model.call_sites.len(), 1);

        assert_eq!(model.guard_nodes.len(), 2);
        assert_eq!(model.guard_nodes[0].guard_id, "guard::1");
        assert_eq!(model.guard_nodes[1].guard_id, "guard::2");
    }

    #[test]
    fn normalized_models_serialize_deterministically() {
        let mut left = sample_model();
        let mut right = sample_model();

        left.cfg_edges.reverse();
        right.functions.reverse();
        right.guard_nodes.reverse();

        left.normalize();
        right.normalize();

        assert_eq!(left, right);

        let left_json = to_vec(&left).expect("serialization should succeed");
        let right_json = to_vec(&right).expect("serialization should succeed");
        assert_eq!(left_json, right_json);
    }

    #[test]
    fn deserialize_partial_semantic_model_defaults_missing_collections() {
        let model = from_str::<SemanticModel>(
            r#"{
  "functions": []
}"#,
        )
        .expect("partial semantic model should deserialize");
        assert!(model.functions.is_empty());
        assert!(model.expressions.is_empty());
        assert!(model.statements.is_empty());
        assert!(model.cfg_blocks.is_empty());
        assert!(model.cfg_edges.is_empty());
        assert!(model.dfg_edges.is_empty());
        assert!(model.call_sites.is_empty());
        assert!(model.guard_nodes.is_empty());
    }

    #[test]
    fn statement_block_map_indexes_statement_blocks() {
        let model = SemanticModel {
            cfg_blocks: vec![
                CfgBlock {
                    function_symbol_id: "fn::a".to_string(),
                    block_id: "bb0".to_string(),
                    statement_ids: vec!["stmt::0".to_string()],
                },
                CfgBlock {
                    function_symbol_id: "fn::a".to_string(),
                    block_id: "bb1".to_string(),
                    statement_ids: vec!["stmt::1".to_string(), "stmt::2".to_string()],
                },
            ],
            ..SemanticModel::default()
        };

        let map = model.statement_block_map("fn::a");
        assert_eq!(map.get("stmt::0"), Some(&"bb0".to_string()));
        assert_eq!(map.get("stmt::1"), Some(&"bb1".to_string()));
        assert_eq!(map.get("stmt::2"), Some(&"bb1".to_string()));
    }

    #[test]
    fn cfg_dominators_computes_forward_dominance() {
        let model = SemanticModel {
            cfg_blocks: vec![
                CfgBlock {
                    function_symbol_id: "fn::a".to_string(),
                    block_id: "bb0".to_string(),
                    statement_ids: vec![],
                },
                CfgBlock {
                    function_symbol_id: "fn::a".to_string(),
                    block_id: "bb1".to_string(),
                    statement_ids: vec![],
                },
                CfgBlock {
                    function_symbol_id: "fn::a".to_string(),
                    block_id: "bb2".to_string(),
                    statement_ids: vec![],
                },
            ],
            cfg_edges: vec![
                CfgEdge {
                    function_symbol_id: "fn::a".to_string(),
                    from_block_id: "bb0".to_string(),
                    to_block_id: "bb1".to_string(),
                    kind: CfgEdgeKind::Unconditional,
                },
                CfgEdge {
                    function_symbol_id: "fn::a".to_string(),
                    from_block_id: "bb1".to_string(),
                    to_block_id: "bb2".to_string(),
                    kind: CfgEdgeKind::Unconditional,
                },
            ],
            ..SemanticModel::default()
        };

        let dominators = model.cfg_dominators("fn::a");
        assert_eq!(
            dominators.get("bb0"),
            Some(&BTreeSet::from(["bb0".to_string()]))
        );
        assert_eq!(
            dominators.get("bb1"),
            Some(&BTreeSet::from(["bb0".to_string(), "bb1".to_string()]))
        );
        assert_eq!(
            dominators.get("bb2"),
            Some(&BTreeSet::from([
                "bb0".to_string(),
                "bb1".to_string(),
                "bb2".to_string()
            ]))
        );
    }
}
