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

pub fn looks_like_enqueue(line: &str, config: &AztecConfig) -> bool {
    let raw = normalize_line(line);
    raw.contains(&format!("self.{}(", config.enqueue_fn)) || raw.contains("enqueue_self")
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

pub fn contains_note_read(line: &str, config: &AztecConfig) -> bool {
    let raw = normalize_line(line);
    config
        .note_getter_fns
        .iter()
        .any(|function| raw.contains(&format!("{function}(")))
}

pub fn contains_note_write(line: &str) -> bool {
    let raw = normalize_line(line);
    raw.contains(".insert(") && (raw.contains("deliver(") || raw.contains("ONCHAIN_CONSTRAINED"))
}

pub fn contains_nullifier_emit(line: &str, config: &AztecConfig) -> bool {
    let raw = normalize_line(line);
    config
        .nullifier_fns
        .iter()
        .any(|function| raw.contains(&format!("{function}(")))
}

pub fn contains_public_sink(line: &str) -> bool {
    let raw = normalize_line(line);
    raw.contains("emit(")
        || raw.contains("public_log(")
        || raw.contains("debug_log(")
        || raw.contains("return ")
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
        extract_enqueue_target_function, has_external_kind, is_same_contract_enqueue,
        is_struct_start,
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
}
