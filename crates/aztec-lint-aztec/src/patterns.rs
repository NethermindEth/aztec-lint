use aztec_lint_core::config::AztecConfig;

pub fn normalize_line(line: &str) -> &str {
    line.split_once("//").map_or(line, |(code, _)| code).trim()
}

pub fn is_contract_start(line: &str) -> Option<String> {
    let raw = normalize_line(line);
    if !(raw.starts_with("contract ") || raw.starts_with("pub contract ")) {
        return None;
    }
    extract_identifier_after_keyword(raw, "contract")
}

pub fn is_function_start(line: &str) -> Option<String> {
    let raw = normalize_line(line);
    if raw.starts_with("fn ")
        || raw.starts_with("pub fn ")
        || raw.starts_with("unconstrained fn ")
        || raw.starts_with("pub unconstrained fn ")
    {
        return extract_identifier_after_keyword(raw, "fn");
    }
    None
}

pub fn is_storage_struct(line: &str, config: &AztecConfig) -> Option<String> {
    let raw = normalize_line(line);
    let marker = format!("#[{}]", config.storage_attribute);
    if !raw.contains(&marker) {
        return None;
    }
    if let Some(name) = extract_identifier_after_keyword(raw, "struct") {
        return Some(name);
    }
    None
}

pub fn is_struct_start(line: &str) -> Option<String> {
    let raw = normalize_line(line);
    if raw.starts_with("struct ") || raw.starts_with("pub struct ") {
        return extract_identifier_after_keyword(raw, "struct");
    }
    None
}

pub fn has_attribute(line: &str, attribute: &str) -> bool {
    let raw = normalize_line(line);
    raw.contains(&format!("#[{attribute}]"))
}

pub fn has_external_kind(line: &str, kind: &str, config: &AztecConfig) -> bool {
    let raw = normalize_line(line);
    let marker = format!("#[{}(\"{kind}\")]", config.external_attribute);
    raw.contains(&marker)
}

/// Text-only fallback helper.
/// Semantic classification should be preferred whenever semantic data is available.
pub fn fallback_looks_like_enqueue(line: &str, config: &AztecConfig) -> bool {
    let raw = normalize_line(line);
    raw.contains(&format!("self.{}(", config.enqueue_fn)) || raw.contains("enqueue_self")
}

pub fn is_note_getter_call_name(name: &str, config: &AztecConfig) -> bool {
    config
        .note_getter_fns
        .iter()
        .any(|configured| call_name_matches(name, configured))
}

pub fn is_nullifier_call_name(name: &str, config: &AztecConfig) -> bool {
    config
        .nullifier_fns
        .iter()
        .any(|configured| call_name_matches(name, configured))
}

pub fn is_enqueue_call_name(name: &str, config: &AztecConfig) -> bool {
    call_name_matches(name, &config.enqueue_fn) || call_name_matches(name, "enqueue_self")
}

pub fn is_public_sink_call_name(name: &str) -> bool {
    call_name_matches(name, "emit")
        || call_name_matches(name, "public_log")
        || call_name_matches(name, "debug_log")
}

pub fn is_note_write_call_name(name: &str) -> bool {
    call_name_matches(name, "insert")
}

pub fn extract_call_name(expression: &str) -> Option<String> {
    let normalized = normalize_line(expression);
    let open = normalized.find('(')?;
    let before = normalized[..open].trim_end();
    let candidate = before
        .rsplit_once('.')
        .map(|(_, tail)| tail)
        .or_else(|| before.rsplit_once("::").map(|(_, tail)| tail))
        .unwrap_or(before)
        .trim();
    if candidate.is_empty() {
        return None;
    }
    let mut chars = candidate.char_indices();
    let (start, first) = chars.next()?;
    if !is_ident_start(first) {
        return None;
    }
    let mut end = start + first.len_utf8();
    for (idx, ch) in chars {
        if !is_ident_continue(ch) {
            break;
        }
        end = idx + ch.len_utf8();
    }
    Some(candidate[start..end].to_string())
}

pub fn extract_enqueue_target_function(line: &str) -> Option<String> {
    let raw = normalize_line(line);

    if let Some(idx) = raw.find("enqueue_self") {
        let tail = &raw[idx..];
        if let Some(dot) = tail.find('.') {
            return extract_identifier(&tail[dot + 1..]);
        }
    }

    if let Some(dot) = raw.rfind('.') {
        let after = &raw[dot + 1..];
        if after.contains('(') {
            return extract_identifier(after);
        }
    }

    None
}

pub fn is_same_contract_enqueue(line: &str) -> bool {
    let raw = normalize_line(line);
    raw.contains("this_address") || raw.contains("enqueue_self")
}

