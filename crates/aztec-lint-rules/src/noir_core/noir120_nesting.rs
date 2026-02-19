use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::MAINTAINABILITY;

use crate::Rule;
use crate::engine::context::RuleContext;
use crate::noir_core::util::find_function_scopes;

pub struct Noir120NestingRule;

const NESTING_LIMIT: usize = 3;

impl Rule for Noir120NestingRule {
    fn id(&self) -> &'static str {
        "NOIR120"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            let source = file.text();
            for function in find_function_scopes(source) {
                let body = &source[function.body_start..function.body_end];
                let max_depth = max_brace_depth(body);
                let logical_depth = max_depth.saturating_sub(1);
                if logical_depth <= NESTING_LIMIT {
                    continue;
                }

                out.push(ctx.diagnostic(
                    self.id(),
                    MAINTAINABILITY,
                    format!(
                        "function `{}` nesting depth is {logical_depth} (limit: {NESTING_LIMIT})",
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

fn max_brace_depth(body: &str) -> usize {
    let mut depth = 0usize;
    let mut max_depth = 0usize;

    for byte in body.bytes() {
        match byte {
            b'{' => {
                depth += 1;
                max_depth = max_depth.max(depth);
            }
            b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }

    max_depth
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir120NestingRule;

    #[test]
    fn reports_deep_nesting() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { if true { if true { if true { if true { let x = 1; } } } } }"
                    .to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir120NestingRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_shallow_nesting() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { if true { let x = 1; } }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir120NestingRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }
}
