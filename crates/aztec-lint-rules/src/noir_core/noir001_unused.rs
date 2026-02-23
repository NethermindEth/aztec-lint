use std::collections::{BTreeMap, BTreeSet};

use aztec_lint_core::diagnostics::{
    Applicability, Diagnostic, SuggestionGroup, TextEdit, normalize_file_path,
};
use aztec_lint_core::model::{ExpressionCategory, StatementCategory, SymbolKind};
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::engine::context::{RuleContext, SourceFile};
use crate::noir_core::util::{
    count_identifier_occurrences, extract_identifiers, is_ident_continue,
    text_fallback_line_bindings, text_fallback_statement_bindings,
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
            let mut seen_definitions = BTreeSet::new();
            definitions.retain(|definition| seen_definitions.insert(definition.clone()));
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

            let bindings = text_fallback_statement_bindings(statement_source);
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
                let mut diagnostic = ctx
                    .diagnostic(
                        self.id(),
                        CORRECTNESS,
                        format!("`{name}` is declared but never used"),
                        local_span.clone(),
                    )
                    .help(
                        "prefix intentionally unused local bindings with `_` to silence this warning",
                    );
                diagnostic.suggestion_groups.push(SuggestionGroup {
                    id: "sg0001".to_string(),
                    message: format!("prefix `{name}` with `_`"),
                    applicability: Applicability::MachineApplicable,
                    edits: vec![TextEdit {
                        span: local_span,
                        replacement: format!("_{name}"),
                    }],
                    provenance: None,
                });
                out.push(diagnostic);
            }
        }

        let mut identifiers_by_file = BTreeMap::<String, BTreeSet<String>>::new();
        let mut method_calls_by_file = BTreeMap::<String, BTreeSet<String>>::new();
        let mut associated_calls_by_file = BTreeMap::<String, BTreeSet<String>>::new();
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
        for (normalized_file, file) in &files {
            let mut references = attribute_reference_identifiers(file.text());
            references.extend(type_reference_identifiers(file.text()));
            references.extend(value_path_reference_identifiers(file.text()));
            if !references.is_empty() {
                identifiers_by_file
                    .entry(normalized_file.clone())
                    .or_default()
                    .extend(references);
            }
            method_calls_by_file.insert(
                normalized_file.clone(),
                method_call_identifiers(file.text()),
            );
            associated_calls_by_file.insert(
                normalized_file.clone(),
                associated_call_identifiers(file.text()),
            );
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
            if is_public_use_statement(import_source) {
                continue;
            }

            let imported_bindings = import_bindings_in_use_statement(import_source);
            let Some(import_start) = usize::try_from(import_symbol.span.start).ok() else {
                continue;
            };
            let used_identifiers = identifiers_by_file
                .get(&normalized_file)
                .cloned()
                .unwrap_or_default();
            let method_calls = method_calls_by_file
                .get(&normalized_file)
                .cloned()
                .unwrap_or_default();
            let associated_calls = associated_calls_by_file
                .get(&normalized_file)
                .cloned()
                .unwrap_or_default();
            for binding in imported_bindings {
                if binding.local_name.starts_with('_')
                    || import_binding_is_used(
                        &binding,
                        &used_identifiers,
                        &method_calls,
                        &associated_calls,
                    )
                {
                    continue;
                }

                let declaration_offset = import_start.saturating_add(binding.relative_start);
                if !seen.insert((binding.local_name.clone(), declaration_offset)) {
                    continue;
                }
                out.push(
                    ctx.diagnostic(
                        self.id(),
                        CORRECTNESS,
                        format!("import `{}` is never used", binding.local_name),
                        file.span_for_range(
                            declaration_offset,
                            declaration_offset + binding.local_name.len(),
                        ),
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
            let method_calls = method_call_identifiers(source);
            let associated_calls = associated_call_identifiers(source);
            let empty_identifiers = BTreeSet::<String>::new();

            for line in source.lines() {
                for (name, column) in text_fallback_line_bindings(line) {
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
                        let mut diagnostic = ctx
                            .diagnostic(
                                self.id(),
                                CORRECTNESS,
                                format!("`{name}` is declared but never used"),
                                span.clone(),
                            )
                            .help(
                                "prefix intentionally unused local bindings with `_` to silence this warning",
                            );
                        diagnostic.suggestion_groups.push(SuggestionGroup {
                            id: "sg0001".to_string(),
                            message: format!("prefix `{name}` with `_`"),
                            applicability: Applicability::MachineApplicable,
                            edits: vec![TextEdit {
                                span,
                                replacement: format!("_{name}"),
                            }],
                            provenance: None,
                        });
                        out.push(diagnostic);
                    }
                }

                for (binding, column) in import_bindings(line) {
                    if binding.local_name.starts_with('_')
                        || import_binding_is_used(
                            &binding,
                            &empty_identifiers,
                            &method_calls,
                            &associated_calls,
                        )
                    {
                        continue;
                    }
                    let declaration_offset = offset + column;
                    if !seen.insert((binding.local_name.clone(), declaration_offset)) {
                        continue;
                    }
                    if count_identifier_occurrences(source, &binding.local_name) <= 1 {
                        out.push(
                            ctx.diagnostic(
                                self.id(),
                                CORRECTNESS,
                                format!("import `{}` is never used", binding.local_name),
                                file.span_for_range(
                                    declaration_offset,
                                    declaration_offset + binding.local_name.len(),
                                ),
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

fn attribute_reference_identifiers(source: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let bytes = source.as_bytes();
    let mut cursor = 0usize;

    while cursor + 1 < bytes.len() {
        if bytes[cursor] != b'#' || bytes[cursor + 1] != b'[' {
            cursor += 1;
            continue;
        }

        let content_start = cursor + 2;
        let Some(content_end) = find_attribute_content_end(source, content_start) else {
            break;
        };
        collect_attribute_path_tails(&source[content_start..content_end], &mut out);
        cursor = content_end + 1;
    }

    out
}

fn find_attribute_content_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 1usize;
    let mut cursor = start;

    while cursor < bytes.len() {
        match bytes[cursor] {
            b'"' | b'\'' => {
                cursor = skip_quoted_literal(bytes, cursor);
                continue;
            }
            b'[' => depth += 1,
            b']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(cursor);
                }
            }
            _ => {}
        }
        cursor += 1;
    }

    None
}

fn collect_attribute_path_tails(content: &str, out: &mut BTreeSet<String>) {
    let bytes = content.as_bytes();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        if matches!(bytes[cursor], b'"' | b'\'') {
            cursor = skip_quoted_literal(bytes, cursor);
            continue;
        }
        if !is_ident_continue(bytes[cursor])
            || (cursor > 0 && is_ident_continue(bytes[cursor - 1]))
            || !bytes[cursor].is_ascii_alphabetic() && bytes[cursor] != b'_'
        {
            cursor += 1;
            continue;
        }

        let mut segment_start = cursor;
        cursor += 1;
        while cursor < bytes.len() && is_ident_continue(bytes[cursor]) {
            cursor += 1;
        }
        let mut segment_end = cursor;

        loop {
            let mut lookahead = cursor;
            while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
                lookahead += 1;
            }
            if lookahead + 1 >= bytes.len()
                || bytes[lookahead] != b':'
                || bytes[lookahead + 1] != b':'
            {
                break;
            }
            lookahead += 2;
            while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
                lookahead += 1;
            }
            if lookahead >= bytes.len()
                || !bytes[lookahead].is_ascii_alphabetic() && bytes[lookahead] != b'_'
            {
                break;
            }

            segment_start = lookahead;
            lookahead += 1;
            while lookahead < bytes.len() && is_ident_continue(bytes[lookahead]) {
                lookahead += 1;
            }
            segment_end = lookahead;
            cursor = lookahead;
        }

        let mut after = cursor;
        while after < bytes.len() && bytes[after].is_ascii_whitespace() {
            after += 1;
        }
        if bytes.get(after) == Some(&b'=') {
            continue;
        }

        let candidate = content[segment_start..segment_end].trim();
        if candidate.is_empty() || matches!(candidate, "self" | "super" | "crate") {
            continue;
        }
        out.insert(candidate.to_string());
    }
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

fn type_reference_identifiers(source: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    collect_colon_type_references(source, &mut out);
    collect_return_type_references(source, &mut out);
    collect_type_alias_references(source, &mut out);
    collect_impl_trait_references(source, &mut out);
    collect_as_trait_references(source, &mut out);
    out
}

fn collect_colon_type_references(source: &str, out: &mut BTreeSet<String>) {
    let bytes = source.as_bytes();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        if bytes[cursor] != b':' {
            cursor += 1;
            continue;
        }
        if bytes.get(cursor.saturating_sub(1)) == Some(&b':')
            || bytes.get(cursor + 1) == Some(&b':')
        {
            cursor += 1;
            continue;
        }

        let mut start = cursor + 1;
        while start < bytes.len() && bytes[start].is_ascii_whitespace() {
            start += 1;
        }
        if start >= bytes.len() {
            break;
        }

        let end = type_expression_end(source, start);
        collect_identifier_tokens(&source[start..end], out);
        cursor = end;
    }
}

fn collect_return_type_references(source: &str, out: &mut BTreeSet<String>) {
    let bytes = source.as_bytes();
    let mut cursor = 0usize;

    while cursor + 1 < bytes.len() {
        if bytes[cursor] != b'-' || bytes[cursor + 1] != b'>' {
            cursor += 1;
            continue;
        }
        let mut start = cursor + 2;
        while start < bytes.len() && bytes[start].is_ascii_whitespace() {
            start += 1;
        }
        if start >= bytes.len() {
            break;
        }
        let end = type_expression_end(source, start);
        collect_identifier_tokens(&source[start..end], out);
        cursor = end;
    }
}

fn collect_type_alias_references(source: &str, out: &mut BTreeSet<String>) {
    let mut cursor = 0usize;
    while let Some(type_start) = find_keyword(&source[cursor..], "type") {
        let absolute_type_start = cursor + type_start;
        let after_type = absolute_type_start + "type".len();
        let Some(eq_offset) = source[after_type..].find('=') else {
            cursor = after_type;
            continue;
        };
        let value_start = after_type + eq_offset + 1;
        let Some(semicolon_offset) = source[value_start..].find(';') else {
            cursor = value_start;
            continue;
        };
        let value_end = value_start + semicolon_offset;
        collect_identifier_tokens(&source[value_start..value_end], out);
        cursor = value_end + 1;
    }
}

fn collect_impl_trait_references(source: &str, out: &mut BTreeSet<String>) {
    let mut cursor = 0usize;
    while let Some(impl_start_rel) = find_keyword(&source[cursor..], "impl") {
        let impl_start = cursor + impl_start_rel + "impl".len();
        let Some(for_rel) = find_keyword(&source[impl_start..], "for") else {
            cursor = impl_start;
            continue;
        };
        let trait_part_end = impl_start + for_rel;
        collect_identifier_tokens(&source[impl_start..trait_part_end], out);
        cursor = trait_part_end + "for".len();
    }
}

fn collect_as_trait_references(source: &str, out: &mut BTreeSet<String>) {
    let bytes = source.as_bytes();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if bytes[cursor] != b'<' {
            cursor += 1;
            continue;
        }
        let mut end = cursor + 1;
        let mut depth = 1usize;
        while end < bytes.len() && depth > 0 {
            match bytes[end] {
                b'<' => depth += 1,
                b'>' => depth = depth.saturating_sub(1),
                b'"' | b'\'' => {
                    end = skip_quoted_literal(bytes, end);
                    continue;
                }
                _ => {}
            }
            end += 1;
        }
        if depth != 0 || end <= cursor + 1 {
            break;
        }

        let inner = &source[cursor + 1..end - 1];
        if let Some(as_index) = find_keyword(inner, "as") {
            let trait_part = &inner[as_index + "as".len()..];
            collect_identifier_tokens(trait_part, out);
        }
        cursor = end;
    }
}

fn type_expression_end(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let mut cursor = start;
    let mut angle_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;

    while cursor < bytes.len() {
        match bytes[cursor] {
            b'"' | b'\'' => {
                cursor = skip_quoted_literal(bytes, cursor);
                continue;
            }
            b'<' => angle_depth += 1,
            b'>' => {
                if angle_depth > 0 {
                    angle_depth -= 1;
                } else if paren_depth == 0 && bracket_depth == 0 {
                    break;
                }
            }
            b'(' => paren_depth += 1,
            b')' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                } else if angle_depth == 0 && bracket_depth == 0 {
                    break;
                }
            }
            b'[' => bracket_depth += 1,
            b']' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                } else if angle_depth == 0 && paren_depth == 0 {
                    break;
                }
            }
            b',' | b';' | b'{' | b'='
                if angle_depth == 0 && paren_depth == 0 && bracket_depth == 0 =>
            {
                break;
            }
            _ => {}
        }
        cursor += 1;
    }

    cursor
}

