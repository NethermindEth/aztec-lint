#[cfg(feature = "noir-compiler")]
use crate::model::SemanticModel;

#[cfg(feature = "noir-compiler")]
use std::collections::HashSet;

#[cfg(feature = "noir-compiler")]
use crate::model::{
    CallSite, CfgBlock, CfgEdge, CfgEdgeKind, DfgEdge, DfgEdgeKind, ExpressionCategory, GuardKind,
    GuardNode, SemanticExpression, SemanticFunction, SemanticStatement, StatementCategory,
    TypeCategory,
};
#[cfg(feature = "noir-compiler")]
use crate::noir::call_graph::resolve_called_function;
#[cfg(feature = "noir-compiler")]
use crate::noir::span_mapper::SpanMapper;
#[cfg(feature = "noir-compiler")]
use noirc_driver::CrateId;
#[cfg(feature = "noir-compiler")]
use noirc_frontend::hir::Context;
#[cfg(feature = "noir-compiler")]
use noirc_frontend::hir::def_map::LocalModuleId;
#[cfg(feature = "noir-compiler")]
use noirc_frontend::hir_def::expr::{
    HirArrayLiteral, HirCallExpression, HirConstrainExpression, HirExpression, HirLiteral, HirMatch,
};
#[cfg(feature = "noir-compiler")]
use noirc_frontend::hir_def::stmt::{HirLValue, HirPattern, HirStatement};
#[cfg(feature = "noir-compiler")]
use noirc_frontend::hir_def::types::Type;
#[cfg(feature = "noir-compiler")]
use noirc_frontend::node_interner::{DefinitionId, ExprId, NodeInterner, StmtId};

#[cfg(feature = "noir-compiler")]
pub fn extract_semantic_model(
    context: &Context<'static, 'static>,
    crate_id: CrateId,
    span_mapper: &SpanMapper<'_>,
) -> SemanticModel {
    let Some(def_map) = context.def_map(&crate_id) else {
        return SemanticModel::default();
    };

    let interner = &context.def_interner;
    let mut model = SemanticModel::default();

    for (_, module) in def_map.modules().iter() {
        if !span_mapper.is_user_file(module.location.file) {
            continue;
        }

        let mut functions = module
            .value_definitions()
            .filter_map(|definition| definition.as_function())
            .collect::<Vec<_>>();
        functions.sort_by_key(|function_id| format!("{function_id}"));

        for function_id in functions {
            let Some(meta) = interner.try_function_meta(&function_id) else {
                continue;
            };
            if !span_mapper.is_user_file(meta.location.file) {
                continue;
            }

            let function_symbol_id = format!("fn::{function_id}");
            let module_symbol_id = module_symbol_id_for_function(def_map, meta.source_module);
            model.functions.push(SemanticFunction {
                symbol_id: function_symbol_id.clone(),
                name: interner.function_name(&function_id).to_string(),
                module_symbol_id,
                return_type_repr: meta.return_type().to_string(),
                return_type_category: type_category(meta.return_type()),
                parameter_types: meta
                    .parameters
                    .iter()
                    .map(|(_, typ, _)| typ.to_string())
                    .collect(),
                is_entrypoint: meta.is_entry_point,
                is_unconstrained: meta.is_unconstrained(),
                span: span_mapper.map_location(meta.location),
            });

            let body = interner.function(&function_id);
            let Some(body_expr) = body.try_as_expr() else {
                continue;
            };

            let mut collector = FunctionCollector::new(function_symbol_id, interner, span_mapper);
            collector.seed_cfg(&top_level_statement_ids(body_expr, interner));
            collector.visit_expression(body_expr, None);
            collector.append_to(&mut model);
        }
    }

    model.normalize();
    model
}

#[cfg(feature = "noir-compiler")]
fn top_level_statement_ids(body_expr: ExprId, interner: &NodeInterner) -> Vec<StmtId> {
    match interner.expression(&body_expr) {
        HirExpression::Block(block) | HirExpression::Unsafe(block) => block.statements,
        _ => Vec::new(),
    }
}

