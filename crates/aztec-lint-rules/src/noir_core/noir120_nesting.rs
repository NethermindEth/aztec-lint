use std::collections::BTreeMap;

use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::diagnostics::normalize_file_path;
use aztec_lint_core::model::ExpressionCategory;
use aztec_lint_core::policy::MAINTAINABILITY;

use crate::Rule;
use crate::engine::context::{RuleContext, SourceFile};
use crate::noir_core::util::text_fallback_function_scopes;

pub struct Noir120NestingRule;

const NESTING_LIMIT: usize = 3;

impl Rule for Noir120NestingRule {
    fn id(&self) -> &'static str {
        "NOIR120"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        if semantic_available(ctx) {
            self.run_semantic(ctx, out);
            return;
        }
        self.run_text_fallback(ctx, out);
    }
}

impl Noir120NestingRule {
    fn run_semantic(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let semantic = ctx.semantic_model();
        let files = file_map(ctx.files());

        for function in &semantic.functions {
            let normalized_file = normalize_file_path(&function.span.file);
            let Some(file) = files.get(&normalized_file).copied() else {
                continue;
            };
            let mut blocks = semantic
                .expressions
                .iter()
                .filter(|expression| expression.function_symbol_id == function.symbol_id)
                .filter(|expression| expression.category == ExpressionCategory::Block)
                .filter(|expression| normalize_file_path(&expression.span.file) == normalized_file)
                .filter_map(|expression| {
                    let start = usize::try_from(expression.span.start).ok()?;
                    let end = usize::try_from(expression.span.end).ok()?;
                    if start >= end {
                        return None;
                    }
                    Some((start, end))
                })
                .collect::<Vec<_>>();

            blocks.sort_by_key(|(start, end)| (*start, std::cmp::Reverse(*end)));
            let max_depth = max_nested_block_depth(&blocks);
            let logical_depth = max_depth.saturating_sub(1);
            if logical_depth <= NESTING_LIMIT {
                continue;
            }

            let Some(name_start) = usize::try_from(function.span.start).ok() else {
                continue;
            };
            let Some(name_end) = usize::try_from(function.span.end).ok() else {
                continue;
            };
            out.push(ctx.diagnostic(
                self.id(),
                MAINTAINABILITY,
                format!(
                    "function `{}` nesting depth is {logical_depth} (limit: {NESTING_LIMIT})",
                    function.name
                ),
                file.span_for_range(name_start, name_end),
            ));
        }
    }

    fn run_text_fallback(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            let source = file.text();
            for function in text_fallback_function_scopes(source) {
                let body = &source[function.body_start..function.body_end];
                let max_depth = max_brace_depth(body);
                let logical_depth = max_depth.saturating_sub(1);
                if logical_depth <= NESTING_LIMIT {
                    continue;
                }

                out.push(ctx.diagnostic(
                    self.id(),
                    MAINTAINABILITY,
                    format!(
                        "function `{}` nesting depth is {logical_depth} (limit: {NESTING_LIMIT})",
                        function.name
                    ),
                    file.span_for_range(
                        function.name_offset,
                        function.name_offset + function.name.len(),
                    ),
                ));
            }
        }
    }
}

fn semantic_available(ctx: &RuleContext<'_>) -> bool {
    let semantic = ctx.semantic_model();
    !semantic.functions.is_empty()
        && semantic
            .expressions
            .iter()
            .any(|expression| expression.category == ExpressionCategory::Block)
}

fn file_map(files: &[SourceFile]) -> BTreeMap<String, &SourceFile> {
    files
        .iter()
        .map(|file| (normalize_file_path(file.path()), file))
        .collect::<BTreeMap<_, _>>()
}

fn max_nested_block_depth(blocks: &[(usize, usize)]) -> usize {
    let mut stack = Vec::<usize>::new();
    let mut max_depth = 0usize;

    for (start, end) in blocks {
        while let Some(last_end) = stack.last().copied() {
            if *start >= last_end {
                stack.pop();
                continue;
            }
            if *end <= last_end {
                break;
            }
            stack.pop();
        }

        stack.push(*end);
        max_depth = max_depth.max(stack.len());
    }

    max_depth
}

fn max_brace_depth(body: &str) -> usize {
    let mut depth = 0usize;
    let mut max_depth = 0usize;

    for byte in body.bytes() {
        match byte {
            b'{' => {
                depth += 1;
                max_depth = max_depth.max(depth);
            }
            b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }

    max_depth
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::{
        ExpressionCategory, ProjectModel, SemanticExpression, SemanticFunction, Span, TypeCategory,
    };

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir120NestingRule;

    #[test]
    fn reports_deep_nesting() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { if true { if true { if true { if true { let x = 1; } } } } }"
                    .to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir120NestingRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_shallow_nesting() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { if true { let x = 1; } }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir120NestingRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_blocks_report_deep_nesting() {
        let source = "fn main() { if true { if true { if true { if true { let x = 1; } } } } }";
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
            span: Span::new("src/main.nr", 3, 7, 1, 4),
        });
        for (index, (start, end)) in brace_blocks(source).into_iter().enumerate() {
            project.semantic.expressions.push(SemanticExpression {
                expr_id: format!("expr::block::{index}"),
                function_symbol_id: "fn::main".to_string(),
                category: ExpressionCategory::Block,
                type_category: TypeCategory::Unknown,
                type_repr: "()".to_string(),
                span: Span::new("src/main.nr", start, end, 1, 1),
            });
        }
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );

        let mut diagnostics = Vec::new();
        Noir120NestingRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn semantic_blocks_ignore_shallow_nesting() {
        let source = "fn main() { if true { let x = 1; } }";
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
            span: Span::new("src/main.nr", 3, 7, 1, 4),
        });
        for (index, (start, end)) in brace_blocks(source).into_iter().enumerate() {
            project.semantic.expressions.push(SemanticExpression {
                expr_id: format!("expr::block::{index}"),
                function_symbol_id: "fn::main".to_string(),
                category: ExpressionCategory::Block,
                type_category: TypeCategory::Unknown,
                type_repr: "()".to_string(),
                span: Span::new("src/main.nr", start, end, 1, 1),
            });
        }
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir120NestingRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn incomplete_semantic_model_falls_back_to_text_for_nesting() {
        let source = "fn main() { if true { if true { if true { if true { let x = 1; } } } } }";
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
            span: Span::new("src/main.nr", 3, 7, 1, 4),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::identifier".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Identifier,
            type_category: TypeCategory::Unknown,
            type_repr: "Field".to_string(),
            span: Span::new("src/main.nr", 20, 21, 1, 1),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir120NestingRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    fn brace_blocks(source: &str) -> Vec<(u32, u32)> {
        let mut stack = Vec::<usize>::new();
        let mut blocks = Vec::<(u32, u32)>::new();

        for (index, ch) in source.char_indices() {
            match ch {
                '{' => stack.push(index),
                '}' => {
                    let Some(start) = stack.pop() else {
                        continue;
                    };
                    blocks.push((
                        u32::try_from(start).unwrap_or(u32::MAX),
                        u32::try_from(index + 1).unwrap_or(u32::MAX),
                    ));
                }
                _ => {}
            }
        }

        blocks
    }
}
