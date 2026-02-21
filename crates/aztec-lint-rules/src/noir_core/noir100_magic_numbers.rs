use std::collections::BTreeMap;

use aztec_lint_core::diagnostics::normalize_file_path;
use aztec_lint_core::diagnostics::{Applicability, Diagnostic};
use aztec_lint_core::model::{ExpressionCategory, TypeCategory};
use aztec_lint_core::policy::MAINTAINABILITY;

use crate::Rule;
use crate::engine::context::{RuleContext, SourceFile};
use crate::noir_core::util::{extract_numeric_literals, source_slice};

pub struct Noir100MagicNumbersRule;

impl Rule for Noir100MagicNumbersRule {
    fn id(&self) -> &'static str {
        "NOIR100"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        if semantic_available(ctx) {
            self.run_semantic(ctx, out);
            return;
        }
        self.run_text_fallback(ctx, out);
    }
}

impl Noir100MagicNumbersRule {
    fn run_semantic(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let semantic = ctx.semantic_model();
        let files = file_map(ctx.files());

        for expression in semantic.expressions.iter().filter(|expression| {
            expression.category == ExpressionCategory::Literal
                && matches!(
                    expression.type_category,
                    TypeCategory::Integer | TypeCategory::Field
                )
        }) {
            let normalized_file = normalize_file_path(&expression.span.file);
            let Some(file) = files.get(&normalized_file).copied() else {
                continue;
            };
            let Some(source) =
                source_slice(file.text(), expression.span.start, expression.span.end)
            else {
                continue;
            };
            let Some(expression_start) = usize::try_from(expression.span.start).ok() else {
                continue;
            };
            if is_constant_declaration_context(file.text(), expression_start) {
                continue;
            }

            for (literal, relative_offset) in extract_numeric_literals(source) {
                if is_zero_or_one_literal(&literal) {
                    continue;
                }

                let start = expression_start.saturating_add(relative_offset);
                let span = file.span_for_range(start, start + literal.len());
                out.push(
                    ctx.diagnostic(
                        self.id(),
                        MAINTAINABILITY,
                        format!("magic number `{literal}` should be named"),
                        span.clone(),
                    )
                    .help("extract this literal into a named constant for readability")
                    .span_suggestion(
                        span,
                        format!("replace `{literal}` with a named constant"),
                        "NAMED_CONSTANT".to_string(),
                        Applicability::MaybeIncorrect,
                    ),
                );
            }
        }
    }

    fn run_text_fallback(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            let mut offset = 0usize;

            for line in file.text().lines() {
                let code = strip_line_comment(line);

                for (literal, column) in extract_numeric_literals(code) {
                    let start = offset + column;
                    if is_constant_declaration_context(file.text(), start) {
                        continue;
                    }
                    if is_zero_or_one_literal(&literal) {
                        continue;
                    }

                    let span = file.span_for_range(start, start + literal.len());
                    out.push(
                        ctx.diagnostic(
                            self.id(),
                            MAINTAINABILITY,
                            format!("magic number `{literal}` should be named"),
                            span.clone(),
                        )
                        .help("extract this literal into a named constant for readability")
                        .span_suggestion(
                            span,
                            format!("replace `{literal}` with a named constant"),
                            "NAMED_CONSTANT".to_string(),
                            Applicability::MaybeIncorrect,
                        ),
                    );
                }

                offset += line.len() + 1;
            }
        }
    }
}

fn semantic_available(ctx: &RuleContext<'_>) -> bool {
    ctx.semantic_model().expressions.iter().any(|expression| {
        expression.category == ExpressionCategory::Literal
            && matches!(
                expression.type_category,
                TypeCategory::Integer | TypeCategory::Field
            )
    })
}

fn file_map(files: &[SourceFile]) -> BTreeMap<String, &SourceFile> {
    files
        .iter()
        .map(|file| (normalize_file_path(file.path()), file))
        .collect::<BTreeMap<_, _>>()
}

fn is_constant_declaration_context(source: &str, offset: usize) -> bool {
    if offset > source.len() {
        return false;
    }
    let statement_start = source[..offset]
        .rfind([';', '{', '}'])
        .map_or(0, |idx| idx + 1);
    let statement_prefix = source[statement_start..offset].trim_start();
    let mut tokens = statement_prefix.split_whitespace();
    match tokens.next() {
        Some("const") => true,
        Some("pub") => matches!(tokens.next(), Some("const")),
        _ => false,
    }
}

fn is_zero_or_one_literal(literal: &str) -> bool {
    let mut value = 0u8;
    for byte in literal.bytes() {
        if !byte.is_ascii_digit() {
            return false;
        }
        value = (value.saturating_mul(10).saturating_add(byte - b'0')).min(2);
    }
    value <= 1
}

fn strip_line_comment(line: &str) -> &str {
    line.split_once("//").map_or(line, |(code, _)| code)
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::{
        ExpressionCategory, ProjectModel, SemanticExpression, SemanticFunction, Span, TypeCategory,
    };

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir100MagicNumbersRule;

    #[test]
    fn reports_magic_numbers() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let fee = 42; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].structured_suggestions.len(), 1);
        assert_eq!(
            diagnostics[0].structured_suggestions[0].applicability,
            aztec_lint_core::diagnostics::Applicability::MaybeIncorrect
        );
    }

    #[test]
    fn ignores_constants_and_small_literals() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "const FEE: u32 = 42; fn main() { let flag = 1; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_literals_report_magic_numbers() {
        let source = "fn main() { let fee = 42; }";
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
        let literal_start = source.find("42").expect("literal should exist");
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::lit".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Literal,
            type_category: TypeCategory::Integer,
            type_repr: "u32".to_string(),
            span: Span::new(
                "src/main.nr",
                u32::try_from(literal_start).unwrap_or(u32::MAX),
                u32::try_from(literal_start + 2).unwrap_or(u32::MAX),
                1,
                23,
            ),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn semantic_literals_in_const_context_are_ignored() {
        let source = "const FEE: u32 = 42;\nfn main() { let one = 1; }";
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
            span: Span::new("src/main.nr", 25, 29, 2, 4),
        });
        let const_literal_start = source.find("42").expect("literal should exist");
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::const::lit".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Literal,
            type_category: TypeCategory::Integer,
            type_repr: "u32".to_string(),
            span: Span::new(
                "src/main.nr",
                u32::try_from(const_literal_start).unwrap_or(u32::MAX),
                u32::try_from(const_literal_start + 2).unwrap_or(u32::MAX),
                1,
                18,
            ),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn fallback_ignores_multiline_const_declaration() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "const FEE: u128 =\n    42;\nfn main() { let one = 1; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_large_literals_even_when_parse_overflows() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let fee = 1844674407370955161600; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn incomplete_semantic_model_falls_back_to_text_for_magic_numbers() {
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
            span: Span::new("src/main.nr", 20, 23, 1, 1),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let fee = 42; }".to_string(),
            )],
        );
        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }
}
