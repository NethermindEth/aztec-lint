use std::collections::{BTreeMap, BTreeSet};

use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::diagnostics::normalize_file_path;
use aztec_lint_core::model::CfgEdgeKind;
use aztec_lint_core::policy::MAINTAINABILITY;

use crate::Rule;
use crate::engine::context::{RuleContext, SourceFile};
use crate::noir_core::util::text_fallback_function_scopes;

pub struct Noir110ComplexityRule;

const COMPLEXITY_LIMIT: usize = 6;

impl Rule for Noir110ComplexityRule {
    fn id(&self) -> &'static str {
        "NOIR110"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        if semantic_available(ctx) {
            self.run_semantic(ctx, out);
            return;
        }
        self.run_text_fallback(ctx, out);
    }
}

impl Noir110ComplexityRule {
    fn run_semantic(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let semantic = ctx.semantic_model();
        let files = file_map(ctx.files());

        let mut decision_blocks_by_function = BTreeMap::<String, BTreeSet<String>>::new();
        for edge in semantic.cfg_edges.iter().filter(|edge| {
            matches!(
                edge.kind,
                CfgEdgeKind::TrueBranch | CfgEdgeKind::FalseBranch | CfgEdgeKind::LoopBack
            )
        }) {
            decision_blocks_by_function
                .entry(edge.function_symbol_id.clone())
                .or_default()
                .insert(edge.from_block_id.clone());
        }

        for function in &semantic.functions {
            let complexity = decision_blocks_by_function
                .get(&function.symbol_id)
                .map_or(0usize, BTreeSet::len);
            if complexity <= COMPLEXITY_LIMIT {
                continue;
            }

            let normalized_file = normalize_file_path(&function.span.file);
            let Some(file) = files.get(&normalized_file).copied() else {
                continue;
            };
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
                    "function `{}` complexity is {complexity} (limit: {COMPLEXITY_LIMIT})",
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
                let complexity = compute_complexity_score(body);
                if complexity <= COMPLEXITY_LIMIT {
                    continue;
                }

                out.push(ctx.diagnostic(
                    self.id(),
                    MAINTAINABILITY,
                    format!(
                        "function `{}` complexity is {complexity} (limit: {COMPLEXITY_LIMIT})",
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
    !ctx.semantic_model().functions.is_empty() && !ctx.semantic_model().cfg_edges.is_empty()
}

fn file_map(files: &[SourceFile]) -> BTreeMap<String, &SourceFile> {
    files
        .iter()
        .map(|file| (normalize_file_path(file.path()), file))
        .collect::<BTreeMap<_, _>>()
}

fn compute_complexity_score(body: &str) -> usize {
    body.matches("if ").count()
        + body.matches("for ").count()
        + body.matches("while ").count()
        + body.matches("match ").count()
        + body.matches("&&").count()
        + body.matches("||").count()
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::{
        CfgEdge, CfgEdgeKind, ProjectModel, SemanticFunction, Span, TypeCategory,
    };

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir110ComplexityRule;

    #[test]
    fn reports_complex_function() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                r#"
fn main(x: Field) {
    if x > 1 { }
    if x > 2 { }
    if x > 3 { }
    if x > 4 { }
    if x > 5 { }
    if x > 6 { }
    if x > 7 { }
}
"#
                .to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir110ComplexityRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_simple_function() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let x = 1; assert(x == 1); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir110ComplexityRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_cfg_reports_complex_function() {
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
        for idx in 0..7usize {
            project.semantic.cfg_edges.push(CfgEdge {
                function_symbol_id: "fn::main".to_string(),
                from_block_id: format!("bb{idx}"),
                to_block_id: format!("bb{}", idx + 1),
                kind: CfgEdgeKind::TrueBranch,
            });
            project.semantic.cfg_edges.push(CfgEdge {
                function_symbol_id: "fn::main".to_string(),
                from_block_id: format!("bb{idx}"),
                to_block_id: format!("bb{}", idx + 2),
                kind: CfgEdgeKind::FalseBranch,
            });
        }
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { if true {} }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir110ComplexityRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn semantic_cfg_ignores_simple_function() {
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
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let x = 1; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir110ComplexityRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn incomplete_semantic_model_falls_back_to_text_for_complexity() {
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
            span: Span::new("src/main.nr", 4, 8, 1, 1),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                r#"
fn main(x: Field) {
    if x > 1 { }
    if x > 2 { }
    if x > 3 { }
    if x > 4 { }
    if x > 5 { }
    if x > 6 { }
    if x > 7 { }
}
"#
                .to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir110ComplexityRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }
}