#[cfg(feature = "noir-compiler")]
fn module_symbol_id_for_function(
    def_map: &noirc_frontend::hir::def_map::CrateDefMap,
    module_id: LocalModuleId,
) -> String {
    let module = &def_map[module_id];
    let raw = def_map.get_module_path(module_id, module.parent);
    if raw.is_empty() {
        "module::<root>".to_string()
    } else {
        format!("module::{raw}")
    }
}

#[cfg(feature = "noir-compiler")]
struct FunctionCollector<'a> {
    function_symbol_id: String,
    interner: &'a NodeInterner,
    span_mapper: &'a SpanMapper<'a>,
    visited_exprs: HashSet<ExprId>,
    visited_statements: HashSet<StmtId>,
    expressions: Vec<SemanticExpression>,
    statements: Vec<SemanticStatement>,
    cfg_blocks: Vec<CfgBlock>,
    cfg_edges: Vec<CfgEdge>,
    dfg_edges: Vec<DfgEdge>,
    call_sites: Vec<CallSite>,
    guard_nodes: Vec<GuardNode>,
}

#[cfg(feature = "noir-compiler")]
impl<'a> FunctionCollector<'a> {
    fn new(
        function_symbol_id: String,
        interner: &'a NodeInterner,
        span_mapper: &'a SpanMapper<'a>,
    ) -> Self {
        Self {
            function_symbol_id,
            interner,
            span_mapper,
            visited_exprs: HashSet::new(),
            visited_statements: HashSet::new(),
            expressions: Vec::new(),
            statements: Vec::new(),
            cfg_blocks: Vec::new(),
            cfg_edges: Vec::new(),
            dfg_edges: Vec::new(),
            call_sites: Vec::new(),
            guard_nodes: Vec::new(),
        }
    }

    fn seed_cfg(&mut self, statements: &[StmtId]) {
        let entry_block = self.cfg_entry_id();
        let exit_block = self.cfg_exit_id();
        self.cfg_blocks.push(CfgBlock {
            function_symbol_id: self.function_symbol_id.clone(),
            block_id: entry_block.clone(),
            statement_ids: Vec::new(),
        });
        self.cfg_blocks.push(CfgBlock {
            function_symbol_id: self.function_symbol_id.clone(),
            block_id: exit_block.clone(),
            statement_ids: Vec::new(),
        });

        if statements.is_empty() {
            self.push_cfg_edge(&entry_block, &exit_block, CfgEdgeKind::Unconditional);
            return;
        }

        for (index, stmt_id) in statements.iter().enumerate() {
            self.cfg_blocks.push(CfgBlock {
                function_symbol_id: self.function_symbol_id.clone(),
                block_id: self.cfg_statement_block_id(index),
                statement_ids: vec![stmt_node_id(*stmt_id)],
            });
        }

        self.push_cfg_edge(
            &entry_block,
            &self.cfg_statement_block_id(0),
            CfgEdgeKind::Unconditional,
        );

        for (index, stmt_id) in statements.iter().enumerate() {
            let current = self.cfg_statement_block_id(index);
            let next = if index + 1 < statements.len() {
                self.cfg_statement_block_id(index + 1)
            } else {
                exit_block.clone()
            };
            self.push_cfg_edge(&current, &next, CfgEdgeKind::Unconditional);

            let statement = self.interner.statement(stmt_id);
            if statement_has_branch(&statement, self.interner) {
                self.push_cfg_edge(&current, &next, CfgEdgeKind::TrueBranch);
                self.push_cfg_edge(&current, &next, CfgEdgeKind::FalseBranch);
            }
            if statement_is_looping(&statement) {
                self.push_cfg_edge(&current, &current, CfgEdgeKind::LoopBack);
            }
        }
    }

    fn append_to(self, model: &mut SemanticModel) {
        model.expressions.extend(self.expressions);
        model.statements.extend(self.statements);
        model.cfg_blocks.extend(self.cfg_blocks);
        model.cfg_edges.extend(self.cfg_edges);
        model.dfg_edges.extend(self.dfg_edges);
        model.call_sites.extend(self.call_sites);
        model.guard_nodes.extend(self.guard_nodes);
    }

