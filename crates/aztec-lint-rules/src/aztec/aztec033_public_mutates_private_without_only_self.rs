use std::collections::{BTreeMap, BTreeSet};

use aztec_lint_aztec::patterns::{is_contract_start, is_struct_start, normalize_line};
use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::diagnostics::normalize_file_path;
use aztec_lint_core::model::EntrypointKind;
use aztec_lint_core::policy::PROTOCOL;

use crate::Rule;
use crate::aztec::text_scan::{call_name, is_note_consume_call_name, scan_functions};
use crate::engine::context::RuleContext;

pub struct Aztec033PublicMutatesPrivateWithoutOnlySelfRule;

impl Rule for Aztec033PublicMutatesPrivateWithoutOnlySelfRule {
    fn id(&self) -> &'static str {
        "AZTEC033"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(model) = ctx.aztec_model() else {
            return;
        };

        let private_storage_fields = private_storage_fields_by_contract(ctx);
        let scanned_by_symbol = scan_functions(ctx)
            .into_iter()
            .map(|function| (function.function_symbol_id.clone(), function))
            .collect::<BTreeMap<_, _>>();

        let mut visited = BTreeSet::<String>::new();
        for entry in &model.entrypoints {
            if entry.kind != EntrypointKind::Public {
                continue;
            }
            if !visited.insert(entry.function_symbol_id.clone()) {
                continue;
            }

            let has_only_self = model.entrypoints.iter().any(|candidate| {
                candidate.contract_id == entry.contract_id
                    && candidate.function_symbol_id == entry.function_symbol_id
                    && candidate.kind == EntrypointKind::OnlySelf
            });
            if has_only_self {
                continue;
            }

            let note_write_span = model
                .note_write_sites
                .iter()
                .find(|site| site.function_symbol_id == entry.function_symbol_id)
                .map(|site| site.span.clone());
            let consume_span =
                scanned_by_symbol
                    .get(&entry.function_symbol_id)
                    .and_then(|function| {
                        function.lines.iter().find_map(|line| {
                            let is_note_consume = call_name(&line.text)
                                .is_some_and(|name| is_note_consume_call_name(&name, &line.text));
                            if is_note_consume
                                || looks_like_private_state_transition(
                                    &line.text,
                                    private_storage_fields.get(&entry.contract_id),
                                )
                            {
                                Some(line.span.clone())
                            } else {
                                None
                            }
                        })
                    });

            let Some(span) = note_write_span.or(consume_span) else {
                continue;
            };

            out.push(ctx.diagnostic(
                self.id(),
                PROTOCOL,
                "public entrypoint mutates private state without #[only_self]",
                span,
            ));
        }
    }
}

fn looks_like_private_state_transition(
    line: &str,
    private_storage_fields: Option<&BTreeSet<String>>,
) -> bool {
    let lower = line.to_ascii_lowercase();
    (lower.contains(".insert(") && lower.contains("deliver("))
        || private_storage_fields.is_some_and(|fields| {
            let Some(field_name) = storage_field_name(line) else {
                return false;
            };
            fields.contains(&field_name)
                && (lower.contains(".write(")
                    || lower.contains(".set(")
                    || lower.contains(".insert(")
                    || lower.contains(".remove("))
        })
}

fn storage_field_name(line: &str) -> Option<String> {
    let normalized = normalize_line(line);
    let marker = "self.storage.";
    let start = normalized.find(marker)? + marker.len();
    if start >= normalized.len() {
        return None;
    }
    let tail = &normalized[start..];
    let mut end = 0usize;
    for (idx, ch) in tail.char_indices() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            end = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    Some(tail[..end].to_string())
}

fn private_storage_fields_by_contract(ctx: &RuleContext<'_>) -> BTreeMap<String, BTreeSet<String>> {
    let mut by_contract = BTreeMap::<String, BTreeSet<String>>::new();
    for file in ctx.files() {
        let contract_ranges = contract_ranges(file.text(), file.path());
        for (contract_id, body_start, body_end) in contract_ranges {
            let body = &file.text()[body_start..body_end];
            let private_fields = private_storage_fields_in_contract_body(body);
            if private_fields.is_empty() {
                continue;
            }
            by_contract
                .entry(contract_id)
                .or_default()
                .extend(private_fields);
        }
    }
    by_contract
}

