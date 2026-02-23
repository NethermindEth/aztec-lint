use std::collections::BTreeSet;

use aztec_lint_aztec::patterns::{
    extract_call_name, is_contract_start, is_function_start, normalize_line,
};
use aztec_lint_core::diagnostics::normalize_file_path;
use aztec_lint_core::model::Span;

use crate::engine::context::RuleContext;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ScannedLine {
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ScannedFunction {
    pub contract_id: String,
    pub function_symbol_id: String,
    pub function_name: String,
    pub lines: Vec<ScannedLine>,
}

pub(crate) fn scan_functions(ctx: &RuleContext<'_>) -> Vec<ScannedFunction> {
    let mut functions = Vec::<ScannedFunction>::new();

    for file in ctx.files() {
        let normalized_file = normalize_file_path(file.path());
        let mut current_contract: Option<(String, usize)> = None;
        let mut current_function: Option<(ScannedFunction, usize)> = None;
        let mut brace_depth = 0usize;
        let mut offset = 0usize;

        for line in file.text().lines() {
            let trimmed = line.trim();
            let (_, code_after_attributes) = split_inline_attributes(trimmed);

            if current_contract.is_none()
                && let Some(contract_name) = is_contract_start(code_after_attributes)
            {
                let contract_id = format!("{normalized_file}::{contract_name}");
                current_contract = Some((contract_id, brace_depth + line.matches('{').count()));
            }

            if let Some((contract_id, _)) = current_contract.clone()
                && let Some(function_name) = is_function_start(code_after_attributes)
            {
                current_function = Some((
                    ScannedFunction {
                        contract_id: contract_id.clone(),
                        function_symbol_id: format!("{contract_id}::fn::{function_name}"),
                        function_name,
                        lines: Vec::new(),
                    },
                    brace_depth + line.matches('{').count(),
                ));
            }

            if let Some((function, _)) = current_function.as_mut() {
                let normalized = normalize_line(line);
                if !normalized.is_empty() {
                    function.lines.push(ScannedLine {
                        text: normalized.to_string(),
                        span: file.span_for_range(offset, offset + line.len()),
                    });
                }
            }

            brace_depth = update_depth(brace_depth, line);

            if let Some((function, end_depth)) = current_function.take() {
                if brace_depth < end_depth {
                    functions.push(function);
                } else {
                    current_function = Some((function, end_depth));
                }
            }

            if let Some((_, contract_depth)) = current_contract.clone()
                && brace_depth < contract_depth
            {
                current_contract = None;
            }

            offset += line.len() + 1;
        }

        if let Some((function, _)) = current_function.take() {
            functions.push(function);
        }
    }

    functions
}

pub(crate) fn call_name(line: &str) -> Option<String> {
    extract_call_name(line)
}

pub(crate) fn call_arguments(line: &str, call_name: &str) -> Option<String> {
    let normalized = normalize_line(line);
    let marker = format!("{call_name}(");
    let mut search_from = 0usize;
    while let Some(relative) = normalized[search_from..].find(&marker) {
        let start = search_from + relative;
        if start > 0
            && normalized[..start]
                .chars()
                .next_back()
                .is_some_and(is_ident_continue)
        {
            search_from = start + 1;
            continue;
        }
        let open = start + marker.len() - 1;
        let (arg_start, arg_end) = balanced_range(normalized, open)?;
        return Some(normalized[arg_start..arg_end].trim().to_string());
    }
    None
}

pub(crate) fn has_hash_like_call(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("hash(")
        || lower.contains("_hash(")
        || lower.contains("poseidon(")
        || lower.contains("pedersen(")
        || lower.contains("serialize(")
}

pub(crate) fn hash_like_arguments(text: &str) -> Vec<String> {
    let normalized = normalize_line(text);
    let mut args = Vec::<String>::new();

    for (open, ch) in normalized.char_indices() {
        if ch != '(' {
            continue;
        }
        let mut end = open;
        while end > 0
            && normalized
                .as_bytes()
                .get(end - 1)
                .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            end -= 1;
        }
        if end == 0 {
            continue;
        }
        let mut start = end;
        while start > 0
            && normalized
                .as_bytes()
                .get(start - 1)
                .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            start -= 1;
        }
        if start == end || !is_ident_start(char::from(normalized.as_bytes()[start])) {
            continue;
        }

        let name = &normalized[start..end];
        if !is_hash_like_call_name(name) {
            continue;
        }

        let Some((arg_start, arg_end)) = balanced_range(normalized, open) else {
            continue;
        };
        args.push(normalized[arg_start..arg_end].trim().to_string());
    }

    args
}

