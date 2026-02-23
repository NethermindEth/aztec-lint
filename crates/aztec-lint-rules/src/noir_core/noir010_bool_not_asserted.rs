use std::collections::{BTreeMap, BTreeSet, VecDeque};

use aztec_lint_core::diagnostics::{Diagnostic, normalize_file_path};
use aztec_lint_core::model::{GuardKind, StatementCategory, TypeCategory};
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::engine::context::{RuleContext, SourceFile};
use crate::noir_core::util::{
    count_identifier_occurrences, extract_identifiers, is_ident_continue, source_slice,
    text_fallback_line_bindings, text_fallback_statement_bindings,
};

pub struct Noir010BoolNotAssertedRule;

impl Rule for Noir010BoolNotAssertedRule {
    fn id(&self) -> &'static str {
        "NOIR010"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        if semantic_available(ctx) {
            self.run_semantic(ctx, out);
            return;
        }
        self.run_text_fallback(ctx, out);
    }
}

impl Noir010BoolNotAssertedRule {
    fn run_semantic(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let semantic = ctx.semantic_model();
        let files = file_map(ctx.files());

        let bool_expression_ids = semantic
            .expressions
            .iter()
            .filter(|expression| expression.type_category == TypeCategory::Bool)
            .map(|expression| expression.expr_id.clone())
            .collect::<BTreeSet<_>>();

        let mut bool_bindings = Vec::<BoolBinding>::new();
        for statement in semantic
            .statements
            .iter()
            .filter(|statement| statement.category == StatementCategory::Let)
        {
            if !semantic.dfg_edges.iter().any(|edge| {
                edge.function_symbol_id == statement.function_symbol_id
                    && edge.to_node_id == statement.stmt_id
                    && bool_expression_ids.contains(&edge.from_node_id)
            }) {
                continue;
            }

            let normalized_file = normalize_file_path(&statement.span.file);
            let Some(file) = files.get(&normalized_file).copied() else {
                continue;
            };
            let Some(statement_source) =
                source_slice(file.text(), statement.span.start, statement.span.end)
            else {
                continue;
            };
            let Some(statement_start) = usize::try_from(statement.span.start).ok() else {
                continue;
            };

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
            let mut seen_definitions = BTreeSet::new();
            definitions.retain(|definition| seen_definitions.insert(definition.clone()));

            let parsed_bindings = text_fallback_statement_bindings(statement_source);
            for (index, definition_node_id) in definitions.iter().enumerate() {
                let Some((name, relative_offset)) = parsed_bindings.get(index) else {
                    continue;
                };
                bool_bindings.push(BoolBinding {
                    function_symbol_id: statement.function_symbol_id.clone(),
                    definition_node_id: definition_node_id.clone(),
                    file: normalized_file.clone(),
                    name: name.clone(),
                    start: statement_start.saturating_add(*relative_offset),
                });
            }
        }

        let mut adjacency_by_function = BTreeMap::<String, BTreeMap<String, Vec<String>>>::new();
        for edge in &semantic.dfg_edges {
            adjacency_by_function
                .entry(edge.function_symbol_id.clone())
                .or_default()
                .entry(edge.from_node_id.clone())
                .or_default()
                .push(edge.to_node_id.clone());
        }

        let mut assert_targets_by_function = BTreeMap::<String, BTreeSet<String>>::new();
        for guard in &semantic.guard_nodes {
            if guard.kind != GuardKind::Assert {
                continue;
            }
            let Some(guarded_expr_id) = &guard.guarded_expr_id else {
                continue;
            };
            assert_targets_by_function
                .entry(guard.function_symbol_id.clone())
                .or_default()
                .insert(guarded_expr_id.clone());
        }

        let asserted_identifiers_by_file = files
            .iter()
            .map(|(normalized_file, file)| {
                (
                    normalized_file.clone(),
                    assertion_reference_identifiers(file.text()),
                )
            })
            .collect::<BTreeMap<_, _>>();

        for binding in bool_bindings {
            if binding.name.starts_with('_') {
                continue;
            }
            let assert_targets = assert_targets_by_function
                .get(&binding.function_symbol_id)
                .cloned()
                .unwrap_or_default();
            let is_asserted_semantic = adjacency_by_function
                .get(&binding.function_symbol_id)
                .is_some_and(|adjacency| {
                    has_path_to_any(&binding.definition_node_id, &assert_targets, adjacency)
                });
            let is_asserted_text = asserted_identifiers_by_file
                .get(&binding.file)
                .is_some_and(|identifiers| identifiers.contains(&binding.name));
            if is_asserted_semantic || is_asserted_text {
                continue;
            }

            let Some(file) = files.get(&binding.file).copied() else {
                continue;
            };
            let has_meaningful_use_semantic = adjacency_by_function
                .get(&binding.function_symbol_id)
                .is_some_and(|adjacency| {
                    has_meaningful_use(&binding.definition_node_id, adjacency)
                });
            let has_meaningful_use_text =
                count_identifier_occurrences(file.text(), &binding.name) > 1;
            if has_meaningful_use_semantic || has_meaningful_use_text {
                continue;
            }

            out.push(ctx.diagnostic(
                self.id(),
                CORRECTNESS,
                format!("boolean `{}` is computed but never asserted", binding.name),
                file.span_for_range(binding.start, binding.start + binding.name.len()),
            ));
        }
    }

