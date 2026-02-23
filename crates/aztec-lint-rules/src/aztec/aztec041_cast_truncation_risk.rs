use std::collections::BTreeSet;

use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::aztec::aztec034_hash_input_not_range_constrained::looks_like_range_guard;
use crate::aztec::text_scan::{extract_identifiers, scan_functions};
use crate::engine::context::RuleContext;

pub struct Aztec041CastTruncationRiskRule;

impl Rule for Aztec041CastTruncationRiskRule {
    fn id(&self) -> &'static str {
        "AZTEC041"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(_model) = ctx.aztec_model() else {
            return;
        };

        for function in scan_functions(ctx) {
            let mut guarded = BTreeSet::<String>::new();
            let mut field_vars = BTreeSet::<String>::new();
            let mut integer_vars = BTreeSet::<String>::new();
            let mut reported = false;

            for line in &function.lines {
                collect_typed_vars(&line.text, &mut field_vars, &mut integer_vars);

                if let Some(binding_name) = extract_let_binding_name(&line.text) {
                    let cast_sources = extract_cast_sources(&line.text);
                    if looks_like_safe_conversion_helper(&line.text)
                        || cast_sources.iter().any(|source| guarded.contains(source))
                    {
                        guarded.insert(binding_name);
                    }
                }

                if looks_like_range_guard(&line.text) {
                    guarded.extend(extract_identifiers(&line.text));
                    continue;
                }
                if looks_like_safe_conversion_helper(&line.text) {
                    continue;
                }

                let risky_field_to_int = extract_field_to_integer_cast_sources(&line.text)
                    .into_iter()
                    .find(|source| field_vars.contains(source) && !guarded.contains(source));
                if let Some(source) = risky_field_to_int {
                    out.push(ctx.diagnostic(
                        self.id(),
                        CORRECTNESS,
                        format!(
                            "cast from Field value `{source}` to integer appears unguarded and may truncate"
                        ),
                        line.span.clone(),
                    ));
                    reported = true;
                    break;
                }

                let risky_int_to_field = extract_integer_to_field_cast_sources(&line.text)
                    .into_iter()
                    .find(|source| integer_vars.contains(source) && !guarded.contains(source));
                if let Some(source) = risky_int_to_field {
                    out.push(ctx.diagnostic(
                        self.id(),
                        CORRECTNESS,
                        format!(
                            "cast from integer value `{source}` to Field appears unguarded and may alias unexpectedly"
                        ),
                        line.span.clone(),
                    ));
                    reported = true;
                    break;
                }
            }

            if reported {
                continue;
            }
        }
    }
}

fn collect_typed_vars(
    line: &str,
    field_vars: &mut BTreeSet<String>,
    integer_vars: &mut BTreeSet<String>,
) {
    for (name, ty) in extract_typed_bindings(line) {
        if ty == "Field" {
            field_vars.insert(name);
        } else if is_integer_type(&ty) {
            integer_vars.insert(name);
        }
    }

    if let Some(name) = extract_let_binding_name(line) {
        if line_casts_to_field(line) {
            field_vars.insert(name.clone());
        }
        if line_casts_to_integer(line) {
            integer_vars.insert(name);
        }
    }
}

fn extract_typed_bindings(line: &str) -> Vec<(String, String)> {
    let mut out = Vec::<(String, String)>::new();
    let bytes = line.as_bytes();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        if bytes[cursor] != b':' {
            cursor += 1;
            continue;
        }

        let mut left = cursor;
        while left > 0 && bytes[left - 1].is_ascii_whitespace() {
            left -= 1;
        }
        let mut name_start = left;
        while name_start > 0
            && (bytes[name_start - 1].is_ascii_alphanumeric() || bytes[name_start - 1] == b'_')
        {
            name_start -= 1;
        }
        if name_start == left {
            cursor += 1;
            continue;
        }
        let name = line[name_start..left].trim();
        if name.is_empty() {
            cursor += 1;
            continue;
        }

        let mut right = cursor + 1;
        while right < bytes.len() && bytes[right].is_ascii_whitespace() {
            right += 1;
        }
        let type_start = right;
        while right < bytes.len() && (bytes[right].is_ascii_alphanumeric() || bytes[right] == b'_')
        {
            right += 1;
        }
        if type_start == right {
            cursor += 1;
            continue;
        }

        let ty = line[type_start..right].trim().to_string();
        if !ty.is_empty() {
            out.push((name.to_string(), ty));
        }
        cursor = right;
    }

    out
}

fn extract_field_to_integer_cast_sources(line: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let bytes = line.as_bytes();
    let mut cursor = 0usize;

    while let Some(relative) = line[cursor..].find(" as ") {
        let as_start = cursor + relative;
        let mut ty_start = as_start + 4;
        while ty_start < bytes.len() && bytes[ty_start].is_ascii_whitespace() {
            ty_start += 1;
        }
        let mut ty_end = ty_start;
        while ty_end < bytes.len()
            && (bytes[ty_end].is_ascii_alphanumeric() || bytes[ty_end] == b'_')
        {
            ty_end += 1;
        }
        if ty_start < ty_end {
            let target = &line[ty_start..ty_end];
            if is_integer_type(target)
                && let Some(source) = parse_ident_before(line, as_start)
            {
                out.insert(source);
            }
        }
        cursor = as_start + 4;
    }

    out
}

