use std::collections::BTreeMap;

use aztec_lint_core::diagnostics::{Diagnostic, normalize_file_path};
use aztec_lint_core::model::StatementCategory;
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::engine::context::{RuleContext, SourceFile};
use crate::noir_core::util::{
    is_ident_continue, text_fallback_function_scopes, text_fallback_statement_bindings,
};

pub struct Noir002ShadowingRule;

impl Rule for Noir002ShadowingRule {
    fn id(&self) -> &'static str {
        "NOIR002"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        if semantic_available(ctx) {
            self.run_semantic(ctx, out);
            return;
        }
        self.run_text_fallback(ctx, out);
    }
}

impl Noir002ShadowingRule {
    fn run_semantic(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let semantic = ctx.semantic_model();
        let files = file_map(ctx.files());

        for function in &semantic.functions {
            let normalized_file = normalize_file_path(&function.span.file);
            let Some(file) = files.get(&normalized_file).copied() else {
                continue;
            };

            let bindings = bindings_for_function(semantic, &function.symbol_id, file);

            let mut declared = Vec::<DeclaredBinding>::new();
            for binding in bindings {
                if binding.name.starts_with('_') {
                    continue;
                }
                let scope_path = scope_path_for_offset(file.text(), binding.start);
                if declared.iter().any(|prior| {
                    prior.name == binding.name
                        && prior.active_from <= binding.start
                        && scope_path_is_prefix(&prior.scope_path, &scope_path)
                }) {
                    out.push(ctx.diagnostic(
                        self.id(),
                        CORRECTNESS,
                        format!("`{}` shadows an existing binding in scope", binding.name),
                        file.span_for_range(binding.start, binding.start + binding.name.len()),
                    ));
                }
                declared.push(DeclaredBinding {
                    name: binding.name,
                    active_from: binding.active_from,
                    scope_path,
                });
            }
        }
    }