    fn run_text_fallback(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            let mut bool_bindings = Vec::<(String, usize)>::new();
            let mut offset = 0usize;

            for line in file.text().lines() {
                for (name, column) in text_fallback_line_bindings(line) {
                    let Some(rhs) = assignment_rhs(line, &name, column) else {
                        continue;
                    };
                    if looks_boolean_expression(rhs) {
                        bool_bindings.push((name, offset + column));
                    }
                }

                offset += line.len() + 1;
            }

            let asserted = assertion_reference_identifiers(file.text());

            for (name, declaration_offset) in bool_bindings {
                if name.starts_with('_') || asserted.contains(&name) {
                    continue;
                }
                if count_identifier_occurrences(file.text(), &name) > 1 {
                    continue;
                }
                out.push(ctx.diagnostic(
                    self.id(),
                    CORRECTNESS,
                    format!("boolean `{name}` is computed but never asserted"),
                    file.span_for_range(declaration_offset, declaration_offset + name.len()),
                ));
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BoolBinding {
    function_symbol_id: String,
    definition_node_id: String,
    file: String,
    name: String,
    start: usize,
}

fn semantic_available(ctx: &RuleContext<'_>) -> bool {
    let semantic = ctx.semantic_model();
    !semantic.statements.is_empty()
        && !semantic.expressions.is_empty()
        && !semantic.dfg_edges.is_empty()
}

fn file_map(files: &[SourceFile]) -> BTreeMap<String, &SourceFile> {
    files
        .iter()
        .map(|file| (normalize_file_path(file.path()), file))
        .collect::<BTreeMap<_, _>>()
}

fn has_path_to_any(
    start_node_id: &str,
    targets: &BTreeSet<String>,
    adjacency: &BTreeMap<String, Vec<String>>,
) -> bool {
    if targets.is_empty() {
        return false;
    }
    let mut visited = BTreeSet::<String>::new();
    let mut queue = VecDeque::<String>::from([start_node_id.to_string()]);

    while let Some(node_id) = queue.pop_front() {
        if !visited.insert(node_id.clone()) {
            continue;
        }
        if targets.contains(&node_id) {
            return true;
        }
        if let Some(next_nodes) = adjacency.get(&node_id) {
            for next in next_nodes {
                queue.push_back(next.clone());
            }
        }
    }

    false
}

fn has_meaningful_use(definition_node_id: &str, adjacency: &BTreeMap<String, Vec<String>>) -> bool {
    let mut visited = BTreeSet::<String>::new();
    let mut queue = VecDeque::<String>::from([definition_node_id.to_string()]);

    while let Some(node_id) = queue.pop_front() {
        if !visited.insert(node_id.clone()) {
            continue;
        }
        if node_id != definition_node_id
            && (node_id.starts_with("expr::") || node_id.starts_with("stmt::"))
        {
            return true;
        }
        if let Some(next_nodes) = adjacency.get(&node_id) {
            for next in next_nodes {
                queue.push_back(next.clone());
            }
        }
    }

    false
}

fn assertion_reference_identifiers(source: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let bytes = source.as_bytes();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        if !is_ident_start(bytes[cursor])
            || (cursor > 0 && is_ident_continue(bytes[cursor.saturating_sub(1)]))
        {
            cursor += 1;
            continue;
        }

        let call_start = cursor;
        cursor += 1;
        while cursor < bytes.len() && is_ident_continue(bytes[cursor]) {
            cursor += 1;
        }
        let call_end = cursor;
        let call_name = &source[call_start..call_end];

        let mut lookahead = cursor;
        while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
            lookahead += 1;
        }
        if bytes.get(lookahead) != Some(&b'(') {
            continue;
        }
        if is_definition_like_call(source, call_start) || !is_assertion_like_name(call_name) {
            continue;
        }

        let Some(close) = find_matching_paren(bytes, lookahead) else {
            continue;
        };
        let args = &source[lookahead + 1..close];
        for (identifier, _) in extract_identifiers(args) {
            if matches!(
                identifier.as_str(),
                "assert"
                    | "assert_eq"
                    | "fn"
                    | "let"
                    | "mut"
                    | "pub"
                    | "return"
                    | "self"
                    | "super"
                    | "crate"
                    | "true"
                    | "false"
                    | "if"
                    | "else"
                    | "for"
                    | "while"
            ) {
                continue;
            }
            out.insert(identifier);
        }
        cursor = close + 1;
    }

    out
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_assertion_like_name(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    normalized == "assert"
        || normalized == "assert_eq"
        || normalized.starts_with("assert_")
        || normalized.ends_with("_assert")
        || normalized.contains("require")
        || normalized.contains("ensure")
        || normalized.starts_with("check_")
}

fn is_definition_like_call(source: &str, call_start: usize) -> bool {
    if call_start == 0 {
        return false;
    }
    let bytes = source.as_bytes();
    let mut cursor = call_start;
    while cursor > 0 && bytes[cursor - 1].is_ascii_whitespace() {
        cursor -= 1;
    }
    if cursor == 0 {
        return false;
    }

    let mut token_end = cursor;
    while token_end > 0 && is_ident_continue(bytes[token_end - 1]) {
        token_end -= 1;
    }
    if token_end == cursor {
        return false;
    }
    matches!(
        &source[token_end..cursor],
        "fn" | "trait" | "impl" | "mod" | "use" | "pub"
    )
}

fn find_matching_paren(bytes: &[u8], open_paren: usize) -> Option<usize> {
    if bytes.get(open_paren) != Some(&b'(') {
        return None;
    }

    let mut depth = 1usize;
    let mut cursor = open_paren + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'(' => depth += 1,
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(cursor);
                }
            }
            b'"' | b'\'' => {
                cursor = skip_quoted_literal(bytes, cursor);
                continue;
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn skip_quoted_literal(bytes: &[u8], start: usize) -> usize {
    let quote = bytes[start];
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' {
            cursor = cursor.saturating_add(2);
            continue;
        }
        if bytes[cursor] == quote {
            return cursor + 1;
        }
        cursor += 1;
    }
    bytes.len()
}

