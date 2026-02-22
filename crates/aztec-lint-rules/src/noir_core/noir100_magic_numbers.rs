use std::collections::BTreeMap;

use aztec_lint_core::diagnostics::normalize_file_path;
use aztec_lint_core::diagnostics::{Applicability, Diagnostic, SuggestionGroup, TextEdit};
use aztec_lint_core::model::{ExpressionCategory, TypeCategory};
use aztec_lint_core::policy::MAINTAINABILITY;

use crate::Rule;
use crate::engine::context::{RuleContext, SourceFile};
use crate::noir_core::util::{
    FunctionScope, extract_identifiers, extract_numeric_literals, source_slice,
    text_fallback_function_scopes,
};

pub struct Noir100MagicNumbersRule;
pub struct Noir101RepeatedLocalInitMagicNumbersRule;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MagicLiteralSignal {
    High,
    LocalInit,
    Ignore,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LocalInitCandidate {
    file: String,
    function_scope: String,
    literal: String,
    start: usize,
    len: usize,
}

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
        let include_test_paths = include_test_path_magic_number_checks();

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
            if !include_test_paths && is_test_path(file.path()) {
                continue;
            }
            let Some(source) =
                source_slice(file.text(), expression.span.start, expression.span.end)
            else {
                continue;
            };
            let Some(expression_start) = usize::try_from(expression.span.start).ok() else {
                continue;
            };
            if is_named_constant_declaration_context(file.text(), expression_start) {
                continue;
            }

            for (literal, relative_offset) in extract_numeric_literals(source) {
                let start = expression_start.saturating_add(relative_offset);
                if magic_literal_signal(file.text(), start, literal.len(), &literal)
                    != MagicLiteralSignal::High
                {
                    continue;
                }
                emit_magic_literal_diagnostic(ctx, self.id(), out, file, start, &literal);
            }
        }
    }

    fn run_text_fallback(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let include_test_paths = include_test_path_magic_number_checks();
        for file in ctx.files() {
            if !include_test_paths && is_test_path(file.path()) {
                continue;
            }
            let mut offset = 0usize;

            for line in file.text().lines() {
                let code = strip_line_comment(line);

                for (literal, column) in extract_numeric_literals(code) {
                    let start = offset + column;
                    if magic_literal_signal(file.text(), start, literal.len(), &literal)
                        != MagicLiteralSignal::High
                    {
                        continue;
                    }
                    emit_magic_literal_diagnostic(ctx, self.id(), out, file, start, &literal);
                }

                offset += line.len() + 1;
            }
        }
    }
}

impl Rule for Noir101RepeatedLocalInitMagicNumbersRule {
    fn id(&self) -> &'static str {
        "NOIR101"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        if semantic_available(ctx) {
            self.run_semantic(ctx, out);
            return;
        }
        self.run_text_fallback(ctx, out);
    }
}

impl Noir101RepeatedLocalInitMagicNumbersRule {
    fn run_semantic(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let semantic = ctx.semantic_model();
        let files = file_map(ctx.files());
        let include_test_paths = include_test_path_magic_number_checks();
        let mut candidates = Vec::<LocalInitCandidate>::new();

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
            if !include_test_paths && is_test_path(file.path()) {
                continue;
            }
            let Some(source) =
                source_slice(file.text(), expression.span.start, expression.span.end)
            else {
                continue;
            };
            let Some(expression_start) = usize::try_from(expression.span.start).ok() else {
                continue;
            };
            if is_named_constant_declaration_context(file.text(), expression_start) {
                continue;
            }

            for (literal, relative_offset) in extract_numeric_literals(source) {
                let start = expression_start.saturating_add(relative_offset);
                let literal_len = literal.len();
                if magic_literal_signal(file.text(), start, literal_len, &literal)
                    != MagicLiteralSignal::LocalInit
                {
                    continue;
                }
                candidates.push(LocalInitCandidate {
                    file: file.path().to_string(),
                    function_scope: expression.function_symbol_id.clone(),
                    literal,
                    start,
                    len: literal_len,
                });
            }
        }

