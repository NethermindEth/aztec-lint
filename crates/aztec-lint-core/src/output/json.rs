use crate::diagnostics::{Diagnostic, diagnostic_sort_key};

pub fn render_diagnostics(diagnostics: &[&Diagnostic]) -> Result<String, serde_json::Error> {
    let mut sorted = diagnostics.to_vec();
    sorted.sort_by_key(|diagnostic| diagnostic_sort_key(diagnostic));
    serde_json::to_string_pretty(&sorted)
}

#[cfg(test)]
mod tests {
    use super::render_diagnostics;
    use crate::diagnostics::{Confidence, Diagnostic, Severity};
    use crate::model::Span;

    fn diagnostic(rule_id: &str, line: u32, message: &str) -> Diagnostic {
        Diagnostic {
            rule_id: rule_id.to_string(),
            severity: Severity::Warning,
            confidence: Confidence::Low,
            policy: "maintainability".to_string(),
            message: message.to_string(),
            primary_span: Span::new("src/main.nr", line, line + 1, line, 1),
            secondary_spans: Vec::new(),
            suggestions: Vec::new(),
            fixes: Vec::new(),
            suppressed: false,
            suppression_reason: None,
        }
    }

    #[test]
    fn json_output_is_stably_sorted() {
        let later = diagnostic("NOIR100", 2, "later");
        let earlier = diagnostic("NOIR100", 1, "earlier");

        let rendered = render_diagnostics(&[&later, &earlier]).expect("json rendering should pass");
        let earlier_idx = rendered
            .find("\"earlier\"")
            .expect("earlier message should exist");
        let later_idx = rendered
            .find("\"later\"")
            .expect("later message should exist");
        assert!(earlier_idx < later_idx);
    }
}
