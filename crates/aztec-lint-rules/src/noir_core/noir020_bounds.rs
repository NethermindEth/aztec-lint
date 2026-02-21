use std::collections::{BTreeMap, BTreeSet};

use aztec_lint_core::diagnostics::{Diagnostic, normalize_file_path};
use aztec_lint_core::model::{
    ExpressionCategory, GuardKind, SemanticExpression, SemanticFunction, SemanticStatement,
    TypeCategory,
};
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::engine::context::{RuleContext, SourceFile};
use crate::noir_core::util::{
    collect_identifiers, extract_identifiers, is_ident_continue, source_slice,
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
        let report_unknown = report_unknown_bounds_diagnostics();
        let functions_by_id = semantic
            .functions
            .iter()
            .map(|function| (function.symbol_id.clone(), function))
            .collect::<BTreeMap<_, _>>();
        let expressions_by_function = semantic.expressions.iter().fold(
            BTreeMap::<String, Vec<&SemanticExpression>>::new(),
            |mut acc, expression| {
                acc.entry(expression.function_symbol_id.clone())
                    .or_default()
                    .push(expression);
                acc
            },
        );
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

        let mut analyses_by_function = BTreeMap::<String, FunctionAnalysis>::new();
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
            let Some(access) = parse_index_access(source) else {
                continue;
            };
            let Some(expression_start) = usize::try_from(expression.span.start).ok() else {
                continue;
            };

            if !analyses_by_function.contains_key(&expression.function_symbol_id) {
                let analysis = functions_by_id
                    .get(&expression.function_symbol_id)
                    .map(|function| build_function_analysis(file.text(), function))
                    .unwrap_or_default();
                analyses_by_function.insert(expression.function_symbol_id.clone(), analysis);
            }
            let Some(function_analysis) = analyses_by_function.get(&expression.function_symbol_id)
            else {
                continue;
            };

            let semantic_array_len = semantic_array_length_for_index(
                &expressions_by_function,
                expression,
                &normalized_file,
                expression_start.saturating_add(access.open_rel),
            );
            let bounds_proof = prove_local_bounds(
                function_analysis,
                &access,
                expression_start,
                semantic_array_len,
            );
            if bounds_proof == BoundsProof::Safe
                || (bounds_proof == BoundsProof::Unknown && !report_unknown)
            {
                continue;
            }

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
                    let index_identifiers = collect_identifiers(&access.index_expr);
                    guards.iter().any(|guard| {
                        guard.identifiers.contains(&access.base_name)
                            && !index_identifiers.is_empty()
                            && index_identifiers
                                .iter()
                                .all(|identifier| guard.identifiers.contains(identifier))
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

            let start = expression_start.saturating_add(access.index_rel_start);
            let span = file.span_for_range(start, start + access.index_expr.len());
            let diagnostic = match bounds_proof {
                BoundsProof::Unsafe => ctx.diagnostic(
                    self.id(),
                    CORRECTNESS,
                    format!(
                        "index `{}` is provably out of bounds for `{}`",
                        access.index_expr, access.base_name
                    ),
                    span,
                ),
                BoundsProof::Unknown => ctx
                    .diagnostic(
                        self.id(),
                        CORRECTNESS,
                        format!(
                            "index `{}` could not be locally proven in bounds",
                            access.index_expr
                        ),
                        span,
                    )
                    .note(
                        "local proof confidence is low; this diagnostic is opt-in via AZTEC_LINT_NOIR020_REPORT_UNKNOWN=1",
                    ),
                BoundsProof::Safe => continue,
            };
            out.push(diagnostic);
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Interval {
    min: i128,
    max: i128,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BoundsProof {
    Safe,
    Unsafe,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum AffineIdx {
    Const(i128),
    Var { name: String, offset: i128 },
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct IndexAccess {
    base_name: String,
    index_expr: String,
    index_rel_start: usize,
    open_rel: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LoopBinding {
    var_name: String,
    interval: Interval,
    body_open: usize,
    body_close: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ArrayLenBinding {
    name: String,
    len: i128,
    decl_start: usize,
    scope_end: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct FunctionAnalysis {
    loop_ranges: Vec<LoopBinding>,
    array_lens: Vec<ArrayLenBinding>,
}

impl FunctionAnalysis {
    fn interval_for_var(&self, name: &str, position: usize) -> Option<Interval> {
        self.loop_ranges
            .iter()
            .filter(|loop_binding| {
                loop_binding.var_name == name
                    && loop_binding.body_open < position
                    && position < loop_binding.body_close
            })
            .min_by_key(|loop_binding| {
                loop_binding
                    .body_close
                    .saturating_sub(loop_binding.body_open)
            })
            .map(|loop_binding| loop_binding.interval)
    }

    fn array_len_at(&self, name: &str, position: usize) -> Option<i128> {
        self.array_lens
            .iter()
            .filter(|binding| {
                binding.name == name
                    && binding.decl_start <= position
                    && position < binding.scope_end
            })
            .max_by_key(|binding| binding.decl_start)
            .map(|binding| binding.len)
    }
}

fn parse_index_access(expression: &str) -> Option<IndexAccess> {
    let open = expression.find('[')?;
    let close_rel = expression[open + 1..].find(']')?;
    let close = open + 1 + close_rel;
    let base = expression[..open].trim();
    let base_name = extract_identifiers(base).into_iter().last()?.0;
    let inner = &expression[open + 1..close];
    let index_expr = inner.trim();
    if index_expr.is_empty() {
        return None;
    }
    let first_non_ws = inner
        .char_indices()
        .find_map(|(idx, ch)| (!ch.is_ascii_whitespace()).then_some(idx))
        .unwrap_or(0);
    Some(IndexAccess {
        base_name,
        index_expr: index_expr.to_string(),
        index_rel_start: open + 1 + first_non_ws,
        open_rel: open,
    })
}

fn prove_local_bounds(
    analysis: &FunctionAnalysis,
    access: &IndexAccess,
    expression_start: usize,
    semantic_array_len: Option<i128>,
) -> BoundsProof {
    let array_len = analysis
        .array_len_at(&access.base_name, expression_start)
        .or(semantic_array_len);
    let Some(array_len) = array_len else {
        return BoundsProof::Unknown;
    };
    if array_len <= 0 {
        return BoundsProof::Unsafe;
    }

    let affine = eval_affine_index(&access.index_expr);
    let Some(index_interval) = interval_of_affine(&affine, analysis, expression_start) else {
        return BoundsProof::Unknown;
    };
    if index_interval.min >= 0 && index_interval.max < array_len {
        BoundsProof::Safe
    } else {
        BoundsProof::Unsafe
    }
}

fn interval_of_affine(
    affine: &AffineIdx,
    analysis: &FunctionAnalysis,
    expression_start: usize,
) -> Option<Interval> {
    match affine {
        AffineIdx::Const(value) => Some(Interval {
            min: *value,
            max: *value,
        }),
        AffineIdx::Var { name, offset } => {
            let loop_interval = analysis.interval_for_var(name, expression_start)?;
            let min = loop_interval.min.checked_add(*offset)?;
            let max = loop_interval.max.checked_add(*offset)?;
            Some(Interval { min, max })
        }
        AffineIdx::Unknown => None,
    }
}

fn eval_affine_index(expression: &str) -> AffineIdx {
    let normalized = strip_wrapping_parentheses(expression.trim());
    if let Some(value) = parse_int_literal(normalized) {
        return AffineIdx::Const(value);
    }
    if is_identifier(normalized) {
        return AffineIdx::Var {
            name: normalized.to_string(),
            offset: 0,
        };
    }
    let Some((left, op, right)) = split_top_level_add_sub(normalized) else {
        return AffineIdx::Unknown;
    };
    let left = strip_wrapping_parentheses(left.trim());
    let right = strip_wrapping_parentheses(right.trim());

    if is_identifier(left)
        && let Some(constant) = parse_int_literal(right)
    {
        return AffineIdx::Var {
            name: left.to_string(),
            offset: if op == '-' { -constant } else { constant },
        };
    }
    if op == '+'
        && is_identifier(right)
        && let Some(constant) = parse_int_literal(left)
    {
        return AffineIdx::Var {
            name: right.to_string(),
            offset: constant,
        };
    }

    AffineIdx::Unknown
}

fn split_top_level_add_sub(expression: &str) -> Option<(&str, char, &str)> {
    let bytes = expression.as_bytes();
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b'{' => brace_depth += 1,
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            b'+' | b'-' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                let before = expression[..index].trim_end();
                let after = expression[index + 1..].trim_start();
                if before.is_empty() || after.is_empty() {
                    index += 1;
                    continue;
                }
                let before_tail = before.as_bytes().last().copied().unwrap_or_default();
                if matches!(
                    before_tail,
                    b'+' | b'-' | b'*' | b'/' | b'(' | b'[' | b'{' | b','
                ) {
                    index += 1;
                    continue;
                }
                return Some((
                    &expression[..index],
                    bytes[index] as char,
                    &expression[index + 1..],
                ));
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn strip_wrapping_parentheses(mut expression: &str) -> &str {
    loop {
        let trimmed = expression.trim();
        if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
            return trimmed;
        }
        let Some(close) = find_matching_delimiter(trimmed, 0, b'(', b')') else {
            return trimmed;
        };
        if close != trimmed.len().saturating_sub(1) {
            return trimmed;
        }
        expression = &trimmed[1..close];
    }
}

fn is_identifier(value: &str) -> bool {
    let tokens = extract_identifiers(value);
    tokens.len() == 1 && tokens[0].1 == 0 && tokens[0].0.len() == value.len()
}

fn parse_int_literal(value: &str) -> Option<i128> {
    let trimmed = strip_wrapping_parentheses(value.trim());
    if trimmed.is_empty() {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix('-') {
        return parse_digits_i128(rest).and_then(|value| value.checked_neg());
    }
    if let Some(rest) = trimmed.strip_prefix('+') {
        return parse_digits_i128(rest);
    }
    parse_digits_i128(trimmed)
}

fn parse_digits_i128(value: &str) -> Option<i128> {
    let compact = value.chars().filter(|ch| *ch != '_').collect::<String>();
    if compact.is_empty() || !compact.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    compact.parse::<i128>().ok()
}

fn semantic_array_length_for_index(
    expressions_by_function: &BTreeMap<String, Vec<&SemanticExpression>>,
    index_expression: &SemanticExpression,
    normalized_file: &str,
    open_offset: usize,
) -> Option<i128> {
    let open_offset = u32::try_from(open_offset).ok()?;
    expressions_by_function
        .get(&index_expression.function_symbol_id)?
        .iter()
        .filter(|candidate| candidate.type_category == TypeCategory::Array)
        .filter(|candidate| normalize_file_path(&candidate.span.file) == normalized_file)
        .filter(|candidate| index_expression.span.start <= candidate.span.start)
        .filter(|candidate| candidate.span.end <= index_expression.span.end)
        .filter(|candidate| candidate.span.end <= open_offset)
        .max_by_key(|candidate| candidate.span.start)
        .and_then(|candidate| parse_array_len_from_type_repr(&candidate.type_repr))
}

fn build_function_analysis(source: &str, function: &SemanticFunction) -> FunctionAnalysis {
    let Some(function_start) = usize::try_from(function.span.start).ok() else {
        return FunctionAnalysis::default();
    };
    if function_start >= source.len() {
        return FunctionAnalysis::default();
    }
    let Some(function_tail) = source.get(function_start..) else {
        return FunctionAnalysis::default();
    };
    // Semantic function spans can be narrow (for example only the function identifier),
    // so derive the body directly from source starting at span.start.
    let Some(body_open_rel) = function_tail.find('{') else {
        return FunctionAnalysis::default();
    };
    let body_open = function_start + body_open_rel;
    let body_close = matching_brace_end(source, body_open).unwrap_or(source.len());
    if body_close <= body_open {
        return FunctionAnalysis::default();
    }

    let mut analysis = FunctionAnalysis::default();
    collect_parameter_array_bindings(
        source,
        function_start,
        body_open,
        body_close,
        &mut analysis.array_lens,
    );
    collect_local_array_bindings(source, body_open, body_close, &mut analysis.array_lens);
    collect_loop_ranges(source, body_open, body_close, &mut analysis.loop_ranges);
    analysis
}

fn collect_parameter_array_bindings(
    source: &str,
    function_start: usize,
    body_open: usize,
    body_close: usize,
    out: &mut Vec<ArrayLenBinding>,
) {
    let Some(signature) = source.get(function_start..body_open) else {
        return;
    };
    let Some(open_rel) = signature.find('(') else {
        return;
    };
    let open = function_start + open_rel;
    let Some(close) = find_matching_delimiter(source, open, b'(', b')') else {
        return;
    };
    if close <= open + 1 {
        return;
    }

    let Some(params) = source.get(open + 1..close) else {
        return;
    };
    for param in split_top_level(params, b',') {
        let param = param.trim();
        if param.is_empty() {
            continue;
        }
        let Some(colon) = find_top_level_char(param, b':') else {
            continue;
        };
        let Some(name_part) = param.get(..colon) else {
            continue;
        };
        let Some(type_part) = param.get(colon + 1..) else {
            continue;
        };
        let Some(len) = parse_array_len_from_bracketed_type(type_part) else {
            continue;
        };
        let Some(name) = extract_identifiers(name_part)
            .into_iter()
            .last()
            .map(|token| token.0)
        else {
            continue;
        };
        out.push(ArrayLenBinding {
            name,
            len,
            decl_start: function_start,
            scope_end: body_close,
        });
    }
}

fn collect_local_array_bindings(
    source: &str,
    body_open: usize,
    body_close: usize,
    out: &mut Vec<ArrayLenBinding>,
) {
    let mut index = body_open.saturating_add(1);
    while index < body_close {
        if keyword_at(source, index, "let") {
            let Some(statement_end) = find_statement_end(source, index, body_close) else {
                index += 1;
                continue;
            };
            if let Some(statement) = source.get(index..statement_end)
                && let Some((name, len)) = parse_let_array_binding(statement)
            {
                out.push(ArrayLenBinding {
                    name,
                    len,
                    decl_start: index,
                    scope_end: body_close,
                });
            }
            index = statement_end;
            continue;
        }
        index += 1;
    }
}

fn collect_loop_ranges(
    source: &str,
    body_open: usize,
    body_close: usize,
    out: &mut Vec<LoopBinding>,
) {
    let mut index = body_open.saturating_add(1);
    while index < body_close {
        if !keyword_at(source, index, "for") {
            index += 1;
            continue;
        }

        let mut cursor = skip_whitespace(source, index + "for".len(), body_close);
        let Some((loop_var, next_cursor)) = parse_identifier_at(source, cursor, body_close) else {
            index += 1;
            continue;
        };
        cursor = skip_whitespace(source, next_cursor, body_close);
        if !keyword_at(source, cursor, "in") {
            index += 1;
            continue;
        }
        cursor = skip_whitespace(source, cursor + "in".len(), body_close);

        let mut header_end = cursor;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        while header_end < body_close {
            let byte = source.as_bytes()[header_end];
            match byte {
                b'(' => paren_depth += 1,
                b')' => paren_depth = paren_depth.saturating_sub(1),
                b'[' => bracket_depth += 1,
                b']' => bracket_depth = bracket_depth.saturating_sub(1),
                b'{' if paren_depth == 0 && bracket_depth == 0 => break,
                _ => {}
            }
            header_end += 1;
        }
        if header_end >= body_close || source.as_bytes()[header_end] != b'{' {
            index += 1;
            continue;
        }
        let range_expr = source
            .get(cursor..header_end)
            .map(str::trim)
            .unwrap_or_default();
        let Some(interval) = parse_range_interval(range_expr) else {
            index += 1;
            continue;
        };
        let Some(loop_close) = matching_brace_end(source, header_end) else {
            index += 1;
            continue;
        };
        out.push(LoopBinding {
            var_name: loop_var,
            interval,
            body_open: header_end,
            body_close: loop_close.min(body_close),
        });
        index = header_end.saturating_add(1);
    }
}

fn parse_let_array_binding(statement: &str) -> Option<(String, i128)> {
    if !statement.starts_with("let") {
        return None;
    }
    let mut cursor = "let".len();
    cursor = skip_whitespace(statement, cursor, statement.len());
    if keyword_at(statement, cursor, "mut") {
        cursor = skip_whitespace(statement, cursor + "mut".len(), statement.len());
    }
    let (name, next_cursor) = parse_identifier_at(statement, cursor, statement.len())?;
    let tail = statement.get(next_cursor..)?.trim_start();

    let eq_index = find_top_level_char(tail, b'=');
    if let Some(eq_index) = eq_index {
        let lhs = tail.get(..eq_index)?.trim();
        if let Some(colon_index) = find_top_level_char(lhs, b':') {
            let type_part = lhs.get(colon_index + 1..)?.trim();
            if let Some(len) = parse_array_len_from_bracketed_type(type_part) {
                return Some((name, len));
            }
        }
        let rhs = tail
            .get(eq_index + 1..)?
            .trim()
            .trim_end_matches(';')
            .trim();
        if let Some(len) = parse_array_len_from_bracketed_type(rhs) {
            return Some((name, len));
        }
        return None;
    }

    let colon_index = find_top_level_char(tail, b':')?;
    let type_part = tail
        .get(colon_index + 1..)?
        .trim()
        .trim_end_matches(';')
        .trim();
    parse_array_len_from_bracketed_type(type_part).map(|len| (name, len))
}

fn parse_range_interval(range_expr: &str) -> Option<Interval> {
    let range_expr = strip_wrapping_parentheses(range_expr.trim());
    let bytes = range_expr.as_bytes();
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut index = 0usize;

    while index + 1 < bytes.len() {
        match bytes[index] {
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b'{' => brace_depth += 1,
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            b'.' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                if bytes.get(index + 1) != Some(&b'.') {
                    index += 1;
                    continue;
                }
                let inclusive = bytes.get(index + 2) == Some(&b'=');
                let left = range_expr.get(..index)?.trim();
                let right_start = if inclusive { index + 3 } else { index + 2 };
                let right = range_expr.get(right_start..)?.trim();
                let start = parse_int_literal(left)?;
                let end = parse_int_literal(right)?;
                if inclusive {
                    if end < start {
                        return Some(Interval { min: 0, max: -1 });
                    }
                    return Some(Interval {
                        min: start,
                        max: end,
                    });
                }
                if end <= start {
                    return Some(Interval { min: 0, max: -1 });
                }
                return Some(Interval {
                    min: start,
                    max: end.saturating_sub(1),
                });
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn split_top_level(input: &str, delimiter: u8) -> Vec<&str> {
    let bytes = input.as_bytes();
    let mut out = Vec::<&str>::new();
    let mut start = 0usize;
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
            b if b == delimiter && paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                if let Some(segment) = input.get(start..index) {
                    out.push(segment);
                }
                start = index + 1;
            }
            _ => {}
        }
    }
    if let Some(last) = input.get(start..) {
        out.push(last);
    }
    out
}

fn parse_array_len_from_type_repr(type_repr: &str) -> Option<i128> {
    parse_array_len_from_bracketed_type(type_repr.trim())
}

fn parse_array_len_from_bracketed_type(value: &str) -> Option<i128> {
    let trimmed = value.trim();
    if !trimmed.starts_with('[') {
        return None;
    }
    let close = find_matching_delimiter(trimmed, 0, b'[', b']')?;
    let inner = trimmed.get(1..close)?.trim();
    let semi = find_top_level_char(inner, b';')?;
    parse_int_literal(inner.get(semi + 1..)?.trim())
}

fn keyword_at(source: &str, index: usize, keyword: &str) -> bool {
    let bytes = source.as_bytes();
    let keyword_bytes = keyword.as_bytes();
    if index + keyword_bytes.len() > bytes.len() {
        return false;
    }
    if &bytes[index..index + keyword_bytes.len()] != keyword_bytes {
        return false;
    }
    let left_ok = index == 0 || !is_ident_continue(bytes[index - 1]);
    let right_ok = bytes
        .get(index + keyword_bytes.len())
        .is_none_or(|byte| !is_ident_continue(*byte));
    left_ok && right_ok
}

fn skip_whitespace(source: &str, mut index: usize, limit: usize) -> usize {
    let bytes = source.as_bytes();
    while index < limit
        && bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        index += 1;
    }
    index
}

fn parse_identifier_at(source: &str, index: usize, limit: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    let first = bytes.get(index).copied()?;
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    let mut cursor = index + 1;
    while cursor < limit
        && bytes
            .get(cursor)
            .is_some_and(|byte| is_ident_continue(*byte))
    {
        cursor += 1;
    }
    Some((source.get(index..cursor)?.to_string(), cursor))
}

fn find_statement_end(source: &str, start: usize, limit: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut cursor = start;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    while cursor < limit {
        match bytes[cursor] {
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b'{' => brace_depth += 1,
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            b';' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                return Some(cursor + 1);
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn find_matching_delimiter(source: &str, open_index: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = source.as_bytes();
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

fn matching_brace_end(source: &str, open_index: usize) -> Option<usize> {
    find_matching_delimiter(source, open_index, b'{', b'}').map(|close| close + 1)
}

fn find_top_level_char(input: &str, needle: u8) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b'{' => brace_depth += 1,
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            byte if byte == needle
                && paren_depth == 0
                && bracket_depth == 0
                && brace_depth == 0 =>
            {
                return Some(index);
            }
            _ => {}
        }
        index += 1;
    }
    None
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

fn report_unknown_bounds_diagnostics() -> bool {
    std::env::var("AZTEC_LINT_NOIR020_REPORT_UNKNOWN")
        .ok()
        .as_deref()
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
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
    fn semantic_unbounded_indexing_is_non_reporting_without_proof() {
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

        assert!(diagnostics.is_empty());
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
    fn semantic_guard_after_index_remains_non_reporting_without_proof() {
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

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_guard_for_other_collection_is_non_reporting_without_proof() {
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

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_local_loop_bounds_prove_copy_is_safe() {
        let source = "fn main() { let mut a: [Field; 32] = [0; 32]; let b: [Field; 32] = [0; 32]; for i in 0..32 { a[i] = b[i]; } }";
        let (function_start, function_end) = span_range(source, source);
        let (left_index_start, left_index_end) = span_range(source, "a[i]");
        let (right_index_start, right_index_end) = span_range(source, "b[i]");

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
            expr_id: "expr::left_index".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Index,
            type_category: TypeCategory::Field,
            type_repr: "Field".to_string(),
            span: Span::new("src/main.nr", left_index_start, left_index_end, 1, 1),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::right_index".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Index,
            type_category: TypeCategory::Field,
            type_repr: "Field".to_string(),
            span: Span::new("src/main.nr", right_index_start, right_index_end, 1, 1),
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
    fn semantic_local_loop_bounds_prove_offset_index_is_safe() {
        let source = "fn main() { let mut a: [Field; 64] = [0; 64]; let b: [Field; 32] = [0; 32]; for i in 0..32 { a[32 + i] = b[i]; } }";
        let (function_start, function_end) = span_range(source, source);
        let (left_index_start, left_index_end) = span_range(source, "a[32 + i]");
        let (right_index_start, right_index_end) = span_range(source, "b[i]");

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
            expr_id: "expr::left_index".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Index,
            type_category: TypeCategory::Field,
            type_repr: "Field".to_string(),
            span: Span::new("src/main.nr", left_index_start, left_index_end, 1, 1),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::right_index".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Index,
            type_category: TypeCategory::Field,
            type_repr: "Field".to_string(),
            span: Span::new("src/main.nr", right_index_start, right_index_end, 1, 1),
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
    fn semantic_reports_proven_out_of_bounds_offset() {
        let source = "fn main() { let mut a: [Field; 64] = [0; 64]; for i in 0..33 { let value = a[32 + i]; } }";
        let (function_start, function_end) = span_range(source, source);
        let (index_start, index_end) = span_range(source, "a[32 + i]");

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
        assert!(diagnostics[0].message.contains("provably out of bounds"));
    }

    #[test]
    fn semantic_unknown_index_expression_is_non_reporting_by_default() {
        let source =
            "fn main() { let a: [Field; 64] = [0; 64]; for i in 0..32 { let value = a[f(i)]; } }";
        let (function_start, function_end) = span_range(source, source);
        let (index_start, index_end) = span_range(source, "a[f(i)]");

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

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn semantic_uses_base_array_type_information_when_source_lacks_annotation() {
        let source = "fn main() { let bytes = helper(); for i in 0..32 { let value = bytes[i]; } }";
        let (function_start, function_end) = span_range(source, source);
        let (index_start, index_end) = span_range(source, "bytes[i]");
        let base_start = index_start;
        let base_end = base_start + 5;

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
            expr_id: "expr::base".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Identifier,
            type_category: TypeCategory::Array,
            type_repr: "[u8; 32]".to_string(),
            span: Span::new("src/main.nr", base_start, base_end, 1, 1),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::index".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Index,
            type_category: TypeCategory::Field,
            type_repr: "u8".to_string(),
            span: Span::new("src/main.nr", index_start, index_end, 1, 1),
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
    fn semantic_loop_proof_works_with_narrow_function_span() {
        let source = "fn main() { let mut data: [u8; 192] = [0; 192]; let amount_bytes: [u8; 32] = [0; 32]; for i in 0..32 { data[64 + i] = amount_bytes[i]; } }";
        let (function_start, function_end) = span_range(source, "main");
        let (left_index_start, left_index_end) = span_range(source, "data[64 + i]");
        let (right_index_start, right_index_end) = span_range(source, "amount_bytes[i]");

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
            expr_id: "expr::left_index".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Index,
            type_category: TypeCategory::Field,
            type_repr: "u8".to_string(),
            span: Span::new("src/main.nr", left_index_start, left_index_end, 1, 1),
        });
        project.semantic.expressions.push(SemanticExpression {
            expr_id: "expr::right_index".to_string(),
            function_symbol_id: "fn::main".to_string(),
            category: ExpressionCategory::Index,
            type_category: TypeCategory::Field,
            type_repr: "u8".to_string(),
            span: Span::new("src/main.nr", right_index_start, right_index_end, 1, 1),
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

    fn span_range(source: &str, needle: &str) -> (u32, u32) {
        let start = source.find(needle).expect("needle should exist");
        let end = start + needle.len();
        (
            u32::try_from(start).unwrap_or(u32::MAX),
            u32::try_from(end).unwrap_or(u32::MAX),
        )
    }
}