    fn visit_statement(&mut self, stmt_id: StmtId) {
        if !self.visited_statements.insert(stmt_id) {
            return;
        }

        let statement = self.interner.statement(&stmt_id);
        self.statements.push(SemanticStatement {
            stmt_id: stmt_node_id(stmt_id),
            function_symbol_id: self.function_symbol_id.clone(),
            category: statement_category(&statement, self.interner),
            span: self
                .span_mapper
                .map_location(self.interner.statement_location(stmt_id)),
        });

        match statement {
            HirStatement::Let(let_stmt) => {
                self.push_dfg_edge(
                    &expr_node_id(let_stmt.expression),
                    &stmt_node_id(stmt_id),
                    DfgEdgeKind::DefUse,
                );
                self.visit_expression(let_stmt.expression, Some(stmt_id));
                for definition_id in pattern_definition_ids(&let_stmt.pattern) {
                    if self.interner.definition_name(definition_id) == "_" {
                        continue;
                    }
                    self.push_dfg_edge(
                        &stmt_node_id(stmt_id),
                        &definition_node_id(definition_id),
                        DfgEdgeKind::DefUse,
                    );
                }
            }
            HirStatement::Assign(assign) => {
                self.push_dfg_edge(
                    &expr_node_id(assign.expression),
                    &stmt_node_id(stmt_id),
                    DfgEdgeKind::DefUse,
                );
                self.visit_lvalue(&assign.lvalue, stmt_id);
                self.visit_expression(assign.expression, Some(stmt_id));
            }
            HirStatement::For(for_stmt) => {
                self.push_dfg_edge(
                    &expr_node_id(for_stmt.start_range),
                    &stmt_node_id(stmt_id),
                    DfgEdgeKind::DefUse,
                );
                self.push_dfg_edge(
                    &expr_node_id(for_stmt.end_range),
                    &stmt_node_id(stmt_id),
                    DfgEdgeKind::DefUse,
                );
                self.push_dfg_edge(
                    &expr_node_id(for_stmt.block),
                    &stmt_node_id(stmt_id),
                    DfgEdgeKind::DefUse,
                );
                self.visit_expression(for_stmt.start_range, Some(stmt_id));
                self.visit_expression(for_stmt.end_range, Some(stmt_id));
                self.visit_expression(for_stmt.block, Some(stmt_id));
            }
            HirStatement::Loop(expr)
            | HirStatement::Expression(expr)
            | HirStatement::Semi(expr) => {
                self.push_dfg_edge(
                    &expr_node_id(expr),
                    &stmt_node_id(stmt_id),
                    DfgEdgeKind::DefUse,
                );
                self.visit_expression(expr, Some(stmt_id));
            }
            HirStatement::While(condition, body) => {
                self.push_dfg_edge(
                    &expr_node_id(condition),
                    &stmt_node_id(stmt_id),
                    DfgEdgeKind::DefUse,
                );
                self.push_dfg_edge(
                    &expr_node_id(body),
                    &stmt_node_id(stmt_id),
                    DfgEdgeKind::DefUse,
                );
                self.visit_expression(condition, Some(stmt_id));
                self.visit_expression(body, Some(stmt_id));
            }
            HirStatement::Comptime(inner) => self.visit_statement(inner),
            HirStatement::Break | HirStatement::Continue | HirStatement::Error => {}
        }
    }

    fn visit_lvalue(&mut self, lvalue: &HirLValue, statement_id: StmtId) {
        match lvalue {
            HirLValue::Ident(ident, _) => {
                self.push_dfg_edge(
                    &definition_node_id(ident.id),
                    &stmt_node_id(statement_id),
                    DfgEdgeKind::UseDef,
                );
            }
            HirLValue::MemberAccess { object, .. } => self.visit_lvalue(object, statement_id),
            HirLValue::Index { array, index, .. } => {
                self.visit_lvalue(array, statement_id);
                self.push_dfg_edge(
                    &expr_node_id(*index),
                    &stmt_node_id(statement_id),
                    DfgEdgeKind::DefUse,
                );
                self.visit_expression(*index, Some(statement_id));
            }
            HirLValue::Dereference { lvalue, .. } => self.visit_lvalue(lvalue, statement_id),
            HirLValue::Error { .. } => {}
        }
    }

