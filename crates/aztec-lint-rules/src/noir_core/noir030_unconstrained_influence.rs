use std::collections::{BTreeMap, BTreeSet, VecDeque};

use aztec_lint_core::diagnostics::{Diagnostic, normalize_file_path};
use aztec_lint_core::model::{GuardKind, StatementCategory};
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::engine::context::{RuleContext, SourceFile};
use crate::noir_core::util::{
    count_identifier_occurrences, extract_identifiers, source_slice, text_fallback_line_bindings,
    text_fallback_statement_bindings,
};

pub struct Noir030UnconstrainedInfluenceRule;

impl Rule for Noir030UnconstrainedInfluenceRule {
    fn id(&self) -> &'static str {
        "NOIR030"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        if semantic_available(ctx) {
            self.run_semantic(ctx, out);
            return;
        }
        self.run_text_fallback(ctx, out);
    }
}

impl Noir030UnconstrainedInfluenceRule {
    fn run_semantic(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let semantic = ctx.semantic_model();
        let files = file_map(ctx.files());

        let unconstrained_function_ids = semantic
            .functions
            .iter()
            .filter(|function| function.is_unconstrained)
            .map(|function| function.symbol_id.clone())
            .collect::<BTreeSet<_>>();
        if unconstrained_function_ids.is_empty() {
            return;
        }

        let mut call_seeds_by_function = BTreeMap::<String, BTreeSet<String>>::new();
        for call_site in semantic
            .call_sites
            .iter()
            .filter(|call_site| unconstrained_function_ids.contains(&call_site.callee_symbol_id))
        {
            call_seeds_by_function
                .entry(call_site.function_symbol_id.clone())
                .or_default()
                .insert(call_site.expr_id.clone());
        }

        let mut adjacency_by_function = BTreeMap::<String, BTreeMap<String, Vec<String>>>::new();
        let mut reverse_adjacency_by_function =
            BTreeMap::<String, BTreeMap<String, Vec<String>>>::new();
        for edge in &semantic.dfg_edges {
            adjacency_by_function
                .entry(edge.function_symbol_id.clone())
                .or_default()
                .entry(edge.from_node_id.clone())
                .or_default()
                .push(edge.to_node_id.clone());
            reverse_adjacency_by_function
                .entry(edge.function_symbol_id.clone())
                .or_default()
                .entry(edge.to_node_id.clone())
                .or_default()
                .push(edge.from_node_id.clone());
        }

        for function in &semantic.functions {
            let seeds = call_seeds_by_function
                .get(&function.symbol_id)
                .cloned()
                .unwrap_or_default();
            if seeds.is_empty() {
                continue;
            }

            let mut sink_targets = semantic
                .statements
                .iter()
                .filter(|statement| statement.function_symbol_id == function.symbol_id)
                .filter(|statement| {
                    matches!(
                        statement.category,
                        StatementCategory::Assert | StatementCategory::Constrain
                    )
                })
                .map(|statement| statement.stmt_id.clone())
                .collect::<BTreeSet<_>>();
            sink_targets.extend(
                semantic
                    .guard_nodes
                    .iter()
                    .filter(|guard| guard.function_symbol_id == function.symbol_id)
                    .filter(|guard| matches!(guard.kind, GuardKind::Assert | GuardKind::Constrain))
                    .filter_map(|guard| guard.guarded_expr_id.clone()),
            );
            if sink_targets.is_empty() {
                continue;
            }

            let adjacency = adjacency_by_function
                .get(&function.symbol_id)
                .cloned()
                .unwrap_or_default();
            let reverse = reverse_adjacency_by_function
                .get(&function.symbol_id)
                .cloned()
                .unwrap_or_default();

            let reachable_from_seeds = bfs(&seeds, &adjacency);
            if reachable_from_seeds.is_disjoint(&sink_targets) {
                continue;
            }
            let nodes_reaching_sinks = bfs(&sink_targets, &reverse);

            let normalized_file = normalize_file_path(&function.span.file);
            let Some(file) = files.get(&normalized_file).copied() else {
                continue;
            };
            let bindings = function_bindings(semantic, &function.symbol_id, file);
            let mut impacted = bindings
                .into_iter()
                .filter(|binding| {
                    reachable_from_seeds.contains(&binding.definition_node_id)
                        && nodes_reaching_sinks.contains(&binding.definition_node_id)
                        && !binding.name.starts_with('_')
                })
                .collect::<Vec<_>>();
            impacted.sort_by_key(|binding| (binding.start, binding.name.clone()));
            impacted.dedup_by(|left, right| {
                left.definition_node_id == right.definition_node_id
                    || (left.name == right.name && left.start == right.start)
            });

            if impacted.is_empty() {
                if let Some(sink_span) = first_sink_span(semantic, &function.symbol_id) {
                    out.push(ctx.diagnostic(
                        self.id(),
                        CORRECTNESS,
                        "unconstrained value influences constrained logic".to_string(),
                        file.span_for_range(
                            usize::try_from(sink_span.start).unwrap_or_default(),
                            usize::try_from(sink_span.end).unwrap_or_default(),
                        ),
                    ));
                }
                continue;
            }

            for binding in impacted {
                out.push(ctx.diagnostic(
                    self.id(),
                    CORRECTNESS,
                    format!(
                        "unconstrained value `{}` influences constrained logic",
                        binding.name
                    ),
                    file.span_for_range(binding.start, binding.start + binding.name.len()),
                ));
            }
        }
    }

