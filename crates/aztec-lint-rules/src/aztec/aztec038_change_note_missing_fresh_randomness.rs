use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::aztec::text_scan::scan_functions;
use crate::engine::context::RuleContext;

pub struct Aztec038ChangeNoteMissingFreshRandomnessRule;

impl Rule for Aztec038ChangeNoteMissingFreshRandomnessRule {
    fn id(&self) -> &'static str {
        "AZTEC038"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(_model) = ctx.aztec_model() else {
            return;
        };

        for function in scan_functions(ctx) {
            for line in &function.lines {
                let Some(rhs) = extract_randomness_rhs(&line.text) else {
                    continue;
                };
                if !is_change_note_context(&line.text) {
                    continue;
                }
                if !looks_missing_freshness(&rhs) {
                    continue;
                }

                out.push(ctx.diagnostic(
                    self.id(),
                    CORRECTNESS,
                    "change note randomness appears to reuse deterministic or stale entropy",
                    line.span.clone(),
                ));
            }
        }
    }
}

fn extract_randomness_rhs(line: &str) -> Option<String> {
    let normalized = line.trim();
    let lower = normalized.to_ascii_lowercase();

    let rhs_start = find_randomness_field_rhs_start(normalized, &lower)?;
    let rhs = normalized[rhs_start..]
        .trim()
        .trim_end_matches(',')
        .trim_end_matches(';')
        .trim()
        .to_string();
    if rhs.is_empty() {
        return None;
    }
    Some(rhs)
}

fn find_randomness_field_rhs_start(line: &str, lower: &str) -> Option<usize> {
    if let Some(idx) = lower.find("randomness:") {
        return Some(idx + "randomness:".len());
    }

    let marker = ".randomness";
    let idx = lower.find(marker)?;
    let after = idx + marker.len();
    let eq_relative = line[after..].find('=')?;
    Some(after + eq_relative + 1)
}

fn is_change_note_context(line: &str) -> bool {
    has_change_note_token(line)
}

fn has_change_note_token(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("change_note")
        || lower.contains("changenote")
        || (lower.contains("change") && lower.contains("note"))
}

fn looks_missing_freshness(rhs: &str) -> bool {
    let lower = rhs.to_ascii_lowercase();
    if lower.contains(".randomness") {
        return true;
    }
    if lower.contains("hash(")
        || lower.contains("poseidon")
        || lower.contains("pedersen")
        || lower.contains("derive")
    {
        return !deterministic_derivation_has_uniqueness_context(&lower);
    }

    if has_freshness_token(&lower) {
        return false;
    }
    lower.contains("randomness")
}

fn deterministic_derivation_has_uniqueness_context(rhs: &str) -> bool {
    let args = first_call_arguments(rhs).unwrap_or(rhs);
    has_freshness_token(args)
}

fn first_call_arguments(text: &str) -> Option<&str> {
    let open = text.find('(')?;
    let close = text.rfind(')')?;
    if close <= open {
        return None;
    }
    Some(text[open + 1..close].trim())
}

fn has_freshness_token(text: &str) -> bool {
    const TOKENS: [&str; 7] = [
        "fresh", "new", "nonce", "salt", "blinding", "index", "counter",
    ];
    TOKENS.iter().any(|token| text.contains(token))
}

#[cfg(test)]
mod tests {
    use aztec_lint_aztec::build_aztec_model;
    use aztec_lint_aztec::detect::SourceUnit;
    use aztec_lint_core::config::AztecConfig;
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Aztec038ChangeNoteMissingFreshRandomnessRule;

    #[test]
    fn reports_reused_note_randomness() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn spend(note: Field) {
        let change_note = ChangeNote { randomness: note.randomness };
        emit(change_note);
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
        Aztec038ChangeNoteMissingFreshRandomnessRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_fresh_randomness_derived_with_nonce() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn spend(note: Field, nonce: Field) {
        let change_note = ChangeNote { randomness: hash(note, nonce) };
        emit(change_note);
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
        Aztec038ChangeNoteMissingFreshRandomnessRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_deterministic_derivation_without_uniqueness_context() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn spend(note: Field) {
        let change_note = ChangeNote { randomness: derive_randomness(note) };
        emit(change_note);
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
        Aztec038ChangeNoteMissingFreshRandomnessRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_non_change_note_randomness_assignment() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn spend(note: Field, nonce: Field) {
        let outgoing_note = OutgoingNote { randomness: note.randomness };
        let change_note = ChangeNote { randomness: hash(note, nonce) };
        emit(outgoing_note);
        emit(change_note);
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
        Aztec038ChangeNoteMissingFreshRandomnessRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }
}