    fn visit_expression(&mut self, expr_id: ExprId, statement_id: Option<StmtId>) {
        if !self.visited_exprs.insert(expr_id) {
            return;
        }

        let expr = self.interner.expression(&expr_id);
        let expr_type = self.interner.id_type(expr_id);
        self.expressions.push(SemanticExpression {
            expr_id: expr_node_id(expr_id),
            function_symbol_id: self.function_symbol_id.clone(),
            category: expression_category(&expr),
            type_category: type_category(&expr_type),
            type_repr: expr_type.to_string(),
            span: self
                .span_mapper
                .map_location(self.interner.expr_location(&expr_id)),
        });

        match expr {
            HirExpression::Ident(ident, _) => {
                self.push_dfg_edge(
                    &definition_node_id(ident.id),
                    &expr_node_id(expr_id),
                    DfgEdgeKind::UseDef,
                );
            }
            HirExpression::Error | HirExpression::Quote(_) | HirExpression::Unquote(_) => {}
            HirExpression::Literal(literal) => match literal {
                HirLiteral::Array(array) | HirLiteral::Vector(array) => {
                    self.visit_array_literal(expr_id, &array, statement_id);
                }
                HirLiteral::FmtStr(_, values, _) => {
                    for value in values {
                        self.visit_expression_child(
                            expr_id,
                            value,
                            statement_id,
                            DfgEdgeKind::Argument,
                        );
                    }
                }
                HirLiteral::Bool(_)
                | HirLiteral::Integer(_)
                | HirLiteral::Str(_)
                | HirLiteral::Unit => {}
            },
            HirExpression::Block(block) | HirExpression::Unsafe(block) => {
                for statement in block.statements {
                    self.push_dfg_edge(
                        &stmt_node_id(statement),
                        &expr_node_id(expr_id),
                        DfgEdgeKind::UseDef,
                    );
                    self.visit_statement(statement);
                }
            }
            HirExpression::Prefix(prefix) => {
                self.visit_expression_child(expr_id, prefix.rhs, statement_id, DfgEdgeKind::DefUse);
            }
            HirExpression::Infix(infix) => {
                self.visit_expression_child(expr_id, infix.lhs, statement_id, DfgEdgeKind::DefUse);
                self.visit_expression_child(expr_id, infix.rhs, statement_id, DfgEdgeKind::DefUse);
            }
            HirExpression::Index(index) => {
                self.visit_expression_child(
                    expr_id,
                    index.collection,
                    statement_id,
                    DfgEdgeKind::DefUse,
                );
                self.visit_expression_child(
                    expr_id,
                    index.index,
                    statement_id,
                    DfgEdgeKind::Argument,
                );
            }
            HirExpression::Constructor(constructor) => {
                for (_, value) in constructor.fields {
                    self.visit_expression_child(
                        expr_id,
                        value,
                        statement_id,
                        DfgEdgeKind::Argument,
                    );
                }
            }
            HirExpression::EnumConstructor(constructor) => {
                for value in constructor.arguments {
                    self.visit_expression_child(
                        expr_id,
                        value,
                        statement_id,
                        DfgEdgeKind::Argument,
                    );
                }
            }
            HirExpression::MemberAccess(member) => {
                self.visit_expression_child(expr_id, member.lhs, statement_id, DfgEdgeKind::DefUse);
            }
            HirExpression::Call(call) => {
                self.record_call_site(expr_id, &call);
                if let Some(first_argument) = call.arguments.first() {
                    if call_is_assert_like(call.func, self.interner) {
                        self.guard_nodes.push(GuardNode {
                            guard_id: format!("guard::assert::{expr_id:?}"),
                            function_symbol_id: self.function_symbol_id.clone(),
                            kind: GuardKind::Assert,
                            guarded_expr_id: Some(expr_node_id(*first_argument)),
                            span: self.span_mapper.map_location(call.location),
                        });
                    }
                    if call_is_range_like(call.func, self.interner) {
                        self.guard_nodes.push(GuardNode {
                            guard_id: format!("guard::range::{expr_id:?}"),
                            function_symbol_id: self.function_symbol_id.clone(),
                            kind: GuardKind::Range,
                            guarded_expr_id: Some(expr_node_id(*first_argument)),
                            span: self.span_mapper.map_location(call.location),
                        });
                    }
                }
                self.visit_expression_child(expr_id, call.func, statement_id, DfgEdgeKind::DefUse);
                for argument in call.arguments {
                    self.visit_expression_child(
                        expr_id,
                        argument,
                        statement_id,
                        DfgEdgeKind::Argument,
                    );
                }
            }
            HirExpression::Constrain(HirConstrainExpression(condition, _, message)) => {
                self.guard_nodes.push(GuardNode {
                    guard_id: format!("guard::constrain::{expr_id:?}"),
                    function_symbol_id: self.function_symbol_id.clone(),
                    kind: GuardKind::Constrain,
                    guarded_expr_id: Some(expr_node_id(condition)),
                    span: self
                        .span_mapper
                        .map_location(self.interner.expr_location(&expr_id)),
                });
                self.visit_expression_child(expr_id, condition, statement_id, DfgEdgeKind::DefUse);
                if let Some(message) = message {
                    self.visit_expression_child(
                        expr_id,
                        message,
                        statement_id,
                        DfgEdgeKind::Argument,
                    );
                }
            }
            HirExpression::Cast(cast) => {
                self.visit_expression_child(expr_id, cast.lhs, statement_id, DfgEdgeKind::DefUse);
            }
            HirExpression::If(if_expr) => {
                self.visit_expression_child(
                    expr_id,
                    if_expr.condition,
                    statement_id,
                    DfgEdgeKind::DefUse,
                );
                self.visit_expression_child(
                    expr_id,
                    if_expr.consequence,
                    statement_id,
                    DfgEdgeKind::DefUse,
                );
                if let Some(alternative) = if_expr.alternative {
                    self.visit_expression_child(
                        expr_id,
                        alternative,
                        statement_id,
                        DfgEdgeKind::DefUse,
                    );
                }
            }
            HirExpression::Match(match_expr) => {
                self.visit_match(expr_id, &match_expr, statement_id);
            }
            HirExpression::Tuple(values) => {
                for value in values {
                    self.visit_expression_child(
                        expr_id,
                        value,
                        statement_id,
                        DfgEdgeKind::Argument,
                    );
                }
            }
            HirExpression::Lambda(lambda) => {
                self.visit_expression_child(
                    expr_id,
                    lambda.body,
                    statement_id,
                    DfgEdgeKind::DefUse,
                );
            }
        }
    }

