use aztec_lint_core::diagnostics::{Applicability, Diagnostic};
use aztec_lint_core::policy::MAINTAINABILITY;

use crate::Rule;
use crate::engine::context::RuleContext;
use crate::noir_core::util::extract_numeric_literals;

pub struct Noir100MagicNumbersRule;

impl Rule for Noir100MagicNumbersRule {
    fn id(&self) -> &'static str {
        "NOIR100"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            let mut offset = 0usize;

            for line in file.text().lines() {
                let code = strip_line_comment(line);
                let trimmed = code.trim_start();
                if trimmed.starts_with("const ") || trimmed.starts_with("pub const ") {
                    offset += line.len() + 1;
                    continue;
                }

                for (literal, column) in extract_numeric_literals(code) {
                    let value = literal.parse::<i64>().unwrap_or(0);
                    if value == 0 || value == 1 {
                        continue;
                    }

                    let start = offset + column;
                    let span = file.span_for_range(start, start + literal.len());
                    out.push(
                        ctx.diagnostic(
                            self.id(),
                            MAINTAINABILITY,
                            format!("magic number `{literal}` should be named"),
                            span.clone(),
                        )
                        .help("extract this literal into a named constant for readability")
                        .span_suggestion(
                            span,
                            format!("replace `{literal}` with a named constant"),
                            "NAMED_CONSTANT".to_string(),
                            Applicability::MaybeIncorrect,
                        ),
                    );
                }

                offset += line.len() + 1;
            }
        }
    }
}

fn strip_line_comment(line: &str) -> &str {
    line.split_once("//").map_or(line, |(code, _)| code)
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir100MagicNumbersRule;

    #[test]
    fn reports_magic_numbers() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let fee = 42; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].structured_suggestions.len(), 1);
        assert_eq!(
            diagnostics[0].structured_suggestions[0].applicability,
            aztec_lint_core::diagnostics::Applicability::MaybeIncorrect
        );
    }

    #[test]
    fn ignores_constants_and_small_literals() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "const FEE: u32 = 42; fn main() { let flag = 1; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir100MagicNumbersRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }
}
