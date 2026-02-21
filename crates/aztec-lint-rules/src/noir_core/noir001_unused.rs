use std::collections::{BTreeMap, BTreeSet};

use aztec_lint_core::diagnostics::{Applicability, Diagnostic, normalize_file_path};
use aztec_lint_core::model::{ExpressionCategory, StatementCategory, SymbolKind};
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::engine::context::{RuleContext, SourceFile};
use crate::noir_core::util::{
    count_identifier_occurrences, extract_identifiers, find_let_bindings,
    find_let_bindings_in_statement, is_ident_continue,
};

pub struct Noir001UnusedRule;

impl Rule for Noir001UnusedRule {
    fn id(&self) -> &'static str {
        "NOIR001"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        if semantic_available(ctx) {
            self.run_semantic(ctx, out);
            return;
        }
        self.run_text_fallback(ctx, out);
    }
}

impl Noir001UnusedRule {
    fn run_semantic(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let semantic = ctx.semantic_model();
        let files = file_map(ctx.files());

        let let_statement_spans = semantic
            .statements
            .iter()
            .filter(|statement| statement.category == StatementCategory::Let)
            .map(|statement| (statement.stmt_id.clone(), statement.span.clone()))
            .collect::<BTreeMap<_, _>>();

        let mut definitions_by_statement = BTreeMap::<String, Vec<String>>::new();
        for edge in &semantic.dfg_edges {
            if !edge.from_node_id.starts_with("stmt::")
                || !edge.to_node_id.starts_with("def::")
                || !let_statement_spans.contains_key(&edge.from_node_id)
            {
                continue;
            }
            definitions_by_statement
                .entry(edge.from_node_id.clone())
                .or_default()
                .push(edge.to_node_id.clone());
        }
        for definitions in definitions_by_statement.values_mut() {
            definitions.sort();
            definitions.dedup();
        }

        let used_definitions = semantic
            .dfg_edges
            .iter()
            .filter(|edge| {
                edge.from_node_id.starts_with("def::")
                    && (edge.to_node_id.starts_with("expr::")
                        || edge.to_node_id.starts_with("stmt::"))
            })
            .map(|edge| edge.from_node_id.clone())
            .collect::<BTreeSet<_>>();

        let mut seen = BTreeSet::<(String, usize)>::new();
        for (statement_id, definitions) in &definitions_by_statement {
            let Some(span) = let_statement_spans.get(statement_id) else {
                continue;
            };
            let normalized_file = normalize_file_path(&span.file);
            let Some(file) = files.get(&normalized_file).copied() else {
                continue;
            };
            let Some(statement_source) = source_slice(file.text(), span.start, span.end) else {
                continue;
            };

            let bindings = find_let_bindings_in_statement(statement_source);
            let Some(statement_start) = usize::try_from(span.start).ok() else {
                continue;
            };
            for (index, definition_id) in definitions.iter().enumerate() {
                let Some((name, relative_start)) = bindings.get(index) else {
                    continue;
                };
                if name.starts_with('_') || used_definitions.contains(definition_id) {
                    continue;
                }

                let declaration_offset = statement_start.saturating_add(*relative_start);
                if !seen.insert((name.clone(), declaration_offset)) {
                    continue;
                }
                let local_span =
                    file.span_for_range(declaration_offset, declaration_offset + name.len());
                out.push(
                    ctx.diagnostic(
                        self.id(),
                        CORRECTNESS,
                        format!("`{name}` is declared but never used"),
                        local_span.clone(),
                    )
                    .help(
                        "prefix intentionally unused local bindings with `_` to silence this warning",
                    )
                    .span_suggestion(
                        local_span,
                        format!("prefix `{name}` with `_`"),
                        format!("_{name}"),
                        Applicability::MachineApplicable,
                    ),
                );
            }
        }

        let mut identifiers_by_file = BTreeMap::<String, BTreeSet<String>>::new();
        for expression in &semantic.expressions {
            if expression.category != ExpressionCategory::Identifier {
                continue;
            }
            let normalized_file = normalize_file_path(&expression.span.file);
            let Some(file) = files.get(&normalized_file).copied() else {
                continue;
            };
            let Some(ident) = identifier_at_span(file, expression.span.start, expression.span.end)
            else {
                continue;
            };
            identifiers_by_file
                .entry(normalized_file)
                .or_default()
                .insert(ident);
        }

        for import_symbol in ctx
            .project()
            .symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Import)
        {
            let normalized_file = normalize_file_path(&import_symbol.span.file);
            let Some(file) = files.get(&normalized_file).copied() else {
                continue;
            };
            let Some(import_source) = source_slice(
                file.text(),
                import_symbol.span.start,
                import_symbol.span.end,
            ) else {
                continue;
            };

            let imported_bindings = import_bindings_in_use_statement(import_source);
            let Some(import_start) = usize::try_from(import_symbol.span.start).ok() else {
                continue;
            };
            for (name, relative_start) in imported_bindings {
                if name.starts_with('_')
                    || identifiers_by_file
                        .get(&normalized_file)
                        .is_some_and(|identifiers| identifiers.contains(&name))
                {
                    continue;
                }

                let declaration_offset = import_start.saturating_add(relative_start);
                if !seen.insert((name.clone(), declaration_offset)) {
                    continue;
                }
                out.push(
                    ctx.diagnostic(
                        self.id(),
                        CORRECTNESS,
                        format!("import `{name}` is never used"),
                        file.span_for_range(declaration_offset, declaration_offset + name.len()),
                    )
                    .note(
                        "no automatic fix is emitted for imports because aliasing or path changes can alter semantics",
                    ),
                );
            }
        }
    }

    fn run_text_fallback(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            let source = file.text();
            let mut offset = 0usize;
            let mut seen = BTreeSet::<(String, usize)>::new();

            for line in source.lines() {
                for (name, column) in find_let_bindings(line) {
                    if name.starts_with('_') {
                        continue;
                    }
                    let declaration_offset = offset + column;
                    if !seen.insert((name.clone(), declaration_offset)) {
                        continue;
                    }
                    if count_identifier_occurrences(source, &name) <= 1 {
                        let span = file
                            .span_for_range(declaration_offset, declaration_offset + name.len());
                        out.push(
                            ctx.diagnostic(
                                self.id(),
                                CORRECTNESS,
                                format!("`{name}` is declared but never used"),
                                span.clone(),
                            )
                            .help(
                                "prefix intentionally unused local bindings with `_` to silence this warning",
                            )
                            .span_suggestion(
                                span,
                                format!("prefix `{name}` with `_`"),
                                format!("_{name}"),
                                Applicability::MachineApplicable,
                            ),
                        );
                    }
                }

                for (name, column) in import_bindings(line) {
                    if name.starts_with('_') {
                        continue;
                    }
                    let declaration_offset = offset + column;
                    if !seen.insert((name.clone(), declaration_offset)) {
                        continue;
                    }
                    if count_identifier_occurrences(source, &name) <= 1 {
                        out.push(
                            ctx.diagnostic(
                                self.id(),
                                CORRECTNESS,
                                format!("import `{name}` is never used"),
                                file.span_for_range(declaration_offset, declaration_offset + name.len()),
                            )
                            .note(
                                "no automatic fix is emitted for imports because aliasing or path changes can alter semantics",
                            ),
                        );
                    }
                }

                offset += line.len() + 1;
            }
        }
    }
}

