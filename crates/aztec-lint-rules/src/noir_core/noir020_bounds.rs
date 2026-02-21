use std::collections::{BTreeMap, BTreeSet};

use aztec_lint_core::diagnostics::{Diagnostic, normalize_file_path};
use aztec_lint_core::model::{ExpressionCategory, GuardKind, SemanticStatement};
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::engine::context::{RuleContext, SourceFile};
use crate::noir_core::util::{
    collect_identifiers, extract_identifiers, extract_index_access_parts, source_slice,
};

pub struct Noir020BoundsRule;

impl Rule for Noir020BoundsRule {
    fn id(&self) -> &'static str {
        "NOIR020"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        if semantic_available(ctx) {
            self.run_semantic(ctx, out);
            return;
        }
        self.run_text_fallback(ctx, out);
    }
}

impl Noir020BoundsRule {
    fn run_semantic(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let semantic = ctx.semantic_model();
        let files = file_map(ctx.files());
        let expressions_by_id = semantic
            .expressions
            .iter()
            .map(|expression| (expression.expr_id.clone(), expression))
            .collect::<BTreeMap<_, _>>();
        let statement_block_map = statement_block_map(semantic);
        let mut dominators_by_function =
            BTreeMap::<String, BTreeMap<String, BTreeSet<String>>>::new();
        for function in &semantic.functions {
            dominators_by_function.insert(
                function.symbol_id.clone(),
                cfg_dominators(semantic, &function.symbol_id),
            );
        }

        let mut guards_by_function = BTreeMap::<String, Vec<GuardInfo>>::new();
        for guard in &semantic.guard_nodes {
            if !matches!(
                guard.kind,
                GuardKind::Assert | GuardKind::Constrain | GuardKind::Range
            ) {
                continue;
            }
            let (guard_span, guard_source_file) =
                if let Some(guarded_expr_id) = &guard.guarded_expr_id {
                    let Some(guarded_expr) = expressions_by_id.get(guarded_expr_id) else {
                        continue;
                    };
                    (guarded_expr.span.clone(), guarded_expr.span.file.clone())
                } else {
                    (guard.span.clone(), guard.span.file.clone())
                };
            let normalized_file = normalize_file_path(&guard_source_file);
            let Some(file) = files.get(&normalized_file).copied() else {
                continue;
            };
            let Some(source) = source_slice(file.text(), guard_span.start, guard_span.end) else {
                continue;
            };
            let identifiers = collect_identifiers(source)
                .into_iter()
                .filter(|identifier| !matches!(identifier.as_str(), "assert" | "len" | "constrain"))
                .collect::<BTreeSet<_>>();
            if identifiers.is_empty() {
                continue;
            }
            let guard_statement_id =
                innermost_statement(semantic, &guard.function_symbol_id, &guard.span)
                    .map(|statement| statement.stmt_id.clone());
            let guard_block_id = guard_statement_id
                .as_ref()
                .and_then(|statement_id| {
                    statement_block_map
                        .get(&(guard.function_symbol_id.clone(), statement_id.clone()))
                })
                .cloned();
            guards_by_function
                .entry(guard.function_symbol_id.clone())
                .or_default()
                .push(GuardInfo {
                    span_start: guard.span.start,
                    identifiers,
                    block_id: guard_block_id,
                });
        }

        for guards in guards_by_function.values_mut() {
            guards.sort_by_key(|guard| guard.span_start);
        }

        for expression in semantic
            .expressions
            .iter()
            .filter(|expression| expression.category == ExpressionCategory::Index)
        {
            let normalized_file = normalize_file_path(&expression.span.file);
            let Some(file) = files.get(&normalized_file).copied() else {
                continue;
            };
            let Some(source) =
                source_slice(file.text(), expression.span.start, expression.span.end)
            else {
                continue;
            };
            let Some((base_name, index_name, relative_start)) = extract_index_access_parts(source)
            else {
                continue;
            };

            let index_statement_id =
                innermost_statement(semantic, &expression.function_symbol_id, &expression.span)
                    .map(|statement| statement.stmt_id.clone());
            let index_block_id = index_statement_id
                .as_ref()
                .and_then(|statement_id| {
                    statement_block_map
                        .get(&(expression.function_symbol_id.clone(), statement_id.clone()))
                })
                .cloned();
            let function_dominators = dominators_by_function
                .get(&expression.function_symbol_id)
                .cloned()
                .unwrap_or_default();

            let is_guarded = guards_by_function
                .get(&expression.function_symbol_id)
                .is_some_and(|guards| {
                    guards.iter().any(|guard| {
                        guard.identifiers.contains(&index_name)
                            && guard.identifiers.contains(&base_name)
                            && guard_applies_to_index(
                                guard,
                                expression.span.start,
                                index_block_id.as_deref(),
                                &function_dominators,
                            )
                    })
                });
            if is_guarded {
                continue;
            }

            let Some(expression_start) = usize::try_from(expression.span.start).ok() else {
                continue;
            };
            let start = expression_start.saturating_add(relative_start);
            out.push(ctx.diagnostic(
                self.id(),
                CORRECTNESS,
                format!("index `{index_name}` is used without an obvious bounds assertion"),
                file.span_for_range(start, start + index_name.len()),
            ));
        }
    }