    fn run_text_fallback(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            for scope in text_fallback_function_scopes(file.text()) {
                let body_start = scope.body_start.saturating_add(1);
                let body_end = scope.body_end.saturating_sub(1);
                if body_start >= body_end || body_end > file.text().len() {
                    continue;
                }
                let body = &file.text()[body_start..body_end];
                let bindings = let_bindings_with_scope_paths(body, body_start);
                let mut active = Vec::<Binding>::new();
                for binding in bindings {
                    if active.iter().any(|existing| {
                        existing.name == binding.name
                            && existing.active_from <= binding.start
                            && scope_path_is_prefix(&existing.scope_path, &binding.scope_path)
                    }) {
                        out.push(ctx.diagnostic(
                            self.id(),
                            CORRECTNESS,
                            format!("`{}` shadows an existing binding in scope", binding.name),
                            file.span_for_range(binding.start, binding.start + binding.name.len()),
                        ));
                    }

                    active.push(binding);
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SemanticBinding {
    name: String,
    start: usize,
    active_from: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeclaredBinding {
    name: String,
    active_from: usize,
    scope_path: Vec<usize>,
}

fn semantic_available(ctx: &RuleContext<'_>) -> bool {
    let semantic = ctx.semantic_model();
    !semantic.functions.is_empty() && !semantic.statements.is_empty()
}

fn file_map(files: &[SourceFile]) -> BTreeMap<String, &SourceFile> {
    files
        .iter()
        .map(|file| (normalize_file_path(file.path()), file))
        .collect::<BTreeMap<_, _>>()
}

fn source_slice(source: &str, start: u32, end: u32) -> Option<&str> {
    let start = usize::try_from(start).ok()?;
    let end = usize::try_from(end).ok()?;
    if start >= end || end > source.len() {
        return None;
    }
    source.get(start..end)
}

fn bindings_for_function(
    semantic: &aztec_lint_core::model::SemanticModel,
    function_symbol_id: &str,
    file: &SourceFile,
) -> Vec<SemanticBinding> {
    let normalized_file = normalize_file_path(file.path());
    let mut bindings = Vec::<SemanticBinding>::new();

    let mut let_statements = semantic
        .statements
        .iter()
        .filter(|statement| statement.function_symbol_id == function_symbol_id)
        .filter(|statement| statement.category == StatementCategory::Let)
        .collect::<Vec<_>>();
    let_statements.sort_by_key(|statement| (statement.span.start, statement.span.end));

    for statement in let_statements {
        if normalize_file_path(&statement.span.file) != normalized_file {
            continue;
        }
        let Some(statement_source) =
            source_slice(file.text(), statement.span.start, statement.span.end)
        else {
            continue;
        };
        let Some(statement_start) = usize::try_from(statement.span.start).ok() else {
            continue;
        };
        let Some(statement_end) = usize::try_from(statement.span.end).ok() else {
            continue;
        };
        for (name, relative_start) in text_fallback_statement_bindings(statement_source) {
            let binding_start = statement_start.saturating_add(relative_start);
            bindings.push(SemanticBinding {
                name,
                start: binding_start,
                // Let bindings become visible only after initializer evaluation.
                active_from: statement_end.max(binding_start),
            });
        }
    }

    bindings.sort_by_key(|binding| (binding.start, binding.name.clone()));
    bindings
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Binding {
    name: String,
    start: usize,
    active_from: usize,
    scope_path: Vec<usize>,
}

fn let_bindings_with_scope_paths(source: &str, offset: usize) -> Vec<Binding> {
    let bytes = source.as_bytes();
    let mut scope_stack = vec![0usize];
    let mut next_scope_id = 1usize;
    let mut idx = 0usize;
    let mut out = Vec::<Binding>::new();

    while idx < bytes.len() {
        match bytes[idx] {
            b'{' => {
                scope_stack.push(next_scope_id);
                next_scope_id += 1;
                idx += 1;
                continue;
            }
            b'}' => {
                if scope_stack.len() > 1 {
                    scope_stack.pop();
                }
                idx += 1;
                continue;
            }
            b'/' if bytes.get(idx + 1) == Some(&b'/') => {
                while idx < bytes.len() && bytes[idx] != b'\n' {
                    idx += 1;
                }
                continue;
            }
            _ => {}
        }

        let Some((name, name_start, next_idx)) = parse_let_binding(source, idx) else {
            idx += 1;
            continue;
        };
        let active_from = statement_end_offset(source, idx)
            .unwrap_or(next_idx)
            .max(name_start + 1);
        out.push(Binding {
            name,
            start: offset + name_start,
            active_from: offset + active_from,
            scope_path: scope_stack.clone(),
        });
        idx = next_idx;
    }

    out
}

fn scope_path_is_prefix(prefix: &[usize], candidate: &[usize]) -> bool {
    prefix.len() <= candidate.len() && prefix == &candidate[..prefix.len()]
}

fn scope_path_for_offset(source: &str, offset: usize) -> Vec<usize> {
    let bytes = source.as_bytes();
    let limit = offset.min(bytes.len());
    let mut scope_stack = vec![0usize];
    let mut next_scope_id = 1usize;
    let mut idx = 0usize;

    while idx < limit {
        match bytes[idx] {
            b'/' if bytes.get(idx + 1) == Some(&b'/') => {
                while idx < limit && bytes[idx] != b'\n' {
                    idx += 1;
                }
                continue;
            }
            b'"' | b'\'' => {
                idx = skip_quoted_literal(bytes, idx, limit);
                continue;
            }
            b'{' => {
                scope_stack.push(next_scope_id);
                next_scope_id += 1;
            }
            b'}' => {
                if scope_stack.len() > 1 {
                    scope_stack.pop();
                }
            }
            _ => {}
        }
        idx += 1;
    }

    scope_stack
}

fn skip_quoted_literal(bytes: &[u8], start: usize, limit: usize) -> usize {
    let quote = bytes[start];
    let mut idx = start + 1;
    while idx < limit {
        if bytes[idx] == b'\\' {
            idx = idx.saturating_add(2);
            continue;
        }
        if bytes[idx] == quote {
            return idx + 1;
        }
        idx += 1;
    }
    limit
}

fn statement_end_offset(source: &str, start_idx: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if start_idx >= bytes.len() {
        return None;
    }

    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut idx = start_idx;

    while idx < bytes.len() {
        match bytes[idx] {
            b'/' if bytes.get(idx + 1) == Some(&b'/') => {
                while idx < bytes.len() && bytes[idx] != b'\n' {
                    idx += 1;
                }
                continue;
            }
            b'"' | b'\'' => {
                idx = skip_quoted_literal(bytes, idx, bytes.len());
                continue;
            }
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'{' => brace_depth += 1,
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b';' if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                return Some(idx + 1);
            }
            _ => {}
        }
        idx += 1;
    }

    None
}

fn parse_let_binding(source: &str, start_idx: usize) -> Option<(String, usize, usize)> {
    let bytes = source.as_bytes();
    if start_idx + 3 > bytes.len() || &bytes[start_idx..start_idx + 3] != b"let" {
        return None;
    }

    let left_boundary = start_idx == 0 || !is_ident_continue(bytes[start_idx - 1]);
    let right_boundary = bytes
        .get(start_idx + 3)
        .is_some_and(|byte| byte.is_ascii_whitespace());
    if !left_boundary || !right_boundary {
        return None;
    }

    let mut idx = start_idx + 3;
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    if bytes.get(idx..idx + 3) == Some(b"mut") {
        let after_mut = idx + 3;
        if bytes
            .get(after_mut)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            idx = after_mut;
            while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
                idx += 1;
            }
        }
    }

    let first = bytes.get(idx)?;
    if !(first.is_ascii_alphabetic() || *first == b'_') {
        return None;
    }

    let name_start = idx;
    idx += 1;
    while idx < bytes.len() && is_ident_continue(bytes[idx]) {
        idx += 1;
    }
    let name = source[name_start..idx].to_string();
    if name == "_" {
        return None;
    }

    Some((name, name_start, idx))
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::{
        ExpressionCategory, ProjectModel, SemanticExpression, SemanticFunction, SemanticStatement,
        Span, StatementCategory, TypeCategory,
    };

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir002ShadowingRule;

    #[test]
    fn detects_shadowed_binding() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let value = 1; { let value = 2; } }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir002ShadowingRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("shadows"));
    }