fn collect_identifier_tokens(segment: &str, out: &mut BTreeSet<String>) {
    const KEYWORDS: &[&str] = &[
        "as", "contract", "crate", "else", "enum", "fn", "for", "if", "impl", "in", "let", "mod",
        "mut", "pub", "return", "self", "struct", "super", "trait", "type", "use", "where",
        "while",
    ];
    for (identifier, _) in extract_identifiers(segment) {
        if KEYWORDS.contains(&identifier.as_str()) {
            continue;
        }
        out.insert(identifier);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ImportBinding {
    local_name: String,
    source_name: String,
    full_path: String,
    relative_start: usize,
}

impl ImportBinding {
    fn is_trait_namespace_import(&self) -> bool {
        let path = self.full_path.trim_start_matches("::");
        path.contains("::traits::")
            || path.starts_with("traits::")
            || path.ends_with("::traits")
            || self
                .local_name
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
    }
}

fn import_binding_is_used(
    binding: &ImportBinding,
    identifiers: &BTreeSet<String>,
    method_calls: &BTreeSet<String>,
    associated_calls: &BTreeSet<String>,
) -> bool {
    if identifiers.contains(&binding.local_name) || identifiers.contains(&binding.source_name) {
        return true;
    }

    if !binding.is_trait_namespace_import() {
        return false;
    }

    let method_candidates = trait_method_name_candidates(&binding.source_name);
    if method_candidates
        .iter()
        .any(|candidate| method_calls.contains(candidate) || associated_calls.contains(candidate))
    {
        return true;
    }

    // Conservative fallback for trait imports: if method resolution is not available,
    // keep trait imports whenever trait-like call sites are present in the file.
    !method_calls.is_empty() || !associated_calls.is_empty()
}

fn trait_method_name_candidates(trait_name: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let trimmed = trait_name.trim();
    if trimmed.is_empty() {
        return out;
    }
    let snake = camel_to_snake(trimmed);
    if !snake.is_empty() {
        out.insert(snake);
    }
    out.insert(trimmed.to_ascii_lowercase());
    out
}

fn camel_to_snake(input: &str) -> String {
    let mut out = String::with_capacity(input.len().saturating_mul(2));
    let mut previous_is_lower = false;
    let mut iter = input.chars().peekable();

    while let Some(ch) = iter.next() {
        if ch.is_ascii_uppercase() {
            let next_is_lower = iter.peek().is_some_and(|next| next.is_ascii_lowercase());
            if !out.is_empty() && (previous_is_lower || next_is_lower) {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            previous_is_lower = false;
        } else if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch.to_ascii_lowercase());
            previous_is_lower = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        }
    }

    out
}

fn value_path_reference_identifiers(source: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let mut in_use_statement = false;
    for line in source.lines() {
        let trimmed = line.trim_start();
        if !in_use_statement
            && (trimmed.starts_with("use ")
                || trimmed.starts_with("pub use ")
                || (trimmed.starts_with("pub(") && trimmed.contains(" use ")))
        {
            in_use_statement = true;
        }
        if in_use_statement {
            if trimmed.contains(';') {
                in_use_statement = false;
            }
            continue;
        }
        collect_value_path_reference_identifiers(line, &mut out);
    }
    out
}

fn collect_value_path_reference_identifiers(segment: &str, out: &mut BTreeSet<String>) {
    let bytes = segment.as_bytes();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        if !bytes[cursor].is_ascii_alphabetic() && bytes[cursor] != b'_' {
            cursor += 1;
            continue;
        }
        if cursor > 0 && is_ident_continue(bytes[cursor - 1]) {
            cursor += 1;
            continue;
        }

        let ident_start = cursor;
        cursor += 1;
        while cursor < bytes.len() && is_ident_continue(bytes[cursor]) {
            cursor += 1;
        }
        let ident_end = cursor;
        let identifier = &segment[ident_start..ident_end];
        if matches!(identifier, "self" | "super" | "crate") {
            continue;
        }

        let mut lookahead = cursor;
        while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
            lookahead += 1;
        }
        if lookahead + 1 < bytes.len() && bytes[lookahead] == b':' && bytes[lookahead + 1] == b':' {
            out.insert(identifier.to_string());
            continue;
        }
        if bytes.get(lookahead) == Some(&b'{')
            && identifier
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            out.insert(identifier.to_string());
        }
    }
}

