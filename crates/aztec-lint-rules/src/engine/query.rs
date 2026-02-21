use aztec_lint_core::model::{
    CfgBlock, CfgEdge, DfgEdge, ExpressionCategory, SemanticExpression, SemanticFunction,
    SemanticModel, SemanticStatement, StatementCategory,
};

#[derive(Clone, Copy, Debug)]
pub struct RuleQuery<'a> {
    semantic: &'a SemanticModel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalBinding<'a> {
    pub definition_node_id: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CfgView<'a> {
    pub blocks: Vec<&'a CfgBlock>,
    pub edges: Vec<&'a CfgEdge>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DfgView<'a> {
    pub edges: Vec<&'a DfgEdge>,
}

impl<'a> RuleQuery<'a> {
    pub fn new(semantic: &'a SemanticModel) -> Self {
        Self { semantic }
    }

    pub fn functions(&self) -> &[SemanticFunction] {
        &self.semantic.functions
    }

    pub fn locals_in_function(&self, function_symbol_id: &str) -> Vec<LocalBinding<'a>> {
        let mut locals = self
            .semantic
            .dfg_edges
            .iter()
            .filter(|edge| {
                edge.function_symbol_id == function_symbol_id
                    && edge.to_node_id.starts_with("def::")
            })
            .map(|edge| LocalBinding {
                definition_node_id: edge.to_node_id.as_str(),
            })
            .collect::<Vec<_>>();
        locals.sort_by_key(|binding| binding.definition_node_id);
        locals.dedup_by(|left, right| left.definition_node_id == right.definition_node_id);
        locals
    }

    pub fn index_accesses(&self, function_symbol_id: Option<&str>) -> Vec<&'a SemanticExpression> {
        let mut accesses = self
            .semantic
            .expressions
            .iter()
            .filter(|expression| expression.category == ExpressionCategory::Index)
            .filter(|expression| {
                function_symbol_id
                    .map(|symbol_id| expression.function_symbol_id == symbol_id)
                    .unwrap_or(true)
            })
            .collect::<Vec<_>>();
        accesses.sort_by_key(|expression| expression.expr_id.as_str());
        accesses
    }

    pub fn assertions(&self, function_symbol_id: Option<&str>) -> Vec<&'a SemanticStatement> {
        let mut assertions = self
            .semantic
            .statements
            .iter()
            .filter(|statement| {
                matches!(
                    statement.category,
                    StatementCategory::Assert | StatementCategory::Constrain
                )
            })
            .filter(|statement| {
                function_symbol_id
                    .map(|symbol_id| statement.function_symbol_id == symbol_id)
                    .unwrap_or(true)
            })
            .collect::<Vec<_>>();
        assertions.sort_by_key(|statement| statement.stmt_id.as_str());
        assertions
    }

    pub fn cfg(&self, function_symbol_id: &str) -> CfgView<'a> {
        let mut blocks = self
            .semantic
            .cfg_blocks
            .iter()
            .filter(|block| block.function_symbol_id == function_symbol_id)
            .collect::<Vec<_>>();
        blocks.sort_by_key(|block| block.block_id.as_str());

        let mut edges = self
            .semantic
            .cfg_edges
            .iter()
            .filter(|edge| edge.function_symbol_id == function_symbol_id)
            .collect::<Vec<_>>();
        edges.sort_by_key(|edge| {
            (
                edge.from_block_id.as_str(),
                edge.to_block_id.as_str(),
                edge.kind,
            )
        });

        CfgView { blocks, edges }
    }

    pub fn dfg(&self, function_symbol_id: &str) -> DfgView<'a> {
        let mut edges = self
            .semantic
            .dfg_edges
            .iter()
            .filter(|edge| edge.function_symbol_id == function_symbol_id)
            .collect::<Vec<_>>();
        edges.sort_by_key(|edge| {
            (
                edge.from_node_id.as_str(),
                edge.to_node_id.as_str(),
                edge.kind,
            )
        });
        DfgView { edges }
    }
}