    #[test]
    fn ignores_distinct_bindings() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let left = 1; let right = 2; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir002ShadowingRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_same_name_bindings_in_if_else_siblings() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main(flag: bool) { let value = if flag { let owner = 1; owner } else { let owner = 2; owner }; assert(value > 0); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir002ShadowingRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_binding_name_reuse_inside_let_initializer_branches() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main(flag: bool) { let owner = if flag { let owner = 1; owner } else { let owner = 2; owner }; assert(owner > 0); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir002ShadowingRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_rebinding_after_nested_scope_closes_on_same_line() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { { let value = 1; } let value = 2; assert(value == 2); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir002ShadowingRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_same_binding_name_in_different_functions() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn a() { let notes = 1; assert(notes == 1); } fn b() { let notes = 2; assert(notes == 2); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir002ShadowingRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_scope_analysis_detects_shadowed_binding() {
        let source = "fn main() { let value = 1; { let value = 2; assert(value == 2); } }";
        let (function_start, function_end) = span_range(
            source,
            "fn main() { let value = 1; { let value = 2; assert(value == 2); } }",
        );
        let (outer_block_start, outer_block_end) = span_range(
            source,
            "{ let value = 1; { let value = 2; assert(value == 2); } }",
        );
        let (inner_block_start, inner_block_end) =
            span_range(source, "{ let value = 2; assert(value == 2); }");
        let (outer_stmt_start, outer_stmt_end) = span_range(source, "let value = 1;");
        let (inner_stmt_start, inner_stmt_end) = span_range(source, "let value = 2;");

        let mut project = ProjectModel::default();
        project.semantic.functions.push(SemanticFunction {
            symbol_id: "fn::main".to_string(),
            name: "main".to_string(),
            module_symbol_id: "module::main".to_string(),
            return_type_repr: "()".to_string(),
            return_type_category: TypeCategory::Unknown,
            parameter_types: Vec::new(),
            is_entrypoint: true,
            is_unconstrained: false,
            span: Span::new("src/main.nr", function_start, function_end, 1, 1),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::block::outer".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Block,
            type_category: TypeCategory::Unknown,
            type_repr: "()".to_string(),
            span: Span::new("src/main.nr", outer_block_start, outer_block_end, 1, 1),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::block::inner".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Block,
            type_category: TypeCategory::Unknown,
            type_repr: "()".to_string(),
            span: Span::new("src/main.nr", inner_block_start, inner_block_end, 1, 1),
        });
        project.semantic.statements.push(SemanticStatement {
            stmt_id: "stmt::outer".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: StatementCategory::Let,
            span: Span::new("src/main.nr", outer_stmt_start, outer_stmt_end, 1, 1),
        });
        project.semantic.statements.push(SemanticStatement {
            stmt_id: "stmt::inner".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: StatementCategory::Let,
            span: Span::new("src/main.nr", inner_stmt_start, inner_stmt_end, 1, 1),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir002ShadowingRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("shadows"));
    }

    #[test]
    fn semantic_scope_analysis_ignores_if_else_sibling_bindings() {
        let source = "fn main(flag: bool) { let value = if flag { let owner = 1; owner } else { let owner = 2; owner }; assert(value > 0); }";
        let (function_start, function_end) = span_range(source, source);
        let (let_then_start, let_then_end) = nth_span_range(source, "let owner = 1;", 0);
        let (let_else_start, let_else_end) = nth_span_range(source, "let owner = 2;", 0);

        let mut project = ProjectModel::default();
        project.semantic.functions.push(SemanticFunction {
            symbol_id: "fn::main".to_string(),
            name: "main".to_string(),
            module_symbol_id: "module::main".to_string(),
            return_type_repr: "()".to_string(),
            return_type_category: TypeCategory::Unknown,
            parameter_types: vec!["bool".to_string()],
            is_entrypoint: true,
            is_unconstrained: false,
            span: Span::new("src/main.nr", function_start, function_end, 1, 1),
        });
        project.semantic.statements.push(SemanticStatement {
            stmt_id: "stmt::then".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: StatementCategory::Let,
            span: Span::new("src/main.nr", let_then_start, let_then_end, 1, 1),
        });
        project.semantic.statements.push(SemanticStatement {
            stmt_id: "stmt::else".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: StatementCategory::Let,
            span: Span::new("src/main.nr", let_else_start, let_else_end, 1, 1),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir002ShadowingRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_scope_analysis_ignores_initializer_branch_reuse() {
        let source = "fn main(flag: bool) { let owner = if flag { let owner = 1; owner } else { let owner = 2; owner }; assert(owner > 0); }";
        let (function_start, function_end) = span_range(source, source);
        let (outer_let_start, outer_let_end) = span_range(
            source,
            "let owner = if flag { let owner = 1; owner } else { let owner = 2; owner };",
        );
        let (inner_then_start, inner_then_end) = nth_span_range(source, "let owner = 1;", 0);
        let (inner_else_start, inner_else_end) = nth_span_range(source, "let owner = 2;", 0);

        let mut project = ProjectModel::default();
        project.semantic.functions.push(SemanticFunction {
            symbol_id: "fn::main".to_string(),
            name: "main".to_string(),
            module_symbol_id: "module::main".to_string(),
            return_type_repr: "()".to_string(),
            return_type_category: TypeCategory::Unknown,
            parameter_types: vec!["bool".to_string()],
            is_entrypoint: true,
            is_unconstrained: false,
            span: Span::new("src/main.nr", function_start, function_end, 1, 1),
        });
        project.semantic.statements.push(SemanticStatement {
            stmt_id: "stmt::outer".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: StatementCategory::Let,
            span: Span::new("src/main.nr", outer_let_start, outer_let_end, 1, 1),
        });
        project.semantic.statements.push(SemanticStatement {
            stmt_id: "stmt::then".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: StatementCategory::Let,
            span: Span::new("src/main.nr", inner_then_start, inner_then_end, 1, 1),
        });
        project.semantic.statements.push(SemanticStatement {
            stmt_id: "stmt::else".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: StatementCategory::Let,
            span: Span::new("src/main.nr", inner_else_start, inner_else_end, 1, 1),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir002ShadowingRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_scope_analysis_ignores_rebinding_after_nested_scope_closes() {
        let source = "fn main() { { let value = 1; } let value = 2; assert(value == 2); }";
        let (function_start, function_end) = span_range(
            source,
            "fn main() { { let value = 1; } let value = 2; assert(value == 2); }",
        );
        let (outer_block_start, outer_block_end) = span_range(
            source,
            "{ { let value = 1; } let value = 2; assert(value == 2); }",
        );
        let (inner_block_start, inner_block_end) = span_range(source, "{ let value = 1; }");
        let (inner_stmt_start, inner_stmt_end) = span_range(source, "let value = 1;");
        let (outer_stmt_start, outer_stmt_end) = nth_span_range(source, "let value = 2;", 0);

        let mut project = ProjectModel::default();
        project.semantic.functions.push(SemanticFunction {
            symbol_id: "fn::main".to_string(),
            name: "main".to_string(),
            module_symbol_id: "module::main".to_string(),
            return_type_repr: "()".to_string(),
            return_type_category: TypeCategory::Unknown,
            parameter_types: Vec::new(),
            is_entrypoint: true,
            is_unconstrained: false,
            span: Span::new("src/main.nr", function_start, function_end, 1, 1),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::block::outer".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Block,
            type_category: TypeCategory::Unknown,
            type_repr: "()".to_string(),
            span: Span::new("src/main.nr", outer_block_start, outer_block_end, 1, 1),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::block::inner".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Block,
            type_category: TypeCategory::Unknown,
            type_repr: "()".to_string(),
            span: Span::new("src/main.nr", inner_block_start, inner_block_end, 1, 1),
        });
        project.semantic.statements.push(SemanticStatement {
            stmt_id: "stmt::inner".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: StatementCategory::Let,
            span: Span::new("src/main.nr", inner_stmt_start, inner_stmt_end, 1, 1),
        });
        project.semantic.statements.push(SemanticStatement {
            stmt_id: "stmt::outer".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: StatementCategory::Let,
            span: Span::new("src/main.nr", outer_stmt_start, outer_stmt_end, 1, 1),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir002ShadowingRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    fn span_range(source: &str, needle: &str) -> (u32, u32) {
        nth_span_range(source, needle, 0)
    }

    fn nth_span_range(source: &str, needle: &str, index: usize) -> (u32, u32) {
        let start = source
            .match_indices(needle)
            .nth(index)
            .map(|(offset, _)| offset)
            .expect("needle occurrence should exist");
        let end = start + needle.len();
        (
            u32::try_from(start).unwrap_or(u32::MAX),
            u32::try_from(end).unwrap_or(u32::MAX),
        )
    }
}