    fn run_text_fallback(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            let guarded_indices = collect_guarded_indices(file.text());
            let mut offset = 0usize;

            for line in file.text().lines() {
                for (index_name, column) in indexed_accesses(line) {
                    if guarded_indices.contains(&index_name) {
                        continue;
                    }
                    let start = offset + column;
                    out.push(ctx.diagnostic(
                        self.id(),
                        CORRECTNESS,
                        format!("index `{index_name}` is used without an obvious bounds assertion"),
                        file.span_for_range(start, start + index_name.len()),
                    ));
                }

                offset += line.len() + 1;
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GuardInfo {
    span_start: u32,
    identifiers: BTreeSet<String>,
    block_id: Option<String>,
}

fn guard_applies_to_index(
    guard: &GuardInfo,
    index_span_start: u32,
    index_block_id: Option<&str>,
    dominators: &BTreeMap<String, BTreeSet<String>>,
) -> bool {
    let Some(index_block_id) = index_block_id else {
        return guard.span_start < index_span_start;
    };
    let Some(guard_block_id) = guard.block_id.as_deref() else {
        return guard.span_start < index_span_start;
    };
    if guard_block_id == index_block_id {
        return guard.span_start < index_span_start;
    }
    dominators
        .get(index_block_id)
        .is_some_and(|dominators| dominators.contains(guard_block_id))
}

fn statement_block_map(
    semantic: &aztec_lint_core::model::SemanticModel,
) -> BTreeMap<(String, String), String> {
    let mut out = BTreeMap::<(String, String), String>::new();
    for block in &semantic.cfg_blocks {
        for statement_id in &block.statement_ids {
            out.insert(
                (block.function_symbol_id.clone(), statement_id.clone()),
                block.block_id.clone(),
            );
        }
    }
    out
}

fn innermost_statement<'a>(
    semantic: &'a aztec_lint_core::model::SemanticModel,
    function_symbol_id: &str,
    span: &aztec_lint_core::model::Span,
) -> Option<&'a SemanticStatement> {
    let normalized_file = normalize_file_path(&span.file);
    semantic
        .statements
        .iter()
        .filter(|statement| statement.function_symbol_id == function_symbol_id)
        .filter(|statement| normalize_file_path(&statement.span.file) == normalized_file)
        .filter(|statement| statement.span.start <= span.start && span.end <= statement.span.end)
        .min_by_key(|statement| statement.span.end.saturating_sub(statement.span.start))
}

fn cfg_dominators(
    semantic: &aztec_lint_core::model::SemanticModel,
    function_symbol_id: &str,
) -> BTreeMap<String, BTreeSet<String>> {
    let blocks = semantic
        .cfg_blocks
        .iter()
        .filter(|block| block.function_symbol_id == function_symbol_id)
        .map(|block| block.block_id.clone())
        .collect::<BTreeSet<_>>();
    if blocks.is_empty() {
        return BTreeMap::new();
    }

    let mut predecessors = BTreeMap::<String, BTreeSet<String>>::new();
    for block in &blocks {
        predecessors.entry(block.clone()).or_default();
    }
    for edge in semantic
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
                .is_none_or(|preds| preds.is_empty())
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
            let preds = predecessors.get(block_id).cloned().unwrap_or_default();
            if preds.is_empty() {
                let singleton = BTreeSet::from([block_id.clone()]);
                if dominators.get(block_id) != Some(&singleton) {
                    dominators.insert(block_id.clone(), singleton);
                    changed = true;
                }
                continue;
            }

            let mut pred_iter = preds.into_iter();
            let Some(first_pred) = pred_iter.next() else {
                continue;
            };
            let mut next = dominators.get(&first_pred).cloned().unwrap_or_default();
            for pred in pred_iter {
                let pred_doms = dominators.get(&pred).cloned().unwrap_or_default();
                next = next
                    .intersection(&pred_doms)
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

fn semantic_available(ctx: &RuleContext<'_>) -> bool {
    let semantic = ctx.semantic_model();
    !semantic.expressions.is_empty()
}

fn file_map(files: &[SourceFile]) -> BTreeMap<String, &SourceFile> {
    files
        .iter()
        .map(|file| (normalize_file_path(file.path()), file))
        .collect::<BTreeMap<_, _>>()
}

fn collect_guarded_indices(source: &str) -> BTreeSet<String> {
    let mut guarded = BTreeSet::<String>::new();

    for line in source.lines() {
        if !(line.contains("assert(")
            && line.contains("len()")
            && (line.contains('<') || line.contains("<=")))
        {
            continue;
        }

        for identifier in collect_identifiers(line) {
            if matches!(identifier.as_str(), "assert" | "len") {
                continue;
            }
            guarded.insert(identifier);
        }
    }

    guarded
}

fn indexed_accesses(line: &str) -> Vec<(String, usize)> {
    let mut out = Vec::<(String, usize)>::new();
    let bytes = line.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] != b'[' {
            index += 1;
            continue;
        }
        let mut left = index;
        while left > 0 && bytes[left - 1].is_ascii_whitespace() {
            left -= 1;
        }
        if left == 0
            || !(bytes[left - 1].is_ascii_alphanumeric()
                || bytes[left - 1] == b'_'
                || bytes[left - 1] == b')')
        {
            index += 1;
            continue;
        }
        let Some(close_rel) = line[index + 1..].find(']') else {
            break;
        };
        let close = index + 1 + close_rel;
        let expr = line[index + 1..close].trim();

        if expr.is_empty() || expr.chars().all(|ch| ch.is_ascii_digit()) {
            index = close + 1;
            continue;
        }

        let identifiers = extract_identifiers(expr);
        if identifiers.len() == 1 {
            out.push((identifiers[0].0.clone(), index + 1 + identifiers[0].1));
        }

        index = close + 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::{
        ExpressionCategory, GuardKind, GuardNode, ProjectModel, SemanticExpression,
        SemanticFunction, Span, TypeCategory,
    };

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir020BoundsRule;

    #[test]
    fn reports_unbounded_indexing() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main(arr: [Field; 4], idx: u32) { let x = arr[idx]; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir020BoundsRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_indexing_with_asserted_guard() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main(arr: [Field; 4], idx: u32) { assert(idx < arr.len()); let x = arr[idx]; }"
                    .to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir020BoundsRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_reports_unbounded_indexing() {
        let source = "fn main(arr: [Field; 4], idx: u32) { let value = arr[idx]; }";
        let (function_start, function_end) = span_range(
            source,
            "fn main(arr: [Field; 4], idx: u32) { let value = arr[idx]; }",
        );
        let (index_start, index_end) = span_range(source, "arr[idx]");

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
            expr_id: "expr::index".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Index,
            type_category: TypeCategory::Field,
            type_repr: "Field".to_string(),
            span: Span::new("src/main.nr", index_start, index_end, 1, 1),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );

        let mut diagnostics = Vec::new();
        Noir020BoundsRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn semantic_guarded_index_is_ignored() {
        let source =
            "fn main(arr: [Field; 4], idx: u32) { assert(idx < arr.len()); let value = arr[idx]; }";
        let (function_start, function_end) = span_range(
            source,
            "fn main(arr: [Field; 4], idx: u32) { assert(idx < arr.len()); let value = arr[idx]; }",
        );
        let (guard_start, guard_end) = span_range(source, "idx < arr.len()");
        let (index_start, index_end) = span_range(source, "arr[idx]");

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
            expr_id: "expr::guard".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::BinaryOp,
            type_category: TypeCategory::Bool,
            type_repr: "bool".to_string(),
            span: Span::new("src/main.nr", guard_start, guard_end, 1, 1),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::index".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Index,
            type_category: TypeCategory::Field,
            type_repr: "Field".to_string(),
            span: Span::new("src/main.nr", index_start, index_end, 1, 1),
        });
        project.semantic.guard_nodes.push(GuardNode {
            guard_id: "guard::assert::1".to_string(),
            function_symbol_id: "fn::main".to_string(),
            kind: GuardKind::Assert,
            guarded_expr_id: Some("expr::guard".to_string()),
            span: Span::new(
                "src/main.nr",
                guard_start.saturating_sub(7),
                guard_end + 1,
                1,
                1,
            ),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );

        let mut diagnostics = Vec::new();
        Noir020BoundsRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_guard_after_index_does_not_suppress() {
        let source =
            "fn main(arr: [Field; 4], idx: u32) { let value = arr[idx]; assert(idx < arr.len()); }";
        let (function_start, function_end) = span_range(
            source,
            "fn main(arr: [Field; 4], idx: u32) { let value = arr[idx]; assert(idx < arr.len()); }",
        );
        let (guard_start, guard_end) = span_range(source, "idx < arr.len()");
        let (index_start, index_end) = span_range(source, "arr[idx]");

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
            expr_id: "expr::guard".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::BinaryOp,
            type_category: TypeCategory::Bool,
            type_repr: "bool".to_string(),
            span: Span::new("src/main.nr", guard_start, guard_end, 1, 1),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::index".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Index,
            type_category: TypeCategory::Field,
            type_repr: "Field".to_string(),
            span: Span::new("src/main.nr", index_start, index_end, 1, 1),
        });
        project.semantic.guard_nodes.push(GuardNode {
            guard_id: "guard::assert::1".to_string(),
            function_symbol_id: "fn::main".to_string(),
            kind: GuardKind::Assert,
            guarded_expr_id: Some("expr::guard".to_string()),
            span: Span::new(
                "src/main.nr",
                guard_start.saturating_sub(7),
                guard_end + 1,
                1,
                1,
            ),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );

        let mut diagnostics = Vec::new();
        Noir020BoundsRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn semantic_guard_for_other_collection_does_not_suppress() {
        let source = "fn main(arr: [Field; 4], other: [Field; 4], idx: u32) { assert(idx < other.len()); let value = arr[idx]; }";
        let (function_start, function_end) = span_range(
            source,
            "fn main(arr: [Field; 4], other: [Field; 4], idx: u32) { assert(idx < other.len()); let value = arr[idx]; }",
        );
        let (guard_start, guard_end) = span_range(source, "idx < other.len()");
        let (index_start, index_end) = span_range(source, "arr[idx]");

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
            expr_id: "expr::guard".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::BinaryOp,
            type_category: TypeCategory::Bool,
            type_repr: "bool".to_string(),
            span: Span::new("src/main.nr", guard_start, guard_end, 1, 1),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::index".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Index,
            type_category: TypeCategory::Field,
            type_repr: "Field".to_string(),
            span: Span::new("src/main.nr", index_start, index_end, 1, 1),
        });
        project.semantic.guard_nodes.push(GuardNode {
            guard_id: "guard::assert::1".to_string(),
            function_symbol_id: "fn::main".to_string(),
            kind: GuardKind::Assert,
            guarded_expr_id: Some("expr::guard".to_string()),
            span: Span::new(
                "src/main.nr",
                guard_start.saturating_sub(7),
                guard_end + 1,
                1,
                1,
            ),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );

        let mut diagnostics = Vec::new();
        Noir020BoundsRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    fn span_range(source: &str, needle: &str) -> (u32, u32) {
        let start = source.find(needle).expect("needle should exist");
        let end = start + needle.len();
        (
            u32::try_from(start).unwrap_or(u32::MAX),
            u32::try_from(end).unwrap_or(u32::MAX),
        )
    }
}