    fn visit_expression_child(
        &mut self,
        parent_expr_id: ExprId,
        child_expr_id: ExprId,
        statement_id: Option<StmtId>,
        kind: DfgEdgeKind,
    ) {
        self.push_dfg_edge(
            &expr_node_id(child_expr_id),
            &expr_node_id(parent_expr_id),
            kind,
        );
        self.visit_expression(child_expr_id, statement_id);
    }

    fn visit_array_literal(
        &mut self,
        parent_expr_id: ExprId,
        literal: &HirArrayLiteral,
        statement_id: Option<StmtId>,
    ) {
        match literal {
            HirArrayLiteral::Standard(values) => {
                for value in values {
                    self.visit_expression_child(
                        parent_expr_id,
                        *value,
                        statement_id,
                        DfgEdgeKind::Argument,
                    );
                }
            }
            HirArrayLiteral::Repeated {
                repeated_element, ..
            } => {
                self.visit_expression_child(
                    parent_expr_id,
                    *repeated_element,
                    statement_id,
                    DfgEdgeKind::Argument,
                );
            }
        }
    }

    fn visit_match(
        &mut self,
        parent_expr_id: ExprId,
        match_expr: &HirMatch,
        statement_id: Option<StmtId>,
    ) {
        match match_expr {
            HirMatch::Success(expr) => {
                self.visit_expression_child(
                    parent_expr_id,
                    *expr,
                    statement_id,
                    DfgEdgeKind::DefUse,
                );
            }
            HirMatch::Failure { .. } => {}
            HirMatch::Guard {
                cond,
                body,
                otherwise,
            } => {
                self.visit_expression_child(
                    parent_expr_id,
                    *cond,
                    statement_id,
                    DfgEdgeKind::DefUse,
                );
                self.visit_expression_child(
                    parent_expr_id,
                    *body,
                    statement_id,
                    DfgEdgeKind::DefUse,
                );
                self.visit_match(parent_expr_id, otherwise, statement_id);
            }
            HirMatch::Switch(_, cases, fallback) => {
                for case in cases {
                    self.visit_match(parent_expr_id, &case.body, statement_id);
                }
                if let Some(fallback) = fallback {
                    self.visit_match(parent_expr_id, fallback, statement_id);
                }
            }
        }
    }