        emit_repeated_local_init_diagnostics(ctx, self.id(), out, ctx.files(), candidates);
    }

    fn run_text_fallback(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let include_test_paths = include_test_path_magic_number_checks();
        let mut candidates = Vec::<LocalInitCandidate>::new();

        for file in ctx.files() {
            if !include_test_paths && is_test_path(file.path()) {
                continue;
            }
            let mut offset = 0usize;
            let scopes = text_fallback_function_scopes(file.text());

            for line in file.text().lines() {
                let code = strip_line_comment(line);

                for (literal, column) in extract_numeric_literals(code) {
                    let start = offset + column;
                    let literal_len = literal.len();
                    if magic_literal_signal(file.text(), start, literal_len, &literal)
                        != MagicLiteralSignal::LocalInit
                    {
                        continue;
                    }
                    candidates.push(LocalInitCandidate {
                        file: file.path().to_string(),
                        function_scope: function_scope_key_for_offset(&scopes, start),
                        literal,
                        start,
                        len: literal_len,
                    });
                }

                offset += line.len() + 1;
            }
        }

        emit_repeated_local_init_diagnostics(ctx, self.id(), out, ctx.files(), candidates);
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

fn magic_literal_signal(
    source: &str,
    offset: usize,
    literal_len: usize,
    literal: &str,
) -> MagicLiteralSignal {
    if is_fixture_context(source, offset) {
        return MagicLiteralSignal::Ignore;
    }
    if is_poseidon2_domain_tag_context(source, offset, literal_len, literal) {
        return MagicLiteralSignal::Ignore;
    }
    if is_named_constant_declaration_context(source, offset) {
        return MagicLiteralSignal::Ignore;
    }
    if is_byte_packing_context(source, offset, literal_len) {
        return MagicLiteralSignal::Ignore;
    }
    if is_zero_or_one_literal(literal) {
        return MagicLiteralSignal::Ignore;
    }
    if is_sensitive_magic_context(source, offset, literal_len) {
        return MagicLiteralSignal::High;
    }
    if is_local_initializer_context(source, offset, literal_len) {
        return MagicLiteralSignal::LocalInit;
    }
    MagicLiteralSignal::Ignore
}

fn file_map(files: &[SourceFile]) -> BTreeMap<String, &SourceFile> {
    files
        .iter()
        .map(|file| (normalize_file_path(file.path()), file))
        .collect::<BTreeMap<_, _>>()
}

fn emit_magic_literal_diagnostic(
    ctx: &RuleContext<'_>,
    rule_id: &str,
    out: &mut Vec<Diagnostic>,
    file: &SourceFile,
    start: usize,
    literal: &str,
) {
    let span = file.span_for_range(start, start + literal.len());
    let mut diagnostic = ctx
        .diagnostic(
            rule_id,
            MAINTAINABILITY,
            format!("magic number `{literal}` should be named"),
            span.clone(),
        )
        .help("extract this literal into a named constant for readability");
    diagnostic.suggestion_groups.push(SuggestionGroup {
        id: "sg0001".to_string(),
        message: format!("replace `{literal}` with a named constant"),
        applicability: Applicability::MaybeIncorrect,
        edits: vec![TextEdit {
            span,
            replacement: "NAMED_CONSTANT".to_string(),
        }],
        provenance: None,
    });
    out.push(diagnostic);
}

fn emit_repeated_local_init_diagnostics(
    ctx: &RuleContext<'_>,
    rule_id: &str,
    out: &mut Vec<Diagnostic>,
    files: &[SourceFile],
    candidates: Vec<LocalInitCandidate>,
) {
    let mut counts = BTreeMap::<(String, String, String), usize>::new();
    for candidate in &candidates {
        *counts
            .entry((
                normalize_file_path(&candidate.file),
                candidate.function_scope.clone(),
                candidate.literal.clone(),
            ))
            .or_default() += 1;
    }

    let files_by_path = file_map(files);
    for candidate in candidates {
        let key = (
            normalize_file_path(&candidate.file),
            candidate.function_scope.clone(),
            candidate.literal.clone(),
        );
        if counts.get(&key).copied().unwrap_or_default() < 2 {
            continue;
        }
        let Some(file) = files_by_path.get(&key.0).copied() else {
            continue;
        };
        let span = file.span_for_range(candidate.start, candidate.start + candidate.len);
        let mut diagnostic = ctx
            .diagnostic(
                rule_id,
                MAINTAINABILITY,
                format!(
                    "repeated local initializer magic number `{}` should be named",
                    candidate.literal
                ),
                span.clone(),
            )
            .help("literal repeats across local initializers; prefer a named constant");
        diagnostic.suggestion_groups.push(SuggestionGroup {
            id: "sg0001".to_string(),
            message: format!(
                "replace repeated `{}` with a named constant",
                candidate.literal
            ),
            applicability: Applicability::MaybeIncorrect,
            edits: vec![TextEdit {
                span,
                replacement: "NAMED_CONSTANT".to_string(),
            }],
            provenance: None,
        });
        out.push(diagnostic);
    }
}

fn is_named_constant_declaration_context(source: &str, offset: usize) -> bool {
    if offset > source.len() {
        return false;
    }
    let statement_start = statement_start(source, offset);
    let statement_prefix = source[statement_start..offset].trim_start();
    let mut tokens = statement_prefix.split_whitespace();
    let keyword_matches = match tokens.next() {
        Some("const") => true,
        Some("global") => true,
        Some("pub") => matches!(tokens.next(), Some("const" | "global")),
        _ => false,
    };
    if keyword_matches {
        return true;
    }

    assigned_identifier_before_offset(statement_prefix)
        .is_some_and(|identifier| is_screaming_snake_case(&identifier))
}

fn assigned_identifier_before_offset(statement_prefix: &str) -> Option<String> {
    let eq_offset = statement_prefix.rfind('=')?;
    let lhs = statement_prefix[..eq_offset]
        .split(':')
        .next()
        .map(str::trim_end)
        .unwrap_or("")
        .trim();
    if lhs.is_empty() {
        return None;
    }

    const KEYWORDS: &[&str] = &["let", "mut", "pub", "const", "global"];
    extract_identifiers(lhs)
        .into_iter()
        .map(|(identifier, _)| identifier)
        .rev()
        .find(|identifier| !KEYWORDS.contains(&identifier.as_str()))
}

fn is_screaming_snake_case(identifier: &str) -> bool {
    if identifier.is_empty() || identifier.as_bytes()[0].is_ascii_digit() {
        return false;
    }
    let mut saw_upper = false;
    for byte in identifier.bytes() {
        if byte.is_ascii_uppercase() {
            saw_upper = true;
            continue;
        }
        if byte.is_ascii_digit() || byte == b'_' {
            continue;
        }
        return false;
    }
    saw_upper
}

fn is_byte_packing_context(source: &str, offset: usize, literal_len: usize) -> bool {
    if offset >= source.len() {
        return false;
    }
    let (line_start, line_end) = line_bounds(source, offset);
    let line = source.get(line_start..line_end).unwrap_or("").trim();
    if line.is_empty() {
        return false;
    }
    let normalized = line
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect::<String>();

    if [
        "to_be_bytes",
        "to_le_bytes",
        "from_be_bytes",
        "from_le_bytes",
        "to_be_bits",
        "to_le_bits",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
    {
        return true;
    }

    if normalized.contains("[u8;") {
        return true;
    }

    if is_range_boundary_literal(source, offset, literal_len) {
        return true;
    }

    let before = source[..offset].trim_end();
    let after = source[offset + literal_len..].trim_start();
    let offset_math = before.ends_with('+')
        || before.ends_with('-')
        || after.starts_with('+')
        || after.starts_with('-');
    if offset_math && normalized.contains('[') && normalized.contains(']') {
        return true;
    }

    false
}

fn is_fixture_context(source: &str, offset: usize) -> bool {
    if offset > source.len() {
        return false;
    }
    let statement_start = statement_start(source, offset);
    let statement_end = statement_end(source, offset);
    let statement = source
        .get(statement_start..statement_end)
        .unwrap_or("")
        .to_ascii_lowercase();
    let statement = statement.trim_start();
    if statement.is_empty() {
        return false;
    }

    let fixture_labeled = [
        "fixture", "fixtures", "test", "tests", "case", "mock", "sample", "input", "expected",
        "vector",
    ]
    .iter()
    .any(|token| statement.contains(token));
    if !fixture_labeled {
        return false;
    }

    statement.starts_with("let ")
        && (statement.contains('[') || statement.contains('{') || statement.contains("new("))
}

fn is_poseidon2_domain_tag_context(
    source: &str,
    offset: usize,
    literal_len: usize,
    literal: &str,
) -> bool {
    if offset + literal_len > source.len() {
        return false;
    }
    let statement_start = statement_start(source, offset);
    let statement_end = statement_end(source, offset);
    let Some(statement) = source.get(statement_start..statement_end) else {
        return false;
    };
    let Some(call_offset) = statement.find("poseidon2_hash") else {
        return false;
    };
    let Some(open_paren_rel) = statement[call_offset..].find('(') else {
        return false;
    };
    let open_paren = call_offset + open_paren_rel;
    let Some(close_paren) = find_matching(statement, open_paren, b'(', b')') else {
        return false;
    };
    let args = statement
        .get(open_paren + 1..close_paren)
        .unwrap_or("")
        .trim();
    if !args.starts_with('[') {
        return false;
    }
    let Some(array_close) = find_matching(args, 0, b'[', b']') else {
        return false;
    };
    let array_inner = args.get(1..array_close).unwrap_or("");
    let first = first_top_level_item(array_inner).trim();
    if first.is_empty() {
        return false;
    }

    let local_offset = offset.saturating_sub(statement_start);
    let literal_end = local_offset + literal_len;
    let first_start = array_inner.find(first).map(|idx| idx + open_paren + 2);
    let Some(first_start) = first_start else {
        return false;
    };
    let first_end = first_start + first.len();
    if local_offset < first_start || literal_end > first_end {
        return false;
    }

    matches_known_hash_domain_tag(first, literal)
}

fn matches_known_hash_domain_tag(first_item: &str, literal: &str) -> bool {
    const KNOWN_TAGS: &[&str] = &[
        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
        "32", "64", "128", "160", "192", "256", "512", "1024",
    ];
    if first_item != literal {
        return false;
    }
    let compact = literal.chars().filter(|ch| *ch != '_').collect::<String>();
    KNOWN_TAGS.contains(&compact.as_str())
}

fn first_top_level_item(input: &str) -> &str {
    let bytes = input.as_bytes();
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    for (index, byte) in bytes.iter().enumerate() {
        match *byte {
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b'{' => brace_depth += 1,
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            b',' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                return input.get(..index).unwrap_or(input);
            }
            _ => {}
        }
    }
    input
}

fn find_matching(input: &str, open_index: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = input.as_bytes();
    if bytes.get(open_index).copied()? != open {
        return None;
    }
    let mut depth = 0usize;
    let mut cursor = open_index;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte == open {
            depth += 1;
        } else if byte == close {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(cursor);
            }
        }
        cursor += 1;
    }
    None
}

