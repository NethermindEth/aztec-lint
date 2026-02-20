pub mod fingerprint;
pub mod types;

pub use fingerprint::{
    diagnostic_fingerprint, message_hash, normalize_file_path, span_fingerprint,
};
pub use types::{
    Applicability, Confidence, Diagnostic, Fix, FixSafety, MultipartSuggestionPart, Severity,
    StructuredMessage, StructuredSuggestion,
};

pub fn diagnostic_sort_key(diagnostic: &Diagnostic) -> (String, u32, u32, String, String) {
    (
        normalize_file_path(&diagnostic.primary_span.file),
        diagnostic.primary_span.start,
        diagnostic.primary_span.end,
        diagnostic.rule_id.clone(),
        message_hash(&diagnostic.message),
    )
}

pub fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by_key(diagnostic_sort_key);
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, sort_diagnostics};
    use crate::diagnostics::{Confidence, Severity};
    use crate::model::Span;

    fn diag(file: &str, start: u32, end: u32, rule_id: &str, message: &str) -> Diagnostic {
        Diagnostic {
            rule_id: rule_id.to_string(),
            severity: Severity::Warning,
            confidence: Confidence::High,
            policy: "privacy".to_string(),
            message: message.to_string(),
            primary_span: Span::new(file, start, end, 1, 1),
            secondary_spans: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
            structured_suggestions: Vec::new(),
            fixes: Vec::new(),
            suppressed: false,
            suppression_reason: None,
        }
    }

    #[test]
    fn diagnostics_sort_is_deterministic() {
        let mut a = vec![
            diag("src/b.nr", 4, 8, "AZTEC020", "z message"),
            diag("src/a.nr", 10, 12, "AZTEC001", "b message"),
            diag("src/a.nr", 10, 11, "AZTEC001", "a message"),
            diag("src/a.nr", 10, 11, "AZTEC001", "c message"),
        ];
        let mut b = a.iter().cloned().rev().collect::<Vec<_>>();

        sort_diagnostics(&mut a);
        sort_diagnostics(&mut b);

        assert_eq!(a, b);
        assert_eq!(a[0].message, "a message");
        assert_eq!(a[1].message, "c message");
        assert_eq!(a[2].message, "b message");
        assert_eq!(a[3].primary_span.file, "src/b.nr");
    }
}