fn method_call_identifiers(source: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let bytes = source.as_bytes();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        if bytes[cursor] != b'.' {
            cursor += 1;
            continue;
        }
        cursor += 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || (!bytes[cursor].is_ascii_alphabetic() && bytes[cursor] != b'_')
        {
            continue;
        }

        let start = cursor;
        cursor += 1;
        while cursor < bytes.len() && is_ident_continue(bytes[cursor]) {
            cursor += 1;
        }
        let end = cursor;

        let mut lookahead = cursor;
        while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
            lookahead += 1;
        }
        if lookahead + 2 < bytes.len()
            && bytes[lookahead] == b':'
            && bytes[lookahead + 1] == b':'
            && bytes[lookahead + 2] == b'<'
        {
            lookahead += 3;
            let mut depth = 1usize;
            while lookahead < bytes.len() && depth > 0 {
                match bytes[lookahead] {
                    b'<' => depth += 1,
                    b'>' => depth = depth.saturating_sub(1),
                    b'"' | b'\'' => {
                        lookahead = skip_quoted_literal(bytes, lookahead);
                        continue;
                    }
                    _ => {}
                }
                lookahead += 1;
            }
            while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
                lookahead += 1;
            }
        }

        if bytes.get(lookahead) != Some(&b'(') {
            continue;
        }

        out.insert(source[start..end].to_string());
        cursor = end;
    }

    out
}