fn assignment_rhs<'a>(line: &'a str, name: &str, name_column: usize) -> Option<&'a str> {
    let tail = &line[name_column + name.len()..];
    let equals = tail.find('=')?;
    Some(tail[equals + 1..].trim())
}

fn looks_boolean_expression(rhs: &str) -> bool {
    rhs.contains("==")
        || rhs.contains("!=")
        || rhs.contains(">=")
        || rhs.contains("<=")
        || rhs.contains(" > ")
        || rhs.contains(" < ")
        || rhs.contains("true")
        || rhs.contains("false")
        || rhs.trim_start().starts_with('!')
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::{
        DfgEdge, DfgEdgeKind, GuardKind, GuardNode, ProjectModel, SemanticExpression,
        SemanticFunction, SemanticStatement, Span, StatementCategory, TypeCategory,
    };

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir010BoolNotAssertedRule;

    #[test]
    fn reports_bool_value_that_is_not_asserted() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let is_valid = 1 == 2; let x = 5; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir010BoolNotAssertedRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_asserted_bool_value() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let is_valid = 1 == 1; assert(is_valid); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir010BoolNotAssertedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_negated_asserted_bool_value() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let violates_requirement = !(1 > 0); assert(!violates_requirement); }"
                    .to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir010BoolNotAssertedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_asserted_alias_bool_value() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let b = 1 == 1; let b2 = b; assert(b2); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir010BoolNotAssertedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_bool_used_in_assert_conjunction() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let b = 1 == 1; let other = true; assert(b && other); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir010BoolNotAssertedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_bool_with_non_assert_meaningful_use() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let intent_consumed_before: bool = false; let _snapshot = intent_consumed_before; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir010BoolNotAssertedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_assertion_like_wrapper_call() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let violates_requirement = !(1 > 0); require(violates_requirement); }"
                    .to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir010BoolNotAssertedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_bool_binding_without_assert_is_reported() {
        let source = "fn main() { let ready = 1 == 2; }";
        let (function_start, function_end) =
            span_range(source, "fn main() { let ready = 1 == 2; }");
        let (statement_start, statement_end) = span_range(source, "let ready = 1 == 2;");

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
            expr_id: "expr::bool".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: aztec_lint_core::model::ExpressionCategory::BinaryOp,
            type_category: TypeCategory::Bool,
            type_repr: "bool".to_string(),
            span: Span::new(
                "src/main.nr",
                statement_start + 12,
                statement_start + 18,
                1,
                1,
            ),
        });
        project.semantic.statements.push(SemanticStatement {
            stmt_id: "stmt::ready".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: StatementCategory::Let,
            span: Span::new("src/main.nr", statement_start, statement_end, 1, 1),
        });
        project.semantic.dfg_edges.push(DfgEdge {
            function_symbol_id: "fn::main".to_string(),
            from_node_id: "expr::bool".to_string(),
            to_node_id: "stmt::ready".to_string(),
            kind: DfgEdgeKind::DefUse,
        });
        project.semantic.dfg_edges.push(DfgEdge {
            function_symbol_id: "fn::main".to_string(),
            from_node_id: "stmt::ready".to_string(),
            to_node_id: "def::ready".to_string(),
            kind: DfgEdgeKind::DefUse,
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );

        let mut diagnostics = Vec::new();
        Noir010BoolNotAssertedRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("never asserted"));
    }

    #[test]
    fn semantic_assert_use_marks_bool_binding_as_safe() {
        let source = "fn main() { let ready = 1 == 1; assert(ready); }";
        let (function_start, function_end) =
            span_range(source, "fn main() { let ready = 1 == 1; assert(ready); }");
        let (statement_start, statement_end) = span_range(source, "let ready = 1 == 1;");

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
            expr_id: "expr::bool".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: aztec_lint_core::model::ExpressionCategory::BinaryOp,
            type_category: TypeCategory::Bool,
            type_repr: "bool".to_string(),
            span: Span::new(
                "src/main.nr",
                statement_start + 12,
                statement_start + 18,
                1,
                1,
            ),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::ready_use".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: aztec_lint_core::model::ExpressionCategory::Identifier,
            type_category: TypeCategory::Bool,
            type_repr: "bool".to_string(),
            span: Span::new("src/main.nr", statement_end + 8, statement_end + 13, 1, 1),
        });
        project.semantic.statements.push(SemanticStatement {
            stmt_id: "stmt::ready".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: StatementCategory::Let,
            span: Span::new("src/main.nr", statement_start, statement_end, 1, 1),
        });
        project.semantic.dfg_edges.push(DfgEdge {
            function_symbol_id: "fn::main".to_string(),
            from_node_id: "expr::bool".to_string(),
            to_node_id: "stmt::ready".to_string(),
            kind: DfgEdgeKind::DefUse,
        });
        project.semantic.dfg_edges.push(DfgEdge {
            function_symbol_id: "fn::main".to_string(),
            from_node_id: "stmt::ready".to_string(),
            to_node_id: "def::ready".to_string(),
            kind: DfgEdgeKind::DefUse,
        });
        project.semantic.dfg_edges.push(DfgEdge {
            function_symbol_id: "fn::main".to_string(),
            from_node_id: "def::ready".to_string(),
            to_node_id: "expr::ready_use".to_string(),
            kind: DfgEdgeKind::UseDef,
        });
        project.semantic.guard_nodes.push(GuardNode {
            guard_id: "guard::assert::1".to_string(),
            function_symbol_id: "fn::main".to_string(),
            kind: GuardKind::Assert,
            guarded_expr_id: Some("expr::ready_use".to_string()),
            span: Span::new("src/main.nr", statement_end + 1, statement_end + 15, 1, 1),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );

        let mut diagnostics = Vec::new();
        Noir010BoolNotAssertedRule.run(&context, &mut diagnostics);

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