    fn record_call_site(&mut self, call_expr_id: ExprId, call: &HirCallExpression) {
        let Some(callee) = resolve_called_function(call.func, self.interner) else {
            return;
        };

        self.call_sites.push(CallSite {
            call_site_id: format!("call::{}::{call_expr_id:?}", self.function_symbol_id),
            function_symbol_id: self.function_symbol_id.clone(),
            callee_symbol_id: format!("fn::{callee}"),
            expr_id: expr_node_id(call_expr_id),
            span: self.span_mapper.map_location(call.location),
        });
    }

    fn push_cfg_edge(&mut self, from_block_id: &str, to_block_id: &str, kind: CfgEdgeKind) {
        self.cfg_edges.push(CfgEdge {
            function_symbol_id: self.function_symbol_id.clone(),
            from_block_id: from_block_id.to_string(),
            to_block_id: to_block_id.to_string(),
            kind,
        });
    }

    fn push_dfg_edge(&mut self, from_node_id: &str, to_node_id: &str, kind: DfgEdgeKind) {
        self.dfg_edges.push(DfgEdge {
            function_symbol_id: self.function_symbol_id.clone(),
            from_node_id: from_node_id.to_string(),
            to_node_id: to_node_id.to_string(),
            kind,
        });
    }

    fn cfg_entry_id(&self) -> String {
        format!("bb::{}::entry", self.function_symbol_id)
    }

    fn cfg_exit_id(&self) -> String {
        format!("bb::{}::exit", self.function_symbol_id)
    }

    fn cfg_statement_block_id(&self, index: usize) -> String {
        format!("bb::{}::stmt:{index}", self.function_symbol_id)
    }
}

#[cfg(feature = "noir-compiler")]
fn expression_category(expression: &HirExpression) -> ExpressionCategory {
    match expression {
        HirExpression::Literal(_) => ExpressionCategory::Literal,
        HirExpression::Ident(_, _) => ExpressionCategory::Identifier,
        HirExpression::Prefix(_) => ExpressionCategory::UnaryOp,
        HirExpression::Infix(_) => ExpressionCategory::BinaryOp,
        HirExpression::Call(_) => ExpressionCategory::Call,
        HirExpression::Index(_) => ExpressionCategory::Index,
        HirExpression::MemberAccess(_) => ExpressionCategory::MemberAccess,
        HirExpression::Cast(_) => ExpressionCategory::Cast,
        HirExpression::Block(_) | HirExpression::Unsafe(_) => ExpressionCategory::Block,
        HirExpression::Tuple(_) => ExpressionCategory::Tuple,
        HirExpression::Constructor(_) | HirExpression::EnumConstructor(_) => {
            ExpressionCategory::Array
        }
        HirExpression::Constrain(_)
        | HirExpression::If(_)
        | HirExpression::Match(_)
        | HirExpression::Lambda(_)
        | HirExpression::Quote(_)
        | HirExpression::Unquote(_)
        | HirExpression::Error => ExpressionCategory::Unknown,
    }
}

#[cfg(feature = "noir-compiler")]
fn statement_category(statement: &HirStatement, interner: &NodeInterner) -> StatementCategory {
    match statement {
        HirStatement::Let(_) => StatementCategory::Let,
        HirStatement::Assign(_) => StatementCategory::Assign,
        HirStatement::For(_) => StatementCategory::For,
        HirStatement::Loop(_) => StatementCategory::Loop,
        HirStatement::While(_, _) => StatementCategory::While,
        HirStatement::Break => StatementCategory::Break,
        HirStatement::Continue => StatementCategory::Continue,
        HirStatement::Expression(expr) | HirStatement::Semi(expr) => {
            statement_category_from_expression(*expr, interner)
        }
        HirStatement::Comptime(_) | HirStatement::Error => StatementCategory::Unknown,
    }
}

#[cfg(feature = "noir-compiler")]
fn statement_category_from_expression(
    expr_id: ExprId,
    interner: &NodeInterner,
) -> StatementCategory {
    match interner.expression(&expr_id) {
        HirExpression::Constrain(_) => StatementCategory::Constrain,
        HirExpression::Call(call) if call_is_assert_like(call.func, interner) => {
            StatementCategory::Assert
        }
        _ => StatementCategory::Expression,
    }
}