fn associated_call_identifiers(source: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let bytes = source.as_bytes();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        if !bytes[cursor].is_ascii_alphabetic() && bytes[cursor] != b'_' {
            cursor += 1;
            continue;
        }
        if cursor > 0 && is_ident_continue(bytes[cursor - 1]) {
            cursor += 1;
            continue;
        }

        cursor += 1;
        while cursor < bytes.len() && is_ident_continue(bytes[cursor]) {
            cursor += 1;
        }
        let mut lookahead = cursor;
        while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
            lookahead += 1;
        }
        if lookahead + 1 >= bytes.len() || bytes[lookahead] != b':' || bytes[lookahead + 1] != b':'
        {
            continue;
        }

        lookahead += 2;
        while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
            lookahead += 1;
        }
        if lookahead >= bytes.len()
            || (!bytes[lookahead].is_ascii_alphabetic() && bytes[lookahead] != b'_')
        {
            continue;
        }

        let member_start = lookahead;
        lookahead += 1;
        while lookahead < bytes.len() && is_ident_continue(bytes[lookahead]) {
            lookahead += 1;
        }
        let member_end = lookahead;

        while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
            lookahead += 1;
        }
        if lookahead + 2 < bytes.len()
            && bytes[lookahead] == b':'
            && bytes[lookahead + 1] == b':'
            && bytes[lookahead + 2] == b'<'
        {
            lookahead += 3;
            let mut depth = 1usize;
            while lookahead < bytes.len() && depth > 0 {
                match bytes[lookahead] {
                    b'<' => depth += 1,
                    b'>' => depth = depth.saturating_sub(1),
                    b'"' | b'\'' => {
                        lookahead = skip_quoted_literal(bytes, lookahead);
                        continue;
                    }
                    _ => {}
                }
                lookahead += 1;
            }
            while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
                lookahead += 1;
            }
        }

        if bytes.get(lookahead) != Some(&b'(') {
            continue;
        }

        out.insert(source[member_start..member_end].to_string());
        cursor = member_end;
    }

    out
}