/// Text-only fallback helper.
/// Semantic classification should be preferred whenever semantic data is available.
pub fn fallback_contains_note_read(line: &str, config: &AztecConfig) -> bool {
    let raw = normalize_line(line);
    extract_call_names(raw)
        .iter()
        .any(|name| is_note_getter_call_name(name, config))
}

/// Text-only fallback helper.
/// Semantic classification should be preferred whenever semantic data is available.
pub fn fallback_contains_note_write(line: &str) -> bool {
    let raw = normalize_line(line);
    raw.contains(".insert(") && (raw.contains("deliver(") || raw.contains("ONCHAIN_CONSTRAINED"))
}

/// Text-only fallback helper.
/// Semantic classification should be preferred whenever semantic data is available.
pub fn fallback_contains_nullifier_emit(line: &str, config: &AztecConfig) -> bool {
    let raw = normalize_line(line);
    extract_call_names(raw)
        .iter()
        .any(|name| is_nullifier_call_name(name, config))
}

/// Text-only fallback helper.
/// Semantic classification should be preferred whenever semantic data is available.
pub fn fallback_contains_public_sink(line: &str) -> bool {
    let raw = normalize_line(line);
    extract_call_names(raw)
        .iter()
        .any(|name| is_public_sink_call_name(name))
        || raw.contains("return ")
}

fn call_name_matches(candidate: &str, configured: &str) -> bool {
    let normalize = |name: &str| name.trim().trim_start_matches("self.").to_ascii_lowercase();
    normalize(candidate) == normalize(configured)
}

fn extract_call_names(expression: &str) -> Vec<String> {
    let normalized = normalize_line(expression);
    let bytes = normalized.as_bytes();
    let mut names = Vec::<String>::new();

    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'(' {
            continue;
        }
        let mut end = index;
        while end > 0 && bytes[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        if end == 0 {
            continue;
        }
        let mut start = end;
        while start > 0 && is_ident_continue(char::from(bytes[start - 1])) {
            start -= 1;
        }
        if start >= end || !is_ident_start(char::from(bytes[start])) {
            continue;
        }
        names.push(normalized[start..end].to_string());
    }

    names
}

fn extract_identifier_after_keyword(raw: &str, keyword: &str) -> Option<String> {
    let start = raw.find(keyword)? + keyword.len();
    extract_identifier(&raw[start..])
}

fn extract_identifier(raw: &str) -> Option<String> {
    let trimmed = raw.trim_start();
    let mut chars = trimmed.char_indices();
    let (start, first) = chars.next()?;
    if !is_ident_start(first) {
        return None;
    }

    let mut end = start + first.len_utf8();
    for (idx, ch) in chars {
        if !is_ident_continue(ch) {
            break;
        }
        end = idx + ch.len_utf8();
    }

    Some(trimmed[start..end].to_string())
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::config::AztecConfig;

    use super::{
        extract_call_name, extract_enqueue_target_function, fallback_contains_note_read,
        fallback_contains_nullifier_emit, fallback_contains_public_sink, has_external_kind,
        is_public_sink_call_name, is_same_contract_enqueue, is_struct_start,
    };

    #[test]
    fn parses_external_attribute_kind() {
        let config = AztecConfig::default();
        assert!(has_external_kind(
            "#[external(\"public\")]",
            "public",
            &config
        ));
    }

    #[test]
    fn extracts_enqueue_target_name() {
        let target = extract_enqueue_target_function(
            "self.enqueue(Contract::at(self.context.this_address()).mint_public(value));",
        );
        assert_eq!(target.as_deref(), Some("mint_public"));
    }

    #[test]
    fn identifies_same_contract_enqueue() {
        assert!(is_same_contract_enqueue(
            "self.enqueue(Contract::at(self.context.this_address()).mint_public(value));"
        ));
    }

    #[test]
    fn parses_struct_start() {
        assert_eq!(
            is_struct_start("pub struct Storage {"),
            Some("Storage".to_string())
        );
    }

    #[test]
    fn extracts_call_name_from_expression() {
        assert_eq!(
            extract_call_name("self.notes.get_notes()"),
            Some("get_notes".to_string())
        );
        assert_eq!(extract_call_name("emit(value)"), Some("emit".to_string()));
    }

    #[test]
    fn recognizes_public_sink_call_names() {
        assert!(is_public_sink_call_name("emit"));
        assert!(is_public_sink_call_name("debug_log"));
        assert!(!is_public_sink_call_name("hash"));
    }

    #[test]
    fn detects_calls_when_target_call_is_not_first_on_line() {
        let config = AztecConfig::default();
        assert!(fallback_contains_note_read(
            "let notes = wrapper(self.notes.get_notes());",
            &config
        ));
        assert!(fallback_contains_nullifier_emit(
            "let hash = wrapper(self.emit_nullifier(value));",
            &config
        ));
        assert!(fallback_contains_public_sink(
            "let x = wrapper(emit(value));"
        ));
    }
}
