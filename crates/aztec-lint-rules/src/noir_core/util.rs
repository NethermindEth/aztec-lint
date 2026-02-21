use std::collections::BTreeSet;

pub fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

pub fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

pub fn extract_identifiers(line: &str) -> Vec<(String, usize)> {
    let bytes = line.as_bytes();
    let mut idx = 0usize;
    let mut out = Vec::<(String, usize)>::new();

    while idx < bytes.len() {
        if !is_ident_start(bytes[idx]) {
            idx += 1;
            continue;
        }

        let start = idx;
        idx += 1;
        while idx < bytes.len() && is_ident_continue(bytes[idx]) {
            idx += 1;
        }

        out.push((line[start..idx].to_string(), start));
    }

    out
}

pub fn count_identifier_occurrences(source: &str, identifier: &str) -> usize {
    if identifier.is_empty() {
        return 0;
    }

    let mut count = 0usize;
    let mut start = 0usize;
    while let Some(offset) = source[start..].find(identifier) {
        let absolute = start + offset;
        let before = absolute
            .checked_sub(1)
            .and_then(|idx| source.as_bytes().get(idx));
        let after = source.as_bytes().get(absolute + identifier.len());

        let left_ok = before.is_none_or(|byte| !is_ident_continue(*byte));
        let right_ok = after.is_none_or(|byte| !is_ident_continue(*byte));
        if left_ok && right_ok {
            count += 1;
        }
        start = absolute + identifier.len();
    }

    count
}

/// Text-only fallback helper.
/// Correctness/soundness rules must not use this as a primary signal.
pub fn text_fallback_line_bindings(line: &str) -> Vec<(String, usize)> {
    let tokens = extract_identifiers(line);
    let mut out = Vec::<(String, usize)>::new();
    let mut index = 0usize;

    while index < tokens.len() {
        if tokens[index].0 != "let" {
            index += 1;
            continue;
        }

        let mut name_index = index + 1;
        if name_index < tokens.len() && tokens[name_index].0 == "mut" {
            name_index += 1;
        }
        if name_index < tokens.len() {
            let name = tokens[name_index].0.clone();
            let offset = tokens[name_index].1;
            if name != "_" {
                out.push((name, offset));
            }
            index = name_index + 1;
            continue;
        }

        index += 1;
    }

    out
}

/// Text-only fallback helper.
/// Correctness/soundness rules must not use this as a primary signal.
pub fn text_fallback_statement_bindings(statement: &str) -> Vec<(String, usize)> {
    let Some(let_start) = find_keyword(statement, "let") else {
        return Vec::new();
    };

    let mut cursor = let_start + "let".len();
    let bytes = statement.as_bytes();
    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }

    if statement[cursor..].starts_with("mut")
        && statement
            .as_bytes()
            .get(cursor + "mut".len())
            .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        cursor += "mut".len();
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
    }

    let pattern_end = cursor
        + find_pattern_end(&statement[cursor..]).unwrap_or(statement.len().saturating_sub(cursor));
    parse_pattern_bindings(&statement[cursor..pattern_end], cursor)
}

pub fn extract_numeric_literals(line: &str) -> Vec<(String, usize)> {
    let bytes = line.as_bytes();
    let mut out = Vec::<(String, usize)>::new();
    let mut idx = 0usize;

    while idx < bytes.len() {
        if !bytes[idx].is_ascii_digit() {
            idx += 1;
            continue;
        }

        let start = idx;
        idx += 1;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
        }

        let before = start.checked_sub(1).and_then(|n| bytes.get(n));
        let after = bytes.get(idx);
        let left_ok = before.is_none_or(|byte| !is_ident_continue(*byte));
        let right_ok = after.is_none_or(|byte| !is_ident_continue(*byte));

        if left_ok && right_ok {
            out.push((line[start..idx].to_string(), start));
        }
    }

    out
}

pub fn collect_identifiers(line: &str) -> BTreeSet<String> {
    extract_identifiers(line)
        .into_iter()
        .map(|(token, _)| token)
        .collect()
}

pub fn source_slice(source: &str, start: u32, end: u32) -> Option<&str> {
    let start = usize::try_from(start).ok()?;
    let end = usize::try_from(end).ok()?;
    if start >= end || end > source.len() {
        return None;
    }
    source.get(start..end)
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

fn find_pattern_end(pattern_tail: &str) -> Option<usize> {
    let bytes = pattern_tail.as_bytes();
    let mut index = 0usize;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'{' => brace_depth += 1,
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b'=' | b';' if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                return Some(index);
            }
            b':' if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                let prev_is_colon = index > 0 && bytes[index - 1] == b':';
                let next_is_colon = bytes.get(index + 1) == Some(&b':');
                if !prev_is_colon && !next_is_colon {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }

    None
}