fn is_public_use_statement(statement: &str) -> bool {
    let trimmed = statement.trim_start();
    let Some(pub_start) = find_keyword(trimmed, "pub") else {
        return false;
    };
    if pub_start != 0 {
        return false;
    }

    let bytes = trimmed.as_bytes();
    let mut cursor = "pub".len();
    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }

    if bytes.get(cursor) == Some(&b'(') {
        cursor += 1;
        let mut depth = 1usize;
        while cursor < bytes.len() && depth > 0 {
            match bytes[cursor] {
                b'(' => depth += 1,
                b')' => depth = depth.saturating_sub(1),
                _ => {}
            }
            cursor += 1;
        }
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
    }

    trimmed[cursor..].starts_with("use ")
}

fn import_bindings(line: &str) -> Vec<(ImportBinding, usize)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("use ") {
        return Vec::new();
    }

    let use_start = line.find("use ").unwrap_or(0) + "use ".len();
    let clause = line[use_start..]
        .split_once(';')
        .map_or(&line[use_start..], |(prefix, _)| prefix);
    let mut out = Vec::<(ImportBinding, usize)>::new();
    let mut search_from = 0usize;

    for binding in parse_use_clause_binding_details(clause) {
        let Some(relative) = clause[search_from..].find(&binding.local_name) else {
            continue;
        };
        let absolute_relative = search_from + relative;
        let name_len = binding.local_name.len();
        out.push((binding, use_start + absolute_relative));
        search_from = absolute_relative + name_len;
    }

    out
}

