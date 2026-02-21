#[cfg(feature = "noir-compiler")]
use std::collections::HashSet;

#[cfg(feature = "noir-compiler")]
use crate::model::CallEdge;
#[cfg(feature = "noir-compiler")]
use crate::model::SemanticModel;
#[cfg(feature = "noir-compiler")]
use crate::noir::span_mapper::SpanMapper;

#[cfg(feature = "noir-compiler")]
use noirc_driver::CrateId;
#[cfg(feature = "noir-compiler")]
use noirc_frontend::hir::Context;
#[cfg(feature = "noir-compiler")]
use noirc_frontend::hir_def::expr::{
    HirArrayLiteral, HirCallExpression, HirConstrainExpression, HirExpression, HirLiteral, HirMatch,
};
#[cfg(feature = "noir-compiler")]
use noirc_frontend::hir_def::stmt::{HirLValue, HirStatement};
#[cfg(feature = "noir-compiler")]
use noirc_frontend::node_interner::{DefinitionKind, ExprId, FuncId, NodeInterner, StmtId};

#[cfg(feature = "noir-compiler")]
pub fn extract_best_effort_call_edges(
    context: &Context<'static, 'static>,
    crate_id: CrateId,
    span_mapper: &SpanMapper<'_>,
) -> Vec<CallEdge> {
    let Some(def_map) = context.def_map(&crate_id) else {
        return Vec::new();
    };
    let interner = &context.def_interner;
    let mut edges = Vec::<CallEdge>::new();

    for (_, module) in def_map.modules().iter() {
        if !span_mapper.is_user_file(module.location.file) {
            continue;
        }

        let mut functions = module
            .value_definitions()
            .filter_map(|definition| definition.as_function())
            .collect::<Vec<_>>();
        functions.sort_by_key(|function_id| format!("{function_id}"));

        for caller in functions {
            let Some(meta) = interner.try_function_meta(&caller) else {
                continue;
            };
            if !span_mapper.is_user_file(meta.location.file) {
                continue;
            }

            let body = interner.function(&caller);
            let Some(body_expr) = body.try_as_expr() else {
                continue;
            };

            let mut visited_exprs = HashSet::<ExprId>::new();
            let mut visited_statements = HashSet::<StmtId>::new();
            visit_expression(
                caller,
                body_expr,
                interner,
                span_mapper,
                &mut edges,
                &mut visited_exprs,
                &mut visited_statements,
            );
        }
    }

    edges.sort_by_key(|edge| {
        (
            edge.caller_symbol_id.clone(),
            edge.callee_symbol_id.clone(),
            edge.span.file.clone(),
            edge.span.start,
            edge.span.end,
        )
    });
    edges.dedup();
    edges
}

#[cfg(feature = "noir-compiler")]
pub fn call_edges_from_semantic(semantic: &SemanticModel) -> Vec<CallEdge> {
    let mut edges = semantic
        .call_sites
        .iter()
        .map(|call_site| CallEdge {
            caller_symbol_id: call_site.function_symbol_id.clone(),
            callee_symbol_id: call_site.callee_symbol_id.clone(),
            span: call_site.span.clone(),
        })
        .collect::<Vec<_>>();
    edges.sort_by_key(|edge| {
        (
            edge.caller_symbol_id.clone(),
            edge.callee_symbol_id.clone(),
            edge.span.file.clone(),
            edge.span.start,
            edge.span.end,
        )
    });
    edges.dedup();
    edges
}

