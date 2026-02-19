use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::engine::context::RuleContext;
use crate::noir_core::util::find_let_bindings;

pub struct Noir002ShadowingRule;

impl Rule for Noir002ShadowingRule {
    fn id(&self) -> &'static str {
        "NOIR002"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            let mut depth = 0usize;
            let mut active = Vec::<(String, usize)>::new();
            let mut offset = 0usize;

            for line in file.text().lines() {
                for (name, column) in find_let_bindings(line) {
                    if name.starts_with('_') {
                        continue;
                    }

                    if active.iter().any(|(existing, _)| existing == &name) {
                        let start = offset + column;
                        out.push(ctx.diagnostic(
                            self.id(),
                            CORRECTNESS,
                            format!("`{name}` shadows an existing binding in scope"),
                            file.span_for_range(start, start + name.len()),
                        ));
                    }

                    active.push((name, depth));
                }

                let opens = line.bytes().filter(|byte| *byte == b'{').count();
                let closes = line.bytes().filter(|byte| *byte == b'}').count();
                depth = depth.saturating_add(opens).saturating_sub(closes);
                active.retain(|(_, declared_depth)| *declared_depth <= depth);

                offset += line.len() + 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Noir002ShadowingRule;

    #[test]
    fn detects_shadowed_binding() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let value = 1; { let value = 2; } }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir002ShadowingRule.run(&context, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("shadows"));
    }

    #[test]
    fn ignores_distinct_bindings() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { let left = 1; let right = 2; }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir002ShadowingRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }
}