fn import_bindings_in_use_statement(statement: &str) -> Vec<ImportBinding> {
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
    let mut out = Vec::<ImportBinding>::new();
    let mut search_from = 0usize;

    for mut binding in parse_use_clause_binding_details(clause) {
        let Some(relative) = clause[search_from..].find(&binding.local_name) else {
            continue;
        };
        let absolute_relative = search_from + relative;
        let name_len = binding.local_name.len();
        binding.relative_start = cursor + absolute_relative;
        out.push(binding);
        search_from = absolute_relative + name_len;
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

fn parse_use_clause_binding_details(clause: &str) -> Vec<ImportBinding> {
    let mut out = Vec::<ImportBinding>::new();
    parse_use_clause_bindings_recursive(clause.trim(), "", &mut out);
    out
}

fn parse_use_clause_bindings_recursive(clause: &str, prefix: &str, out: &mut Vec<ImportBinding>) {
    for part in split_top_level(clause, b',') {
        parse_single_import_binding(part.trim(), prefix, out);
    }
}

fn parse_single_import_binding(part: &str, prefix: &str, out: &mut Vec<ImportBinding>) {
    let trimmed = part.trim();
    if trimmed.is_empty() || trimmed == "*" {
        return;
    }

    if let Some((head, inner)) = split_braced_group(trimmed) {
        let head = head.trim().trim_end_matches("::").trim();
        let nested_prefix = if head.is_empty() {
            prefix.to_string()
        } else {
            join_use_path(prefix, head)
        };
        parse_use_clause_bindings_recursive(inner, &nested_prefix, out);
        return;
    }

    let (path_part, alias) = split_use_alias(trimmed);
    let path_part = path_part.trim_matches('{').trim_matches('}').trim();
    if path_part.is_empty() || path_part == "*" {
        return;
    }

    let source_name = if path_part == "self" {
        prefix
            .rsplit("::")
            .next()
            .map(|value| value.trim())
            .unwrap_or_default()
            .to_string()
    } else {
        path_part
            .rsplit("::")
            .next()
            .map(|value| value.trim())
            .unwrap_or_default()
            .to_string()
    };
    let local_name = alias.unwrap_or_else(|| source_name.clone());
    if local_name.is_empty()
        || matches!(
            local_name.as_str(),
            "crate" | "super" | "self" | "pub" | "mut" | "*"
        )
    {
        return;
    }

    let full_path = if path_part == "self" {
        prefix.to_string()
    } else {
        join_use_path(prefix, path_part)
    };
    out.push(ImportBinding {
        local_name,
        source_name,
        full_path,
        relative_start: 0,
    });
}

fn split_top_level(input: &str, delimiter: u8) -> Vec<&str> {
    let bytes = input.as_bytes();
    let mut out = Vec::<&str>::new();
    let mut start = 0usize;
    let mut cursor = 0usize;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;

    while cursor < bytes.len() {
        match bytes[cursor] {
            b'"' | b'\'' => {
                cursor = skip_quoted_literal(bytes, cursor);
                continue;
            }
            b'{' => brace_depth += 1,
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            _ => {}
        }

        if bytes[cursor] == delimiter && brace_depth == 0 && paren_depth == 0 && bracket_depth == 0
        {
            out.push(&input[start..cursor]);
            start = cursor + 1;
        }
        cursor += 1;
    }

    out.push(&input[start..]);
    out
}

fn split_braced_group(input: &str) -> Option<(&str, &str)> {
    let bytes = input.as_bytes();
    let mut cursor = 0usize;
    let mut open = None::<usize>;
    let mut depth = 0usize;

    while cursor < bytes.len() {
        match bytes[cursor] {
            b'"' | b'\'' => {
                cursor = skip_quoted_literal(bytes, cursor);
                continue;
            }
            b'{' => {
                if depth == 0 {
                    open = Some(cursor);
                }
                depth += 1;
            }
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let open_index = open?;
                    if !input[cursor + 1..].trim().is_empty() {
                        return None;
                    }
                    return Some((input[..open_index].trim(), &input[open_index + 1..cursor]));
                }
            }
            _ => {}
        }
        cursor += 1;
    }

    None
}

fn split_use_alias(input: &str) -> (&str, Option<String>) {
    if let Some((path, alias)) = input.rsplit_once(" as ") {
        return (path.trim(), Some(alias.trim().to_string()));
    }
    (input.trim(), None)
}