fn extract_integer_to_field_cast_sources(line: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let bytes = line.as_bytes();
    let mut cursor = 0usize;

    while let Some(relative) = line[cursor..].find(" as ") {
        let as_start = cursor + relative;
        let mut ty_start = as_start + 4;
        while ty_start < bytes.len() && bytes[ty_start].is_ascii_whitespace() {
            ty_start += 1;
        }
        let mut ty_end = ty_start;
        while ty_end < bytes.len()
            && (bytes[ty_end].is_ascii_alphanumeric() || bytes[ty_end] == b'_')
        {
            ty_end += 1;
        }
        if ty_start < ty_end {
            let target = &line[ty_start..ty_end];
            if target == "Field"
                && let Some(source) = parse_ident_before(line, as_start)
            {
                out.insert(source);
            }
        }
        cursor = as_start + 4;
    }

    out
}

fn parse_ident_before(line: &str, index: usize) -> Option<String> {
    let bytes = line.as_bytes();
    if index == 0 {
        return None;
    }

    let mut end = index;
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }
    if start == end {
        return None;
    }
    Some(line[start..end].to_string())
}

fn extract_let_binding_name(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let body = trimmed.strip_prefix("let ")?;
    let body = body.strip_prefix("mut ").unwrap_or(body);
    let bytes = body.as_bytes();
    let mut end = 0usize;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }
    if end == 0 {
        return None;
    }
    Some(body[..end].to_string())
}

fn line_casts_to_field(line: &str) -> bool {
    extract_integer_to_field_cast_sources(line)
        .into_iter()
        .next()
        .is_some()
}

fn line_casts_to_integer(line: &str) -> bool {
    extract_field_to_integer_cast_sources(line)
        .into_iter()
        .next()
        .is_some()
}

fn extract_cast_sources(line: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let bytes = line.as_bytes();
    let mut cursor = 0usize;

    while let Some(relative) = line[cursor..].find(" as ") {
        let as_start = cursor + relative;
        if let Some(source) = parse_ident_before(line, as_start) {
            out.insert(source);
        }
        cursor = as_start + 4;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
    }

    out
}

fn is_integer_type(ty: &str) -> bool {
    if matches!(ty, "usize" | "isize") {
        return true;
    }
    if ty.len() < 2 {
        return false;
    }
    let mut chars = ty.chars();
    let Some(prefix) = chars.next() else {
        return false;
    };
    if prefix != 'u' && prefix != 'i' {
        return false;
    }
    chars.all(|ch| ch.is_ascii_digit())
}

fn looks_like_safe_conversion_helper(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("checked_to_")
        || lower.contains("checked_from_")
        || lower.contains("try_from_field")
        || lower.contains("safe_cast")
        || lower.contains("saturating_cast")
}

#[cfg(test)]
mod tests {
    use aztec_lint_aztec::build_aztec_model;
    use aztec_lint_aztec::detect::SourceUnit;
    use aztec_lint_core::config::AztecConfig;
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Aztec041CastTruncationRiskRule;

    #[test]
    fn reports_unguarded_field_to_integer_cast() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn settle(raw: Field) {
        let narrowed = raw as u128;
        emit(narrowed as Field);
    }
}
"#;
        let project = ProjectModel::default();
        let mut context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let model = build_aztec_model(
            &[SourceUnit::new("src/main.nr", source)],
            &AztecConfig::default(),
        );
        context.set_aztec_model(model);

        let mut diagnostics = Vec::new();
        Aztec041CastTruncationRiskRule.run(&context, &mut diagnostics);
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn ignores_guarded_casts() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn settle(raw: Field, amount: u256) {
        assert(raw < MAX_U128);
        let narrowed = raw as u128;
        assert(amount < MAX_FIELD_SAFE);
        let canonical = amount as Field;
        emit(canonical);
    }
}
"#;
        let project = ProjectModel::default();
        let mut context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let model = build_aztec_model(
            &[SourceUnit::new("src/main.nr", source)],
            &AztecConfig::default(),
        );
        context.set_aztec_model(model);

        let mut diagnostics = Vec::new();
        Aztec041CastTruncationRiskRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_checked_helper_paths() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn settle(raw: Field, amount: u256) {
        let narrowed = checked_to_u128(raw as u128);
        let canonical = checked_to_field(amount as Field);
        emit(narrowed as Field + canonical);
    }
}
"#;
        let project = ProjectModel::default();
        let mut context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let model = build_aztec_model(
            &[SourceUnit::new("src/main.nr", source)],
            &AztecConfig::default(),
        );
        context.set_aztec_model(model);

        let mut diagnostics = Vec::new();
        Aztec041CastTruncationRiskRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_unguarded_integer_to_field_cast_from_inferred_integer_binding() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn settle(amount: u256) {
        let narrowed = amount as u128;
        let canonical = narrowed as Field;
        emit(canonical);
    }
}
"#;
        let project = ProjectModel::default();
        let mut context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        let model = build_aztec_model(
            &[SourceUnit::new("src/main.nr", source)],
            &AztecConfig::default(),
        );
        context.set_aztec_model(model);

        let mut diagnostics = Vec::new();
        Aztec041CastTruncationRiskRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0]
                .message
                .contains("cast from integer value `narrowed` to Field"),
            "unexpected message: {}",
            diagnostics[0].message
        );
    }
}
