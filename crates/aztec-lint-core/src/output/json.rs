use crate::diagnostics::{Diagnostic, diagnostic_sort_key};

pub fn render_diagnostics(diagnostics: &[&Diagnostic]) -> Result<String, serde_json::Error> {
    let mut sorted = diagnostics
        .iter()
        .map(|diagnostic| normalize_for_json((**diagnostic).clone()))
        .collect::<Vec<_>>();
    sorted.sort_by_key(diagnostic_sort_key);
    serde_json::to_string_pretty(&sorted)
}

fn normalize_for_json(mut diagnostic: Diagnostic) -> Diagnostic {
    diagnostic.suggestions.sort();
    diagnostic.notes.sort_by_key(|note| {
        if let Some(span) = &note.span {
            (
                0u8,
                span.file.clone(),
                span.line,
                span.col,
                span.start,
                span.end,
                note.message.clone(),
            )
        } else {
            (
                1u8,
                String::new(),
                0u32,
                0u32,
                0u32,
                0u32,
                note.message.clone(),
            )
        }
    });
    diagnostic.helps.sort_by_key(|help| {
        if let Some(span) = &help.span {
            (
                0u8,
                span.file.clone(),
                span.line,
                span.col,
                span.start,
                span.end,
                help.message.clone(),
            )
        } else {
            (
                1u8,
                String::new(),
                0u32,
                0u32,
                0u32,
                0u32,
                help.message.clone(),
            )
        }
    });
    diagnostic.structured_suggestions.sort_by_key(|suggestion| {
        (
            suggestion.span.file.clone(),
            suggestion.span.line,
            suggestion.span.col,
            suggestion.span.start,
            suggestion.span.end,
            suggestion.message.clone(),
            suggestion.replacement.clone(),
            suggestion.applicability.as_str().to_string(),
        )
    });
    diagnostic.fixes.sort_by_key(|fix| {
        (
            fix.span.file.clone(),
            fix.span.line,
            fix.span.col,
            fix.span.start,
            fix.span.end,
            fix.description.clone(),
            fix.replacement.clone(),
            format!("{:?}", fix.safety),
        )
    });
    diagnostic
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::render_diagnostics;
    use crate::diagnostics::{
        Applicability, Confidence, Diagnostic, Severity, StructuredMessage, StructuredSuggestion,
    };
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
            notes: Vec::new(),
            helps: Vec::new(),
            structured_suggestions: Vec::new(),
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

    #[test]
    fn json_output_includes_structured_suggestions_with_applicability() {
        let mut item = diagnostic("NOIR100", 2, "message");
        item.suggestions = vec!["legacy help".to_string()];
        item.notes = vec![StructuredMessage {
            message: "extra note".to_string(),
            span: Some(Span::new("src/main.nr", 2, 3, 2, 1)),
        }];
        item.structured_suggestions = vec![StructuredSuggestion {
            message: "replace literal".to_string(),
            span: Span::new("src/main.nr", 2, 3, 2, 1),
            replacement: "NAMED_CONST".to_string(),
            applicability: Applicability::MachineApplicable,
        }];

        let rendered = render_diagnostics(&[&item]).expect("json rendering should pass");
        let value: Value = serde_json::from_str(&rendered).expect("json output should parse");

        assert_eq!(
            value[0]["structured_suggestions"][0]["applicability"].as_str(),
            Some("machine_applicable")
        );
        assert_eq!(
            value[0]["structured_suggestions"][0]["replacement"].as_str(),
            Some("NAMED_CONST")
        );
        assert_eq!(
            value[0]["suggestions"][0].as_str(),
            Some("legacy help"),
            "legacy suggestions should remain for one-release compatibility"
        );
        assert!(
            value[0].get("fixes").is_some(),
            "legacy fixes field must remain for compatibility"
        );
    }
}