    fn run_text_fallback(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            let unconstrained_fns = unconstrained_functions(file.text());
            let mut tainted = BTreeSet::<String>::new();
            let mut offset = 0usize;

            for line in file.text().lines() {
                for (name, column) in text_fallback_line_bindings(line) {
                    let Some(rhs) = assignment_rhs(line, &name, column) else {
                        continue;
                    };
                    let rhs_trimmed = rhs.trim();
                    let influenced = rhs_trimmed.contains("unconstrained")
                        || unconstrained_fns.iter().any(|function_name| {
                            rhs_trimmed.contains(&format!("{function_name}("))
                        });
                    if influenced {
                        tainted.insert(name);
                    }
                }

                if line.contains("assert(") || line.contains("constrain(") {
                    for variable in &tainted {
                        if count_identifier_occurrences(line, variable) > 0 {
                            out.push(ctx.diagnostic(
                                self.id(),
                                CORRECTNESS,
                                format!(
                                    "unconstrained value `{variable}` influences constrained logic"
                                ),
                                file.span_for_range(offset, offset + line.len()),
                            ));
                        }
                    }
                }

                offset += line.len() + 1;
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LocalBinding {
    definition_node_id: String,
    name: String,
    start: usize,
}

fn semantic_available(ctx: &RuleContext<'_>) -> bool {
    let semantic = ctx.semantic_model();
    !semantic.functions.is_empty()
        && !semantic.call_sites.is_empty()
        && !semantic.dfg_edges.is_empty()
}

fn file_map(files: &[SourceFile]) -> BTreeMap<String, &SourceFile> {
    files
        .iter()
        .map(|file| (normalize_file_path(file.path()), file))
        .collect::<BTreeMap<_, _>>()
}

fn bfs(start: &BTreeSet<String>, adjacency: &BTreeMap<String, Vec<String>>) -> BTreeSet<String> {
    let mut visited = BTreeSet::<String>::new();
    let mut queue = VecDeque::<String>::from_iter(start.iter().cloned());

    while let Some(node_id) = queue.pop_front() {
        if !visited.insert(node_id.clone()) {
            continue;
        }
        if let Some(next_nodes) = adjacency.get(&node_id) {
            for next in next_nodes {
                queue.push_back(next.clone());
            }
        }
    }

    visited
}

fn function_bindings(
    semantic: &aztec_lint_core::model::SemanticModel,
    function_symbol_id: &str,
    file: &SourceFile,
) -> Vec<LocalBinding> {
    let mut out = Vec::<LocalBinding>::new();
    let normalized_file = normalize_file_path(file.path());

    for statement in semantic
        .statements
        .iter()
        .filter(|statement| statement.function_symbol_id == function_symbol_id)
        .filter(|statement| statement.category == StatementCategory::Let)
    {
        if normalize_file_path(&statement.span.file) != normalized_file {
            continue;
        }

        let mut definitions = semantic
            .dfg_edges
            .iter()
            .filter(|edge| {
                edge.function_symbol_id == statement.function_symbol_id
                    && edge.from_node_id == statement.stmt_id
                    && edge.to_node_id.starts_with("def::")
            })
            .map(|edge| edge.to_node_id.clone())
            .collect::<Vec<_>>();
        definitions.sort();
        definitions.dedup();

        let Some(statement_source) =
            source_slice(file.text(), statement.span.start, statement.span.end)
        else {
            continue;
        };
        let Some(statement_start) = usize::try_from(statement.span.start).ok() else {
            continue;
        };
        let names = text_fallback_statement_bindings(statement_source);

        for (index, definition_node_id) in definitions.iter().enumerate() {
            let Some((name, relative_start)) = names.get(index) else {
                continue;
            };
            out.push(LocalBinding {
                definition_node_id: definition_node_id.clone(),
                name: name.clone(),
                start: statement_start.saturating_add(*relative_start),
            });
        }
    }

    out
}

fn first_sink_span(
    semantic: &aztec_lint_core::model::SemanticModel,
    function_symbol_id: &str,
) -> Option<aztec_lint_core::model::Span> {
    let statement_sink = semantic
        .statements
        .iter()
        .filter(|statement| statement.function_symbol_id == function_symbol_id)
        .filter(|statement| {
            matches!(
                statement.category,
                StatementCategory::Assert | StatementCategory::Constrain
            )
        })
        .map(|statement| statement.span.clone())
        .min_by_key(|span| (span.file.clone(), span.start));
    let guard_sink = semantic
        .guard_nodes
        .iter()
        .filter(|guard| guard.function_symbol_id == function_symbol_id)
        .filter(|guard| matches!(guard.kind, GuardKind::Assert | GuardKind::Constrain))
        .map(|guard| guard.span.clone())
        .min_by_key(|span| (span.file.clone(), span.start));

    match (statement_sink, guard_sink) {
        (Some(left), Some(right)) => {
            if (left.file.as_str(), left.start) <= (right.file.as_str(), right.start) {
                Some(left)
            } else {
                Some(right)
            }
        }
        (Some(span), None) | (None, Some(span)) => Some(span),
        (None, None) => None,
    }
}

fn unconstrained_functions(source: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();

    for line in source.lines() {
        let Some(marker) = line.find("unconstrained fn ") else {
            continue;
        };
        let tail = &line[marker + "unconstrained fn ".len()..];
        if let Some((name, _)) = extract_identifiers(tail).into_iter().next() {
            out.insert(name);
        }
    }

    out
}

fn assignment_rhs<'a>(line: &'a str, name: &str, name_column: usize) -> Option<&'a str> {
    let tail = &line[name_column + name.len()..];
    let equals = tail.find('=')?;
    Some(tail[equals + 1..].trim())
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::{
        CallSite, DfgEdge, DfgEdgeKind, ExpressionCategory, GuardKind, GuardNode, ProjectModel,
        SemanticExpression, SemanticFunction, SemanticStatement, Span, StatementCategory,
        TypeCategory,
    };

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir030UnconstrainedInfluenceRule;

    #[test]
    fn reports_unconstrained_influence() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                r#"
unconstrained fn read_secret() -> Field { 7 }
fn main() {
    let secret = read_secret();
    assert(secret == 7);
}
"#
                .to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir030UnconstrainedInfluenceRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_constrained_values() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let value = 7; assert(value == 7); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir030UnconstrainedInfluenceRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_unconstrained_flow_into_assert_is_reported() {
        let source = "unconstrained fn read_secret() -> Field { 7 }\nfn main() { let secret = read_secret(); assert(secret == 7); }";
        let (unconstrained_start, unconstrained_end) =
            span_range(source, "unconstrained fn read_secret() -> Field { 7 }");
        let (main_start, main_end) = span_range(
            source,
            "fn main() { let secret = read_secret(); assert(secret == 7); }",
        );
        let (let_start, let_end) = span_range(source, "let secret = read_secret();");
        let (assert_expr_start, assert_expr_end) = span_range(source, "secret == 7");

        let mut project = ProjectModel::default();
        project.semantic.functions.push(SemanticFunction {
            symbol_id: "fn::read_secret".to_string(),
            name: "read_secret".to_string(),
            module_symbol_id: "module::main".to_string(),
            return_type_repr: "Field".to_string(),
            return_type_category: TypeCategory::Field,
            parameter_types: Vec::new(),
            is_entrypoint: false,
            is_unconstrained: true,
            span: Span::new("src/main.nr", unconstrained_start, unconstrained_end, 1, 1),
        });
        project.semantic.functions.push(SemanticFunction {
            symbol_id: "fn::main".to_string(),
            name: "main".to_string(),
            module_symbol_id: "module::main".to_string(),
            return_type_repr: "()".to_string(),
            return_type_category: TypeCategory::Unknown,
            parameter_types: Vec::new(),
            is_entrypoint: true,
            is_unconstrained: false,
            span: Span::new("src/main.nr", main_start, main_end, 2, 1),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::call_secret".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Call,
            type_category: TypeCategory::Field,
            type_repr: "Field".to_string(),
            span: Span::new("src/main.nr", let_start + 13, let_start + 26, 2, 1),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::assert_guard".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::BinaryOp,
            type_category: TypeCategory::Bool,
            type_repr: "bool".to_string(),
            span: Span::new("src/main.nr", assert_expr_start, assert_expr_end, 2, 1),
        });
        project.semantic.statements.push(SemanticStatement {
            stmt_id: "stmt::let_secret".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: StatementCategory::Let,
            span: Span::new("src/main.nr", let_start, let_end, 2, 1),
        });
        project.semantic.dfg_edges.push(DfgEdge {
            function_symbol_id: "fn::main".to_string(),
            from_node_id: "expr::call_secret".to_string(),
            to_node_id: "stmt::let_secret".to_string(),
            kind: DfgEdgeKind::DefUse,
        });
        project.semantic.dfg_edges.push(DfgEdge {
            function_symbol_id: "fn::main".to_string(),
            from_node_id: "stmt::let_secret".to_string(),
            to_node_id: "def::secret".to_string(),
            kind: DfgEdgeKind::DefUse,
        });
        project.semantic.dfg_edges.push(DfgEdge {
            function_symbol_id: "fn::main".to_string(),
            from_node_id: "def::secret".to_string(),
            to_node_id: "expr::assert_guard".to_string(),
            kind: DfgEdgeKind::UseDef,
        });
        project.semantic.call_sites.push(CallSite {
            call_site_id: "call::1".to_string(),
            function_symbol_id: "fn::main".to_string(),
            callee_symbol_id: "fn::read_secret".to_string(),
            expr_id: "expr::call_secret".to_string(),
            span: Span::new("src/main.nr", let_start + 13, let_start + 26, 2, 1),
        });
        project.semantic.guard_nodes.push(GuardNode {
            guard_id: "guard::assert::1".to_string(),
            function_symbol_id: "fn::main".to_string(),
            kind: GuardKind::Assert,
            guarded_expr_id: Some("expr::assert_guard".to_string()),
            span: Span::new(
                "src/main.nr",
                assert_expr_start.saturating_sub(7),
                assert_expr_end + 1,
                2,
                1,
            ),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );

        let mut diagnostics = Vec::new();
        Noir030UnconstrainedInfluenceRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("secret"));
    }

    #[test]
    fn semantic_no_unconstrained_call_produces_no_diagnostic() {
        let source = "fn main() { let value = 7; assert(value == 7); }";
        let (main_start, main_end) =
            span_range(source, "fn main() { let value = 7; assert(value == 7); }");

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
            span: Span::new("src/main.nr", main_start, main_end, 1, 1),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir030UnconstrainedInfluenceRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
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