fn join_use_path(prefix: &str, tail: &str) -> String {
    let prefix = prefix.trim().trim_start_matches("::");
    let tail = tail.trim().trim_start_matches("::");
    if prefix.is_empty() {
        return tail.to_string();
    }
    if tail.is_empty() {
        return prefix.to_string();
    }
    format!("{prefix}::{tail}")
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

    use super::{Noir001UnusedRule, import_bindings_in_use_statement};

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
        assert!(diagnostics[0].structured_suggestions.is_empty());
        assert_eq!(diagnostics[0].suggestion_groups.len(), 1);
        assert_eq!(
            diagnostics[0].suggestion_groups[0].applicability,
            aztec_lint_core::diagnostics::Applicability::MachineApplicable
        );
        assert_eq!(
            diagnostics[0].suggestion_groups[0].edits[0].replacement,
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
        assert!(diagnostics[0].suggestion_groups.is_empty());
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

    #[test]
    fn semantic_binding_mapping_preserves_statement_definition_order() {
        let source = "fn main() { let (unused, _, used) = pair; assert(used == 7); }";
        let (function_start, function_end) = span_range(source, "fn main() {");
        let (statement_start, statement_end) = span_range(source, "let (unused, _, used) = pair;");

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
        project.semantic.dfg_edges.extend([
            DfgEdge {
                function_symbol_id: "fn::main".to_string(),
                from_node_id: "stmt::1".to_string(),
                to_node_id: "def::10".to_string(),
                kind: DfgEdgeKind::DefUse,
            },
            DfgEdge {
                function_symbol_id: "fn::main".to_string(),
                from_node_id: "stmt::1".to_string(),
                to_node_id: "def::2".to_string(),
                kind: DfgEdgeKind::DefUse,
            },
            DfgEdge {
                function_symbol_id: "fn::main".to_string(),
                from_node_id: "def::2".to_string(),
                to_node_id: "expr::used".to_string(),
                kind: DfgEdgeKind::UseDef,
            },
        ]);
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );

        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].message,
            "`unused` is declared but never used"
        );
    }

    #[test]
    fn semantic_attribute_macro_use_marks_import_as_used() {
        let source = "use dep::aztec::macros::aztec;\n#[aztec]\nfn main() {}";
        let project = semantic_project_with_import(
            source,
            "use dep::aztec::macros::aztec;",
            "dep::aztec::macros::aztec",
        );

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_unused_attribute_macro_import_is_reported_without_attribute_use() {
        let source = "use dep::aztec::macros::aztec;\nfn main() {}";
        let project = semantic_project_with_import(
            source,
            "use dep::aztec::macros::aztec;",
            "dep::aztec::macros::aztec",
        );

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "import `aztec` is never used");
    }

    #[test]
    fn semantic_attribute_macro_alias_marks_import_as_used() {
        let source = "use dep::aztec::macros::aztec as az;\n#[az]\nfn main() {}";
        let project = semantic_project_with_import(
            source,
            "use dep::aztec::macros::aztec as az;",
            "dep::aztec::macros::aztec as az",
        );

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_grouped_import_tracks_each_binding_independently() {
        let source =
            "use dep::aztec::macros::{events::event, hash::sha256 as h};\n#[event]\nfn main() {}";
        let project = semantic_project_with_import(
            source,
            "use dep::aztec::macros::{events::event, hash::sha256 as h};",
            "dep::aztec::macros::{events::event, hash::sha256 as h}",
        );

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "import `h` is never used");
    }

    #[test]
    fn semantic_public_reexport_is_not_reported() {
        let source = "pub use dep::types::position::PositionReceipt;\nfn main() {}";
        let project = semantic_project_with_import(
            source,
            "pub use dep::types::position::PositionReceipt;",
            "dep::types::position::PositionReceipt",
        );

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_type_position_use_marks_import_as_used() {
        let source = "use dep::aztec::address::AztecAddress;\nstruct PositionReceipt {\n    pub owner: AztecAddress,\n}\nfn main() {}";
        let project = semantic_project_with_import(
            source,
            "use dep::aztec::address::AztecAddress;",
            "dep::aztec::address::AztecAddress",
        );

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_trait_method_call_marks_trait_import_as_used() {
        let source =
            "use dep::std::traits::ToField;\nfn main() { let x = 1; let y = x.to_field(); }";
        let project = semantic_project_with_import(
            source,
            "use dep::std::traits::ToField;",
            "dep::std::traits::ToField",
        );

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_unused_trait_import_is_reported_without_trait_context() {
        let source = "use dep::std::traits::ToField;\nfn main() {}";
        let project = semantic_project_with_import(
            source,
            "use dep::std::traits::ToField;",
            "dep::std::traits::ToField",
        );

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "import `ToField` is never used");
    }

    #[test]
    fn semantic_trait_alias_import_is_used_by_method_call() {
        let source =
            "use dep::std::traits::ToField as TF;\nfn main() { let x = 1; let y = x.to_field(); }";
        let project = semantic_project_with_import(
            source,
            "use dep::std::traits::ToField as TF;",
            "dep::std::traits::ToField as TF",
        );

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_impl_trait_context_marks_import_as_used() {
        let source =
            "use dep::std::traits::ToField;\nimpl ToField for PositionReceipt {}\nfn main() {}";
        let project = semantic_project_with_import(
            source,
            "use dep::std::traits::ToField;",
            "dep::std::traits::ToField",
        );

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_qualified_trait_path_marks_import_as_used() {
        let source = "use dep::std::traits::ToField;\nfn main() { let y = <Field as ToField>::to_field(1); }";
        let project = semantic_project_with_import(
            source,
            "use dep::std::traits::ToField;",
            "dep::std::traits::ToField",
        );

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_struct_literal_and_associated_trait_call_mark_imports_as_used() {
        let source = "use crate::types::PositionReceiptNote;\nuse aztec::protocol_types::{address::AztecAddress, traits::FromField};\nfn main() {\n    let owner = AztecAddress::from_field(1);\n    let _note = PositionReceiptNote { owner };\n}";
        let project = semantic_project_with_imports(
            source,
            &[
                (
                    "use crate::types::PositionReceiptNote;",
                    "crate::types::PositionReceiptNote",
                ),
                (
                    "use aztec::protocol_types::{address::AztecAddress, traits::FromField};",
                    "aztec::protocol_types::{address::AztecAddress, traits::FromField}",
                ),
            ],
        );

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_test_function_body_uses_mark_imports_as_used() {
        let source = "use crate::types::PositionReceiptNote;\n#[test]\nunconstrained fn withdraw_test() {\n    let _note = PositionReceiptNote {};\n}\nfn main() {}";
        let project = semantic_project_with_import(
            source,
            "use crate::types::PositionReceiptNote;",
            "crate::types::PositionReceiptNote",
        );

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let mut diagnostics = Vec::new();
        Noir001UnusedRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn import_parser_flattens_nested_use_tree() {
        let bindings = import_bindings_in_use_statement("use x::{a, b::{c as d}};");
        let names = bindings
            .into_iter()
            .map(|binding| binding.local_name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["a".to_string(), "d".to_string()]);
    }

    fn semantic_project_with_import(
        source: &str,
        import_statement: &str,
        import_name: &str,
    ) -> ProjectModel {
        semantic_project_with_imports(source, &[(import_statement, import_name)])
    }

    fn semantic_project_with_imports(source: &str, imports: &[(&str, &str)]) -> ProjectModel {
        let function_start = source
            .find("fn main()")
            .expect("semantic fixture should include `fn main()`");
        let function_end = u32::try_from(source.len()).unwrap_or(u32::MAX);
        let function_start_u32 = u32::try_from(function_start).unwrap_or(u32::MAX);

        let mut project = ProjectModel::default();
        for (index, (import_statement, import_name)) in imports.iter().enumerate() {
            let (import_start, import_end) = span_range(source, import_statement);
            project.symbols.push(SymbolRef {
                symbol_id: format!("import::{}", index + 1),
                name: (*import_name).to_string(),
                kind: SymbolKind::Import,
                span: Span::new("src/main.nr", import_start, import_end, 1, 1),
            });
        }
        project.semantic.functions.push(SemanticFunction {
            symbol_id: "fn::main".to_string(),
            name: "main".to_string(),
            module_symbol_id: "module::main".to_string(),
            return_type_repr: "()".to_string(),
            return_type_category: TypeCategory::Unknown,
            parameter_types: Vec::new(),
            is_entrypoint: true,
            is_unconstrained: false,
            span: Span::new("src/main.nr", function_start_u32, function_end, 1, 1),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::seed".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Literal,
            type_category: TypeCategory::Unknown,
            type_repr: "unknown".to_string(),
            span: Span::new(
                "src/main.nr",
                function_start_u32,
                function_start_u32.saturating_add(1),
                1,
                1,
            ),
        });
        project.normalize();
        project
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