fn is_test_path(path: &str) -> bool {
    let normalized = normalize_file_path(path);
    normalized.split('/').any(|segment| segment == "test")
        || normalized.ends_with("_test.nr")
        || normalized.ends_with("_tests.nr")
}

fn include_test_path_magic_number_checks() -> bool {
    std::env::var("AZTEC_LINT_NOIR100_INCLUDE_TEST_PATHS")
        .ok()
        .as_deref()
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

fn function_scope_key_for_offset(scopes: &[FunctionScope], offset: usize) -> String {
    scopes
        .iter()
        .find(|scope| offset >= scope.body_start && offset < scope.body_end)
        .map(|scope| format!("fn:{}:{}", scope.name, scope.name_offset))
        .unwrap_or_else(|| "module".to_string())
}

fn is_local_initializer_context(source: &str, offset: usize, literal_len: usize) -> bool {
    if offset + literal_len > source.len() {
        return false;
    }
    let start = statement_start(source, offset);
    let end = statement_end(source, offset);
    let Some(statement) = source.get(start..end) else {
        return false;
    };
    let trimmed = statement.trim_start();
    if !trimmed.starts_with("let ") {
        return false;
    }
    let Some(eq) = top_level_assignment_offset(statement) else {
        return false;
    };
    let relative = offset.saturating_sub(start);
    relative > eq
}

fn top_level_assignment_offset(statement: &str) -> Option<usize> {
    let bytes = statement.as_bytes();
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    for (index, byte) in bytes.iter().copied().enumerate() {
        match byte {
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b'{' => brace_depth += 1,
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            b'=' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                let previous = index.checked_sub(1).and_then(|idx| bytes.get(idx));
                let next = bytes.get(index + 1);
                let comparison = previous == Some(&b'=')
                    || next == Some(&b'=')
                    || previous == Some(&b'>')
                    || previous == Some(&b'<')
                    || previous == Some(&b'!');
                if !comparison {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn is_sensitive_magic_context(source: &str, offset: usize, literal_len: usize) -> bool {
    if is_range_boundary_literal(source, offset, literal_len) {
        return true;
    }
    let start = statement_start(source, offset);
    let end = statement_end(source, offset);
    let Some(statement) = source.get(start..end) else {
        return false;
    };
    let normalized = statement.to_ascii_lowercase();
    let trimmed = normalized.trim_start();

    if trimmed.starts_with("if ")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("match ")
    {
        return true;
    }
    if normalized.contains("assert(")
        || normalized.contains("assert_eq(")
        || normalized.contains("constrain(")
    {
        return true;
    }

    const KEYWORDS: &[&str] = &[
        "poseidon",
        "hash(",
        "serialize",
        "to_bytes(",
        "from_bytes(",
        "enqueue(",
        "storage::",
        "nullifier",
        "commitment",
    ];
    KEYWORDS.iter().any(|keyword| normalized.contains(keyword))
}

fn is_range_boundary_literal(source: &str, offset: usize, literal_len: usize) -> bool {
    if offset + literal_len > source.len() {
        return false;
    }
    let before = source[..offset].trim_end();
    let after = source[offset + literal_len..].trim_start();
    before.ends_with("..") || after.starts_with("..")
}

fn statement_start(source: &str, offset: usize) -> usize {
    source[..offset]
        .rfind([';', '{', '}'])
        .map_or(0, |idx| idx + 1)
}

fn statement_end(source: &str, offset: usize) -> usize {
    let bytes = source.as_bytes();
    let mut cursor = offset.min(source.len());
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b'{' => brace_depth += 1,
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            b';' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                return cursor + 1;
            }
            b'\n' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                return cursor;
            }
            _ => {}
        }
        cursor += 1;
    }
    source.len()
}

fn line_bounds(source: &str, offset: usize) -> (usize, usize) {
    let start = source[..offset].rfind('\n').map_or(0, |idx| idx + 1);
    let end = source[offset..]
        .find('\n')
        .map_or(source.len(), |idx| offset + idx);
    (start, end)
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

    use super::{Noir100MagicNumbersRule, Noir101RepeatedLocalInitMagicNumbersRule};

    #[test]
    fn reports_magic_numbers() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main(limit: u32) { if limit > 42 { assert(limit > 0); } }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].structured_suggestions.is_empty());
        assert_eq!(diagnostics[0].suggestion_groups.len(), 1);
        assert_eq!(
            diagnostics[0].suggestion_groups[0].applicability,
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
    fn ignores_global_named_constant_declarations() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "pub global PENDING_WITHDRAW: u8 = 2;\nfn main() {}".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_uppercase_domain_constants() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let ACTIVE_STATE = 2; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_byte_packing_context_literals() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() {\n    let mut out: [u8; 32] = [0; 32];\n    for i in 0..32 {\n        out[32 + i] = out[i];\n    }\n}"
                    .to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn skips_magic_numbers_in_test_paths_by_default() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![
                (
                    "src/test/withdraw_tests.nr".to_string(),
                    "fn main() { let fee = 42; }".to_string(),
                ),
                (
                    "src/withdraw_test.nr".to_string(),
                    "fn main() { let fee = 84; }".to_string(),
                ),
            ],
        );

        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_assertion_literals_in_non_fixture_contexts() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { assert(deadline == 1700000000); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_fixture_constructor_literals() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let expected_vector = [42, 7]; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_poseidon2_domain_tag_literals() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main(x: Field) { let h = poseidon2_hash([2, x]); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn still_reports_non_domain_poseidon_literal() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main(x: Field) { let h = poseidon2_hash([42, x]); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn semantic_literals_report_magic_numbers() {
        let source = "fn main(limit: u32) { if limit > 42 { assert(limit > 0); } }";
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
                34,
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
                "fn main(limit: Field) { if limit > 1844674407370955161600 { assert(limit > 0); } }"
                    .to_string(),
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
                "fn main(limit: u32) { if limit > 42 { assert(limit > 0); } }".to_string(),
            )],
        );
        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn noir101_ignores_single_local_initializer_literal() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let fee = 42; assert(fee > 0); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir101RepeatedLocalInitMagicNumbersRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn noir101_ignores_single_local_initializer_literal_for_unused_binding() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let _unused = 9; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir101RepeatedLocalInitMagicNumbersRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn noir101_reports_repeated_local_initializer_literals() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let fee = 42; let limit = 42; assert(limit > 0); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir101RepeatedLocalInitMagicNumbersRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic
                .message
                .contains("repeated local initializer magic number")
        }));
    }
}
