use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::MAINTAINABILITY;

use crate::Rule;
use crate::engine::context::RuleContext;
use crate::noir_core::util::find_function_scopes;

pub struct Noir110ComplexityRule;

const COMPLEXITY_LIMIT: usize = 6;

impl Rule for Noir110ComplexityRule {
    fn id(&self) -> &'static str {
        "NOIR110"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            let source = file.text();
            for function in find_function_scopes(source) {
                let body = &source[function.body_start..function.body_end];
                let complexity = compute_complexity_score(body);
                if complexity <= COMPLEXITY_LIMIT {
                    continue;
                }

                out.push(ctx.diagnostic(
                    self.id(),
                    MAINTAINABILITY,
                    format!(
                        "function `{}` complexity is {complexity} (limit: {COMPLEXITY_LIMIT})",
                        function.name
                    ),
                    file.span_for_range(
                        function.name_offset,
                        function.name_offset + function.name.len(),
                    ),
                ));
            }
        }
    }
}

fn compute_complexity_score(body: &str) -> usize {
    body.matches("if ").count()
        + body.matches("for ").count()
        + body.matches("while ").count()
        + body.matches("match ").count()
        + body.matches("&&").count()
        + body.matches("||").count()
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir110ComplexityRule;

    #[test]
    fn reports_complex_function() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                r#"
fn main(x: Field) {
    if x > 1 { }
    if x > 2 { }
    if x > 3 { }
    if x > 4 { }
    if x > 5 { }
    if x > 6 { }
    if x > 7 { }
}
"#
                .to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir110ComplexityRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_simple_function() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let x = 1; assert(x == 1); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir110ComplexityRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }
}