fn contract_ranges(source: &str, file_path: &str) -> Vec<(String, usize, usize)> {
    let mut ranges = Vec::<(String, usize, usize)>::new();
    let normalized_file = normalize_file_path(file_path);
    let mut offset = 0usize;
    let mut brace_depth = 0usize;
    let mut current_contract: Option<(String, usize, usize)> = None;

    for line in source.lines() {
        let trimmed = line.trim();
        let (_, code_after_attributes) = split_inline_attributes(trimmed);
        if current_contract.is_none()
            && let Some(contract_name) = is_contract_start(code_after_attributes)
        {
            current_contract = Some((
                format!("{normalized_file}::{contract_name}"),
                offset,
                brace_depth + line.matches('{').count(),
            ));
        }

        let opens = line.bytes().filter(|byte| *byte == b'{').count();
        let closes = line.bytes().filter(|byte| *byte == b'}').count();
        brace_depth = brace_depth.saturating_add(opens).saturating_sub(closes);

        if let Some((contract_id, start, end_depth)) = current_contract.clone()
            && brace_depth < end_depth
        {
            ranges.push((contract_id, start, offset + line.len()));
            current_contract = None;
        }

        offset += line.len() + 1;
    }

    if let Some((contract_id, start, _)) = current_contract {
        ranges.push((contract_id, start, source.len()));
    }

    ranges
}

fn private_storage_fields_in_contract_body(contract_body: &str) -> BTreeSet<String> {
    let mut private_fields = BTreeSet::<String>::new();
    let mut pending_storage_attribute = false;
    let mut in_storage_struct = false;
    let mut struct_depth = 0usize;

    for line in contract_body.lines() {
        let trimmed = line.trim();
        let (inline_attrs, code_after_attributes) = split_inline_attributes(trimmed);
        if inline_attrs.iter().any(|attr| attr.contains("#[storage]")) {
            pending_storage_attribute = true;
        }
        let normalized = normalize_line(code_after_attributes);

        if !in_storage_struct && pending_storage_attribute && is_struct_start(normalized).is_some()
        {
            in_storage_struct = true;
            struct_depth = line.bytes().filter(|byte| *byte == b'{').count();
            pending_storage_attribute = false;
            continue;
        }

        if in_storage_struct {
            if let Some((field_name, field_type)) = parse_storage_field(normalized)
                && is_private_storage_field_type(&field_type)
            {
                private_fields.insert(field_name);
            }
            let opens = line.bytes().filter(|byte| *byte == b'{').count();
            let closes = line.bytes().filter(|byte| *byte == b'}').count();
            struct_depth = struct_depth.saturating_add(opens).saturating_sub(closes);
            if struct_depth == 0 {
                in_storage_struct = false;
            }
            continue;
        }

        if !normalized.is_empty() {
            pending_storage_attribute = false;
        }
    }

    private_fields
}

fn parse_storage_field(line: &str) -> Option<(String, String)> {
    let line = line.trim_end_matches(',').trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let colon = line.find(':')?;
    let name_segment = line[..colon].trim();
    if name_segment.is_empty() {
        return None;
    }
    let field_name = name_segment.split_whitespace().next_back()?;
    if !field_name
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
    {
        return None;
    }
    if !field_name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let field_type = line[colon + 1..].trim();
    if field_type.is_empty() {
        return None;
    }
    Some((field_name.to_string(), field_type.to_string()))
}

fn is_private_storage_field_type(field_type: &str) -> bool {
    let normalized = field_type
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    normalized.contains("privateset<")
        || normalized.contains("privatemutable<")
        || normalized.contains("privateimmutable<")
        || normalized.contains("privatemap<")
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

#[cfg(test)]
mod tests {
    use aztec_lint_aztec::build_aztec_model;
    use aztec_lint_aztec::detect::SourceUnit;
    use aztec_lint_core::config::AztecConfig;
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Aztec033PublicMutatesPrivateWithoutOnlySelfRule;

    #[test]
    fn reports_public_mutation_without_only_self() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("public")]
    fn rotate() {
        self.storage.notes.insert(1).deliver(0);
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
        Aztec033PublicMutatesPrivateWithoutOnlySelfRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_public_mutation_with_only_self() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("public")]
    #[only_self]
    fn rotate() {
        self.storage.notes.insert(1).deliver(0);
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
        Aztec033PublicMutatesPrivateWithoutOnlySelfRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_public_mutation_of_public_storage_without_only_self() {
        let source = r#"
#[aztec]
pub contract C {
    #[storage]
    struct Storage<Context> {
        nft_exists: Map<Field, PublicMutable<bool, Context>, Context>,
    }

    #[external("public")]
    fn mint() {
        self.storage.nft_exists.at(1).write(true);
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
        Aztec033PublicMutatesPrivateWithoutOnlySelfRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_public_mutation_of_private_storage_without_only_self() {
        let source = r#"
#[aztec]
pub contract C {
    #[storage]
    struct Storage<Context> {
        private_flag: PrivateMutable<bool, Context>,
    }

    #[external("public")]
    fn mutate_private_flag() {
        self.storage.private_flag.write(true);
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
        Aztec033PublicMutatesPrivateWithoutOnlySelfRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }
}