fn semantic_available(ctx: &RuleContext<'_>) -> bool {
    let semantic = ctx.semantic_model();
    !semantic.statements.is_empty()
        || !semantic.expressions.is_empty()
        || !semantic.dfg_edges.is_empty()
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

fn identifier_at_span(file: &SourceFile, start: u32, end: u32) -> Option<String> {
    let source = source_slice(file.text(), start, end)?;
    extract_identifiers(source)
        .into_iter()
        .map(|(identifier, _)| identifier)
        .next_back()
}

fn import_bindings(line: &str) -> Vec<(String, usize)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("use ") {
        return Vec::new();
    }

    let use_start = line.find("use ").unwrap_or(0) + "use ".len();
    let clause = line[use_start..]
        .split_once(';')
        .map_or(&line[use_start..], |(prefix, _)| prefix);
    let mut out = Vec::<(String, usize)>::new();
    let mut search_from = 0usize;

    for binding in parse_use_clause_bindings(clause) {
        let Some(relative) = clause[search_from..].find(&binding) else {
            continue;
        };
        let absolute_relative = search_from + relative;
        out.push((binding.clone(), use_start + absolute_relative));
        search_from = absolute_relative + binding.len();
    }

    out
}

fn import_bindings_in_use_statement(statement: &str) -> Vec<(String, usize)> {
    let Some(use_start) = find_keyword(statement, "use") else {
        return Vec::new();
    };
    let clause_start = use_start + "use".len();
    let mut cursor = clause_start;
    let bytes = statement.as_bytes();
    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }

    let clause = statement[cursor..]
        .split_once(';')
        .map_or(&statement[cursor..], |(prefix, _)| prefix);
    let mut out = Vec::<(String, usize)>::new();
    let mut search_from = 0usize;

    for binding in parse_use_clause_bindings(clause) {
        let Some(relative) = clause[search_from..].find(&binding) else {
            continue;
        };
        let absolute_relative = search_from + relative;
        out.push((binding.clone(), cursor + absolute_relative));
        search_from = absolute_relative + binding.len();
    }

    out
}

