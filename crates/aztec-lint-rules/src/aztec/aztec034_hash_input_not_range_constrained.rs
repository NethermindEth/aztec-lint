use std::collections::BTreeSet;

use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::SOUNDNESS;

use crate::Rule;
use crate::aztec::text_scan::{extract_identifiers, has_hash_like_call, scan_functions};
use crate::engine::context::RuleContext;

pub struct Aztec034HashInputNotRangeConstrainedRule;

impl Rule for Aztec034HashInputNotRangeConstrainedRule {
    fn id(&self) -> &'static str {
        "AZTEC034"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(_model) = ctx.aztec_model() else {
            return;
        };

        for function in scan_functions(ctx) {
            let mut guarded = BTreeSet::<String>::new();

            for line in &function.lines {
                if looks_like_range_guard(&line.text) {
                    guarded.extend(extract_identifiers(&line.text));
                    continue;
                }

                if !has_hash_like_call(&line.text) {
                    continue;
                }

                let suspect_inputs = extract_casted_hash_inputs(&line.text);
                if suspect_inputs.is_empty() {
                    continue;
                }

                let missing = suspect_inputs
                    .iter()
                    .find(|identifier| !guarded.contains(*identifier))
                    .cloned();
                let Some(identifier) = missing else {
                    continue;
                };

                out.push(ctx.diagnostic(
                    self.id(),
                    SOUNDNESS,
                    format!(
                        "hash input `{identifier}` appears cast to Field without a prior range constraint"
                    ),
                    line.span.clone(),
                ));
            }
        }
    }
}

pub(crate) fn looks_like_range_guard(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    (lower.contains("assert(") || lower.contains("constrain(") || lower.contains("assert_max_bits"))
        && (lower.contains('<')
            || lower.contains("max_bits")
            || lower.contains("num_bits")
            || lower.contains("range"))
}

fn extract_casted_hash_inputs(line: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();

    let marker = " as Field";
    let mut search_from = 0usize;
    while let Some(relative) = line[search_from..].find(marker) {
        let idx = search_from + relative;
        let mut start = idx;
        while start > 0
            && line
                .as_bytes()
                .get(start - 1)
                .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            start -= 1;
        }
        let mut ident_start = start;
        while ident_start > 0
            && line
                .as_bytes()
                .get(ident_start - 1)
                .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            ident_start -= 1;
        }
        if ident_start < start {
            let identifier = line[ident_start..start].trim();
            if !identifier.is_empty() {
                out.insert(identifier.to_string());
            }
        }
        search_from = idx + marker.len();
    }

    let mut search_from = 0usize;
    let to_field = "to_field(";
    while let Some(relative) = line[search_from..].find(to_field) {
        let start = search_from + relative + to_field.len();
        if let Some(end) = line[start..].find(')') {
            let arg = &line[start..start + end];
            out.extend(extract_identifiers(arg));
            search_from = start + end + 1;
        } else {
            break;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use aztec_lint_aztec::build_aztec_model;
    use aztec_lint_aztec::detect::SourceUnit;
    use aztec_lint_core::config::AztecConfig;
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Aztec034HashInputNotRangeConstrainedRule;

    #[test]
    fn reports_casted_hash_input_without_guard() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn hash_amount(amount: u128) {
        let digest = hash(amount as Field);
        emit(digest);
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
        Aztec034HashInputNotRangeConstrainedRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_casted_hash_input_with_guard() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn hash_amount(amount: u128) {
        assert(amount < MAX_AMOUNT);
        let digest = hash(amount as Field);
        emit(digest);
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
        Aztec034HashInputNotRangeConstrainedRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }
}
