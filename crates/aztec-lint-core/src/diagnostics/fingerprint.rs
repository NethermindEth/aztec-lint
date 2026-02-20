use crate::diagnostics::types::Diagnostic;
use crate::model::Span;

const FINGERPRINT_VERSION: &str = "v1";

pub fn normalize_file_path(file: &str) -> String {
    file.replace('\\', "/").trim_start_matches("./").to_string()
}

pub fn message_hash(message: &str) -> String {
    blake3::hash(message.as_bytes()).to_hex().to_string()
}

pub fn span_fingerprint(span: &Span, rule_id: &str) -> String {
    let normalized = normalize_file_path(&span.file);
    let payload = format!(
        "{FINGERPRINT_VERSION}|{normalized}|{}|{}|{}|{}|{rule_id}",
        span.start, span.end, span.line, span.col
    );
    blake3::hash(payload.as_bytes()).to_hex().to_string()
}

pub fn diagnostic_fingerprint(diagnostic: &Diagnostic) -> String {
    span_fingerprint(&diagnostic.primary_span, &diagnostic.rule_id)
}

#[cfg(test)]
mod tests {
    use super::{diagnostic_fingerprint, normalize_file_path, span_fingerprint};
    use crate::diagnostics::types::{Confidence, Diagnostic, Severity};
    use crate::model::Span;

    #[test]
    fn normalizes_file_paths() {
        assert_eq!(normalize_file_path("./src\\contract.nr"), "src/contract.nr");
        assert_eq!(normalize_file_path("src/contract.nr"), "src/contract.nr");
    }

    #[test]
    fn span_fingerprint_is_stable() {
        let span = Span::new("src/main.nr", 10, 12, 4, 3);
        let first = span_fingerprint(&span, "AZTEC010");
        let second = span_fingerprint(&span, "AZTEC010");
        assert_eq!(first, second);
        assert_eq!(
            first,
            "e82d1c033855cbb7a4dedf653579e32f41a0eb9afb33e1009f48156af01d84c6"
        );
    }

    #[test]
    fn diagnostic_fingerprint_is_stable() {
        let diagnostic = Diagnostic {
            rule_id: "AZTEC010".to_string(),
            severity: Severity::Error,
            confidence: Confidence::High,
            policy: "protocol".to_string(),
            message: "only_self required".to_string(),
            primary_span: Span::new("src/main.nr", 10, 12, 4, 3),
            secondary_spans: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
            structured_suggestions: Vec::new(),
            fixes: Vec::new(),
            suppressed: false,
            suppression_reason: None,
        };

        let first = diagnostic_fingerprint(&diagnostic);
        let second = diagnostic_fingerprint(&diagnostic);
        assert_eq!(first, second);
        assert_eq!(
            first,
            "e82d1c033855cbb7a4dedf653579e32f41a0eb9afb33e1009f48156af01d84c6"
        );
    }
}