#[cfg(feature = "noir-compiler")]
fn visit_statement(
    caller: FuncId,
    stmt_id: StmtId,
    interner: &NodeInterner,
    span_mapper: &SpanMapper<'_>,
    edges: &mut Vec<CallEdge>,
    visited_exprs: &mut HashSet<ExprId>,
    visited_statements: &mut HashSet<StmtId>,
) {
    if !visited_statements.insert(stmt_id) {
        return;
    }

    match interner.statement(&stmt_id) {
        HirStatement::Let(let_stmt) => {
            visit_expression(
                caller,
                let_stmt.expression,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
        HirStatement::Assign(assign) => {
            visit_lvalue(
                caller,
                &assign.lvalue,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
            visit_expression(
                caller,
                assign.expression,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
        HirStatement::For(for_stmt) => {
            visit_expression(
                caller,
                for_stmt.start_range,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
            visit_expression(
                caller,
                for_stmt.end_range,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
            visit_expression(
                caller,
                for_stmt.block,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
        HirStatement::Loop(expr) | HirStatement::Expression(expr) | HirStatement::Semi(expr) => {
            visit_expression(
                caller,
                expr,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
        HirStatement::While(condition, body) => {
            visit_expression(
                caller,
                condition,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
            visit_expression(
                caller,
                body,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
        HirStatement::Comptime(inner) => {
            visit_statement(
                caller,
                inner,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
        HirStatement::Break | HirStatement::Continue | HirStatement::Error => {}
    }
}

#[cfg(feature = "noir-compiler")]
fn visit_lvalue(
    caller: FuncId,
    lvalue: &HirLValue,
    interner: &NodeInterner,
    span_mapper: &SpanMapper<'_>,
    edges: &mut Vec<CallEdge>,
    visited_exprs: &mut HashSet<ExprId>,
    visited_statements: &mut HashSet<StmtId>,
) {
    match lvalue {
        HirLValue::Ident(_, _) | HirLValue::Error { .. } => {}
        HirLValue::MemberAccess { object, .. } => {
            visit_lvalue(
                caller,
                object,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
        HirLValue::Index { array, index, .. } => {
            visit_lvalue(
                caller,
                array,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
            visit_expression(
                caller,
                *index,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
        HirLValue::Dereference { lvalue, .. } => {
            visit_lvalue(
                caller,
                lvalue,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
    }
}

#[cfg(feature = "noir-compiler")]
fn visit_expression(
    caller: FuncId,
    expr_id: ExprId,
    interner: &NodeInterner,
    span_mapper: &SpanMapper<'_>,
    edges: &mut Vec<CallEdge>,
    visited_exprs: &mut HashSet<ExprId>,
    visited_statements: &mut HashSet<StmtId>,
) {
    if !visited_exprs.insert(expr_id) {
        return;
    }

    match interner.expression(&expr_id) {
        HirExpression::Ident(_, _)
        | HirExpression::Error
        | HirExpression::Quote(_)
        | HirExpression::Unquote(_) => {}
        HirExpression::Literal(literal) => match literal {
            HirLiteral::Array(array) | HirLiteral::Vector(array) => {
                visit_array_literal(
                    caller,
                    &array,
                    interner,
                    span_mapper,
                    edges,
                    visited_exprs,
                    visited_statements,
                );
            }
            HirLiteral::FmtStr(_, values, _) => {
                for value in values {
                    visit_expression(
                        caller,
                        value,
                        interner,
                        span_mapper,
                        edges,
                        visited_exprs,
                        visited_statements,
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
                visit_statement(
                    caller,
                    statement,
                    interner,
                    span_mapper,
                    edges,
                    visited_exprs,
                    visited_statements,
                );
            }
        }
        HirExpression::Prefix(prefix) => {
            visit_expression(
                caller,
                prefix.rhs,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
        HirExpression::Infix(infix) => {
            visit_expression(
                caller,
                infix.lhs,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
            visit_expression(
                caller,
                infix.rhs,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
        HirExpression::Index(index) => {
            visit_expression(
                caller,
                index.collection,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
            visit_expression(
                caller,
                index.index,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
        HirExpression::Constructor(constructor) => {
            for (_, value) in constructor.fields {
                visit_expression(
                    caller,
                    value,
                    interner,
                    span_mapper,
                    edges,
                    visited_exprs,
                    visited_statements,
                );
            }
        }
        HirExpression::EnumConstructor(constructor) => {
            for value in constructor.arguments {
                visit_expression(
                    caller,
                    value,
                    interner,
                    span_mapper,
                    edges,
                    visited_exprs,
                    visited_statements,
                );
            }
        }
        HirExpression::MemberAccess(member) => {
            visit_expression(
                caller,
                member.lhs,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
        HirExpression::Call(call) => {
            maybe_push_call_edge(caller, &call, interner, span_mapper, edges);
            visit_expression(
                caller,
                call.func,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
            for argument in call.arguments {
                visit_expression(
                    caller,
                    argument,
                    interner,
                    span_mapper,
                    edges,
                    visited_exprs,
                    visited_statements,
                );
            }
        }
        HirExpression::Constrain(HirConstrainExpression(condition, _, message)) => {
            visit_expression(
                caller,
                condition,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
            if let Some(message) = message {
                visit_expression(
                    caller,
                    message,
                    interner,
                    span_mapper,
                    edges,
                    visited_exprs,
                    visited_statements,
                );
            }
        }
        HirExpression::Cast(cast) => {
            visit_expression(
                caller,
                cast.lhs,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
        HirExpression::If(if_expr) => {
            visit_expression(
                caller,
                if_expr.condition,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
            visit_expression(
                caller,
                if_expr.consequence,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
            if let Some(alternative) = if_expr.alternative {
                visit_expression(
                    caller,
                    alternative,
                    interner,
                    span_mapper,
                    edges,
                    visited_exprs,
                    visited_statements,
                );
            }
        }
        HirExpression::Match(match_expr) => {
            visit_match(
                caller,
                &match_expr,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
        HirExpression::Tuple(values) => {
            for value in values {
                visit_expression(
                    caller,
                    value,
                    interner,
                    span_mapper,
                    edges,
                    visited_exprs,
                    visited_statements,
                );
            }
        }
        HirExpression::Lambda(lambda) => {
            visit_expression(
                caller,
                lambda.body,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
    }
}

#[cfg(feature = "noir-compiler")]
fn visit_array_literal(
    caller: FuncId,
    literal: &HirArrayLiteral,
    interner: &NodeInterner,
    span_mapper: &SpanMapper<'_>,
    edges: &mut Vec<CallEdge>,
    visited_exprs: &mut HashSet<ExprId>,
    visited_statements: &mut HashSet<StmtId>,
) {
    match literal {
        HirArrayLiteral::Standard(values) => {
            for value in values {
                visit_expression(
                    caller,
                    *value,
                    interner,
                    span_mapper,
                    edges,
                    visited_exprs,
                    visited_statements,
                );
            }
        }
        HirArrayLiteral::Repeated {
            repeated_element, ..
        } => {
            visit_expression(
                caller,
                *repeated_element,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
    }
}

#[cfg(feature = "noir-compiler")]
fn visit_match(
    caller: FuncId,
    match_expr: &HirMatch,
    interner: &NodeInterner,
    span_mapper: &SpanMapper<'_>,
    edges: &mut Vec<CallEdge>,
    visited_exprs: &mut HashSet<ExprId>,
    visited_statements: &mut HashSet<StmtId>,
) {
    match match_expr {
        HirMatch::Success(expr) => visit_expression(
            caller,
            *expr,
            interner,
            span_mapper,
            edges,
            visited_exprs,
            visited_statements,
        ),
        HirMatch::Failure { .. } => {}
        HirMatch::Guard {
            cond,
            body,
            otherwise,
        } => {
            visit_expression(
                caller,
                *cond,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
            visit_expression(
                caller,
                *body,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
            visit_match(
                caller,
                otherwise,
                interner,
                span_mapper,
                edges,
                visited_exprs,
                visited_statements,
            );
        }
        HirMatch::Switch(_, cases, fallback) => {
            for case in cases {
                visit_match(
                    caller,
                    &case.body,
                    interner,
                    span_mapper,
                    edges,
                    visited_exprs,
                    visited_statements,
                );
            }
            if let Some(fallback) = fallback {
                visit_match(
                    caller,
                    fallback,
                    interner,
                    span_mapper,
                    edges,
                    visited_exprs,
                    visited_statements,
                );
            }
        }
    }
}

#[cfg(feature = "noir-compiler")]
fn maybe_push_call_edge(
    caller: FuncId,
    call: &HirCallExpression,
    interner: &NodeInterner,
    span_mapper: &SpanMapper<'_>,
    edges: &mut Vec<CallEdge>,
) {
    let Some(callee) = resolve_called_function(call.func, interner) else {
        return;
    };
    if caller == callee {
        return;
    }

    let span = span_mapper.map_location(call.location);
    edges.push(CallEdge {
        caller_symbol_id: format!("fn::{caller}"),
        callee_symbol_id: format!("fn::{callee}"),
        span,
    });
}

#[cfg(feature = "noir-compiler")]
pub(crate) fn resolve_called_function(expr_id: ExprId, interner: &NodeInterner) -> Option<FuncId> {
    let mut current = expr_id;
    let mut visited = HashSet::new();

    loop {
        if !visited.insert(current) {
            return None;
        }
        match interner.expression(&current) {
            HirExpression::Ident(identifier, _) => match interner.definition(identifier.id).kind {
                DefinitionKind::Function(function_id) => return Some(function_id),
                DefinitionKind::Local(Some(rhs)) => current = rhs,
                DefinitionKind::Local(None)
                | DefinitionKind::Global(_)
                | DefinitionKind::NumericGeneric(_, _)
                | DefinitionKind::AssociatedConstant(_, _) => return None,
            },
            _ => return None,
        }
    }
}