pub(crate) fn is_note_consume_call_name(name: &str, line: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "pop_note"
        || lower == "pop_notes"
        || lower == "consume_note"
        || lower == "consume_notes"
        || (lower == "pop" && line.to_ascii_lowercase().contains("note"))
        || (lower.contains("consume") && lower.contains("note"))
}

pub(crate) fn extract_identifiers(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let bytes = text.as_bytes();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        if !is_ident_start(char::from(bytes[cursor])) {
            cursor += 1;
            continue;
        }

        let start = cursor;
        cursor += 1;
        while cursor < bytes.len() && is_ident_continue(char::from(bytes[cursor])) {
            cursor += 1;
        }

        let candidate = &text[start..cursor];
        if !is_keyword(candidate) {
            out.insert(candidate.to_string());
        }
    }

    out
}

pub(crate) fn extract_double_at_keys(line: &str) -> Option<(String, String, usize, usize)> {
    let normalized = normalize_line(line);
    let first_start = normalized.find(".at(")?;
    let first_open = first_start + ".at".len();
    let (first_arg_start, first_arg_end) = balanced_range(normalized, first_open)?;
    let first_arg = normalized[first_arg_start..first_arg_end]
        .trim()
        .to_string();

    let next_search = first_arg_end + 1;
    let second_relative = normalized[next_search..].find(".at(")?;
    let second_start = next_search + second_relative;
    let second_open = second_start + ".at".len();
    let (second_arg_start, second_arg_end) = balanced_range(normalized, second_open)?;
    let second_arg = normalized[second_arg_start..second_arg_end]
        .trim()
        .to_string();

    Some((first_arg, second_arg, second_start, second_arg_end + 1))
}

pub(crate) fn normalize_expression(expr: &str) -> String {
    expr.chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .trim_start_matches("self.")
        .to_string()
}

fn split_inline_attributes(line: &str) -> (Vec<String>, &str) {
    let mut attrs = Vec::<String>::new();
    let mut remaining = line.trim_start();

    while remaining.starts_with("#[") {
        let Some(close) = remaining.find(']') else {
            break;
        };
        attrs.push(remaining[..=close].trim().to_string());
        remaining = remaining[close + 1..].trim_start();
    }

    (attrs, remaining)
}

fn is_hash_like_call_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "hash"
        || lower.ends_with("_hash")
        || lower.contains("poseidon")
        || lower.contains("pedersen")
        || lower == "serialize"
}

fn balanced_range(text: &str, open_index: usize) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut start = None;

    for (idx, ch) in text.char_indices().skip(open_index) {
        if idx == open_index {
            if ch != '(' {
                return None;
            }
            depth = 1;
            start = Some(idx + ch.len_utf8());
            continue;
        }

        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some((start?, idx));
                }
            }
            _ => {}
        }
    }

    None
}

fn update_depth(current: usize, line: &str) -> usize {
    let opens = line.bytes().filter(|byte| *byte == b'{').count();
    let closes = line.bytes().filter(|byte| *byte == b'}').count();
    current.saturating_add(opens).saturating_sub(closes)
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_keyword(token: &str) -> bool {
    matches!(
        token,
        "let"
            | "mut"
            | "fn"
            | "if"
            | "while"
            | "match"
            | "for"
            | "return"
            | "assert"
            | "constrain"
            | "self"
            | "pub"
            | "contract"
            | "in"
            | "true"
            | "false"
    )
}