fn find_keyword(source: &str, keyword: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let keyword_bytes = keyword.as_bytes();
    let mut index = 0usize;

    while index + keyword_bytes.len() <= bytes.len() {
        if &bytes[index..index + keyword_bytes.len()] != keyword_bytes {
            index += 1;
            continue;
        }
        let left_ok = index == 0 || !is_ident_continue(bytes[index - 1]);
        let right_ok = bytes
            .get(index + keyword_bytes.len())
            .is_none_or(|byte| !is_ident_continue(*byte));
        if left_ok && right_ok {
            return Some(index);
        }
        index += 1;
    }

    None
}

fn parse_use_clause_bindings(clause: &str) -> Vec<String> {
    let trimmed = clause.trim();
    let mut out = Vec::<String>::new();

    if let (Some(open), Some(close)) = (trimmed.find('{'), trimmed.rfind('}'))
        && open < close
    {
        let inner = &trimmed[open + 1..close];
        for part in inner.split(',') {
            if let Some(binding) = parse_single_import_binding(part) {
                out.push(binding);
            }
        }
        return out;
    }

    for part in trimmed.split(',') {
        if let Some(binding) = parse_single_import_binding(part) {
            out.push(binding);
        }
    }

    out
}

fn parse_single_import_binding(part: &str) -> Option<String> {
    let trimmed = part.trim();
    if trimmed.is_empty() || trimmed == "*" {
        return None;
    }

    let candidate = trimmed
        .rsplit_once(" as ")
        .map(|(_, alias)| alias.trim())
        .unwrap_or_else(|| trimmed.rsplit("::").next().unwrap_or(trimmed).trim());
    if candidate.is_empty() {
        return None;
    }

    let candidate = candidate.trim_matches('{').trim_matches('}');
    if candidate.is_empty() || matches!(candidate, "crate" | "super" | "self" | "pub" | "*") {
        return None;
    }
    Some(candidate.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use aztec_lint_core::fix::{FixApplicationMode, FixSource, apply_fixes};
    use aztec_lint_core::model::{
        DfgEdge, DfgEdgeKind, ExpressionCategory, ProjectModel, SemanticExpression,
        SemanticFunction, SemanticStatement, Span, StatementCategory, SymbolKind, SymbolRef,
        TypeCategory,
    };

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir001UnusedRule;

    #[test]
    fn reports_unused_local_binding() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let value = 7; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("never used"));
    }

    #[test]
    fn ignores_used_bindings() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let value = 7; assert(value == 7); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_module_prefixes_in_use_paths() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "use math::add;\nfn main() { let x = add(1, 2); assert(x == 3); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn emits_machine_applicable_suggestion_for_unused_local_binding() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let value = 7; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].structured_suggestions.len(), 1);
        assert_eq!(
            diagnostics[0].structured_suggestions[0].applicability,
            aztec_lint_core::diagnostics::Applicability::MachineApplicable
        );
        assert_eq!(
            diagnostics[0].structured_suggestions[0].replacement,
            "_value"
        );
    }

    #[test]
    fn omits_autofix_for_unused_import_when_confidence_is_insufficient() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "use math::add;\nfn main() {}".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].structured_suggestions.is_empty());
    }

    #[test]
    fn machine_applicable_suggestion_produces_valid_fix_output() {
        let project = ProjectModel::default();
        let source_text = "fn main() { let value = 7; }\n";
        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source_text.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);

        let temp_root = temp_test_root("noir001_fix");
        let source_path = temp_root.join("src/main.nr");
        fs::create_dir_all(source_path.parent().expect("source parent should exist"))
            .expect("source directory should exist");
        fs::write(&source_path, source_text).expect("source file should be written");

        let report = apply_fixes(&temp_root, &diagnostics, FixApplicationMode::Apply)
            .expect("fix application should succeed");
        assert_eq!(report.selected.len(), 1);
        assert_eq!(report.selected[0].source, FixSource::StructuredSuggestion);

        let updated = fs::read_to_string(&source_path).expect("updated source should be readable");
        assert!(updated.contains("let _value = 7;"));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn semantic_dfg_identifies_unused_local_bindings() {
        let source = "fn main() { let value = 7; }";
        let (function_start, function_end) = span_range(source, "fn main() { let value = 7; }");
        let (statement_start, statement_end) = span_range(source, "let value = 7;");

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
        project.semantic.statements.push(SemanticStatement {
            stmt_id: "stmt::1".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: StatementCategory::Let,
            span: Span::new("src/main.nr", statement_start, statement_end, 1, 1),
        });
        project.semantic.dfg_edges.push(DfgEdge {
            function_symbol_id: "fn::main".to_string(),
            from_node_id: "stmt::1".to_string(),
            to_node_id: "def::1".to_string(),
            kind: DfgEdgeKind::DefUse,
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );

        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0]
                .message
                .contains("`value` is declared but never used")
        );
    }

    #[test]
    fn semantic_identifier_uses_prevent_import_false_positive() {
        let source = "use math::ops::sum as add_two;\nfn main() { let value = add_two(1, 2); assert(value == 3); }";
        let (import_start, import_end) = span_range(source, "use math::ops::sum as add_two;");
        let add_two_start = source
            .match_indices("add_two")
            .nth(1)
            .map(|(idx, _)| idx)
            .expect("alias call should exist");
        let add_two_end = add_two_start + "add_two".len();

        let mut project = ProjectModel::default();
        project.symbols.push(SymbolRef {
            symbol_id: "import::1".to_string(),
            name: "math::ops::sum as add_two".to_string(),
            kind: SymbolKind::Import,
            span: Span::new("src/main.nr", import_start, import_end, 1, 1),
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
            span: Span::new(
                "src/main.nr",
                import_end.saturating_add(1),
                u32::try_from(source.len()).unwrap_or(u32::MAX),
                2,
                1,
            ),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::1".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Identifier,
            type_category: TypeCategory::Function,
            type_repr: "fn(Field, Field) -> Field".to_string(),
            span: Span::new(
                "src/main.nr",
                u32::try_from(add_two_start).unwrap_or(u32::MAX),
                u32::try_from(add_two_end).unwrap_or(u32::MAX),
                2,
                33,
            ),
        });
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );

        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    fn temp_test_root(prefix: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("aztec_lint_{prefix}_{timestamp}"));
        fs::create_dir_all(&path).expect("temp root should be created");
        path
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