#[cfg(feature = "noir-compiler")]
fn statement_has_branch(statement: &HirStatement, interner: &NodeInterner) -> bool {
    match statement {
        HirStatement::Expression(expr) | HirStatement::Semi(expr) => {
            matches!(
                interner.expression(expr),
                HirExpression::If(_) | HirExpression::Match(_)
            )
        }
        _ => false,
    }
}

#[cfg(feature = "noir-compiler")]
fn statement_is_looping(statement: &HirStatement) -> bool {
    matches!(
        statement,
        HirStatement::Loop(_) | HirStatement::While(_, _) | HirStatement::For(_)
    )
}

#[cfg(feature = "noir-compiler")]
fn call_is_assert_like(func_expr_id: ExprId, interner: &NodeInterner) -> bool {
    call_target_name(func_expr_id, interner).is_some_and(|name| {
        let normalized = name.to_ascii_lowercase();
        normalized == "assert" || normalized == "assert_eq" || normalized.starts_with("assert_")
    })
}

#[cfg(feature = "noir-compiler")]
fn call_is_range_like(func_expr_id: ExprId, interner: &NodeInterner) -> bool {
    call_target_name(func_expr_id, interner).is_some_and(|name| {
        let normalized = name.to_ascii_lowercase();
        normalized.contains("range") || normalized.contains("assert_max_bits")
    })
}

#[cfg(feature = "noir-compiler")]
fn call_target_name(func_expr_id: ExprId, interner: &NodeInterner) -> Option<String> {
    if let Some(function_id) = resolve_called_function(func_expr_id, interner) {
        return Some(interner.function_name(&function_id).to_string());
    }

    match interner.expression(&func_expr_id) {
        HirExpression::Ident(ident, _) => Some(interner.definition_name(ident.id).to_string()),
        _ => None,
    }
}

#[cfg(feature = "noir-compiler")]
fn pattern_definition_ids(pattern: &HirPattern) -> Vec<DefinitionId> {
    match pattern {
        HirPattern::Identifier(ident) => vec![ident.id],
        HirPattern::Mutable(inner, _) => pattern_definition_ids(inner),
        HirPattern::Tuple(fields, _) => fields
            .iter()
            .flat_map(pattern_definition_ids)
            .collect::<Vec<_>>(),
        HirPattern::Struct(_, fields, _) => fields
            .iter()
            .flat_map(|(_, field_pattern)| pattern_definition_ids(field_pattern))
            .collect::<Vec<_>>(),
    }
}

#[cfg(feature = "noir-compiler")]
fn type_category(typ: &Type) -> TypeCategory {
    match typ {
        Type::Bool => TypeCategory::Bool,
        Type::Integer(_, _) | Type::Constant(_, _) | Type::CheckedCast { .. } => {
            TypeCategory::Integer
        }
        Type::FieldElement => TypeCategory::Field,
        Type::Array(_, _) | Type::Vector(_) | Type::String(_) | Type::FmtString(_, _) => {
            TypeCategory::Array
        }
        Type::Tuple(_) => TypeCategory::Tuple,
        Type::DataType(_, _) | Type::Alias(_, _) => TypeCategory::Struct,
        Type::Function(_, _, _, _) => TypeCategory::Function,
        Type::TypeVariable(_)
        | Type::NamedGeneric(_)
        | Type::Forall(_, _)
        | Type::TraitAsType(_, _, _)
        | Type::Quoted(_)
        | Type::Reference(_, _)
        | Type::InfixExpr(_, _, _, _) => TypeCategory::Generic,
        Type::Unit | Type::Error => TypeCategory::Unknown,
    }
}

#[cfg(feature = "noir-compiler")]
fn expr_node_id(expr_id: ExprId) -> String {
    format!("expr::{expr_id:?}")
}

#[cfg(feature = "noir-compiler")]
fn stmt_node_id(stmt_id: StmtId) -> String {
    format!("stmt::{stmt_id:?}")
}

#[cfg(feature = "noir-compiler")]
fn definition_node_id(definition_id: DefinitionId) -> String {
    format!("def::{definition_id:?}")
}