fn parse_pattern_bindings(pattern: &str, base_offset: usize) -> Vec<(String, usize)> {
    let bytes = pattern.as_bytes();
    let mut index = 0usize;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut out = Vec::<(String, usize)>::new();

    while index < bytes.len() {
        match bytes[index] {
            b'(' => {
                paren_depth += 1;
                index += 1;
                continue;
            }
            b')' => {
                paren_depth = paren_depth.saturating_sub(1);
                index += 1;
                continue;
            }
            b'{' => {
                brace_depth += 1;
                index += 1;
                continue;
            }
            b'}' => {
                brace_depth = brace_depth.saturating_sub(1);
                index += 1;
                continue;
            }
            b'[' => {
                bracket_depth += 1;
                index += 1;
                continue;
            }
            b']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                index += 1;
                continue;
            }
            _ => {}
        }

        if !is_ident_start(bytes[index]) {
            index += 1;
            continue;
        }

        let start = index;
        index += 1;
        while index < bytes.len() && is_ident_continue(bytes[index]) {
            index += 1;
        }

        let name = &pattern[start..index];
        if name == "_" || matches!(name, "let" | "mut" | "ref" | "pub" | "crate" | "super") {
            continue;
        }

        let preceded_by_path = start >= 2 && &bytes[start - 2..start] == b"::";
        if preceded_by_path {
            continue;
        }
        let followed_by_path = bytes.get(index..index + 2) == Some(b"::");
        if followed_by_path {
            continue;
        }

        let next_non_ws = bytes
            .get(index..)
            .and_then(|tail| {
                tail.iter()
                    .position(|byte| !byte.is_ascii_whitespace())
                    .map(|offset| index + offset)
            })
            .and_then(|position| bytes.get(position))
            .copied();

        let is_struct_or_enum_constructor = matches!(next_non_ws, Some(b'{') | Some(b'('))
            && paren_depth == 0
            && bracket_depth == 0;
        if is_struct_or_enum_constructor {
            continue;
        }

        let is_field_label =
            brace_depth > 0 && next_non_ws == Some(b':') && bytes.get(index + 1) != Some(&b':');
        if is_field_label {
            continue;
        }

        out.push((name.to_string(), base_offset + start));
    }

    out
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionScope {
    pub name: String,
    pub name_offset: usize,
    pub body_start: usize,
    pub body_end: usize,
}

/// Text-only fallback helper.
/// Correctness/soundness rules must not use this as a primary signal.
pub fn text_fallback_function_scopes(source: &str) -> Vec<FunctionScope> {
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut scopes = Vec::<FunctionScope>::new();

    while index + 2 < bytes.len() {
        if &bytes[index..index + 3] != b"fn " {
            index += 1;
            continue;
        }
        let left_is_ident = index
            .checked_sub(1)
            .and_then(|left| bytes.get(left))
            .is_some_and(|byte| is_ident_continue(*byte));
        if left_is_ident {
            index += 1;
            continue;
        }

        let mut cursor = index + 3;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || !is_ident_start(bytes[cursor]) {
            index += 1;
            continue;
        }

        let name_start = cursor;
        cursor += 1;
        while cursor < bytes.len() && is_ident_continue(bytes[cursor]) {
            cursor += 1;
        }
        let name = source[name_start..cursor].to_string();

        let Some(open_rel) = source[cursor..].find('{') else {
            index = cursor;
            continue;
        };
        let body_start = cursor + open_rel;
        let body_end = matching_brace_end(source, body_start).unwrap_or(source.len());
        scopes.push(FunctionScope {
            name,
            name_offset: name_start,
            body_start,
            body_end,
        });
        index = body_end;
    }

    scopes
}

fn matching_brace_end(source: &str, open_index: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut cursor = open_index;

    while cursor < bytes.len() {
        match bytes[cursor] {
            b'{' => depth += 1,
            b'}' => {
                if depth == 0 {
                    return Some(cursor + 1);
                }
                depth -= 1;
                if depth == 0 {
                    return Some(cursor + 1);
                }
            }
            _ => {}
        }
        cursor += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{
        count_identifier_occurrences, extract_numeric_literals, text_fallback_function_scopes,
        text_fallback_line_bindings, text_fallback_statement_bindings,
    };

    #[test]
    fn token_count_is_identifier_aware() {
        let source = "let foo = 1; let foobar = foo + 1;";
        assert_eq!(count_identifier_occurrences(source, "foo"), 2);
        assert_eq!(count_identifier_occurrences(source, "foobar"), 1);
    }

    #[test]
    fn finds_let_bindings() {
        let bindings = text_fallback_line_bindings("let mut value = 2; let next = value + 1;");
        assert_eq!(bindings[0].0, "value");
        assert_eq!(bindings[1].0, "next");
    }

    #[test]
    fn finds_let_bindings_in_statement_pattern() {
        let bindings = text_fallback_statement_bindings("let (left, mut right) = pair;");
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].0, "left");
        assert_eq!(bindings[1].0, "right");
    }

    #[test]
    fn finds_let_bindings_in_statement_with_type_annotation() {
        let bindings = text_fallback_statement_bindings("let value: Field = 42;");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].0, "value");
    }

    #[test]
    fn finds_let_bindings_in_struct_pattern_without_type_labels() {
        let bindings = text_fallback_statement_bindings("let Point { x, y: z } = point;");
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].0, "x");
        assert_eq!(bindings[1].0, "z");
    }

    #[test]
    fn finds_let_bindings_in_enum_pattern() {
        let bindings = text_fallback_statement_bindings("let Some(value) = maybe;");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].0, "value");
    }

    #[test]
    fn extracts_decimal_literals() {
        let literals = extract_numeric_literals("let x = 42 + y1 + 7;");
        assert_eq!(literals.len(), 2);
        assert_eq!(literals[0].0, "42");
        assert_eq!(literals[1].0, "7");
    }

    #[test]
    fn extracts_function_scopes() {
        let source = "fn main() { if true { helper(); } } fn helper() {}";
        let scopes = text_fallback_function_scopes(source);
        assert_eq!(scopes.len(), 2);
        assert_eq!(scopes[0].name, "main");
        assert_eq!(scopes[1].name, "helper");
    }
}
