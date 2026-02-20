use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::policy::CORRECTNESS;

use crate::Rule;
use crate::engine::context::RuleContext;
use crate::noir_core::util::{find_function_scopes, is_ident_continue};

pub struct Noir002ShadowingRule;

impl Rule for Noir002ShadowingRule {
    fn id(&self) -> &'static str {
        "NOIR002"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        for file in ctx.files() {
            for scope in find_function_scopes(file.text()) {
                let body_start = scope.body_start.saturating_add(1);
                let body_end = scope.body_end.saturating_sub(1);
                if body_start >= body_end || body_end > file.text().len() {
                    continue;
                }
                let body = &file.text()[body_start..body_end];
                let bindings = let_bindings_with_depth(body, body_start);

                let mut depth = 0usize;
                let mut active = Vec::<(String, usize)>::new();
                for binding in bindings {
                    active.retain(|(_, declared_depth)| *declared_depth <= binding.depth);

                    if active.iter().any(|(existing, _)| existing == &binding.name) {
                        out.push(ctx.diagnostic(
                            self.id(),
                            CORRECTNESS,
                            format!("`{}` shadows an existing binding in scope", binding.name),
                            file.span_for_range(binding.start, binding.start + binding.name.len()),
                        ));
                    }

                    active.push((binding.name, binding.depth));
                    depth = binding.depth;
                }

                active.retain(|(_, declared_depth)| *declared_depth <= depth);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Binding {
    name: String,
    start: usize,
    depth: usize,
}

fn let_bindings_with_depth(source: &str, offset: usize) -> Vec<Binding> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut idx = 0usize;
    let mut out = Vec::<Binding>::new();

    while idx < bytes.len() {
        match bytes[idx] {
            b'{' => {
                depth += 1;
                idx += 1;
                continue;
            }
            b'}' => {
                depth = depth.saturating_sub(1);
                idx += 1;
                continue;
            }
            b'/' if bytes.get(idx + 1) == Some(&b'/') => {
                while idx < bytes.len() && bytes[idx] != b'\n' {
                    idx += 1;
                }
                continue;
            }
            _ => {}
        }

        let Some((name, name_start, next_idx)) = parse_let_binding(source, idx) else {
            idx += 1;
            continue;
        };
        out.push(Binding {
            name,
            start: offset + name_start,
            depth,
        });
        idx = next_idx;
    }

    out
}

fn parse_let_binding(source: &str, start_idx: usize) -> Option<(String, usize, usize)> {
    let bytes = source.as_bytes();
    if start_idx + 3 > bytes.len() || &bytes[start_idx..start_idx + 3] != b"let" {
        return None;
    }

    let left_boundary = start_idx == 0 || !is_ident_continue(bytes[start_idx - 1]);
    let right_boundary = bytes
        .get(start_idx + 3)
        .is_some_and(|byte| byte.is_ascii_whitespace());
    if !left_boundary || !right_boundary {
        return None;
    }

    let mut idx = start_idx + 3;
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    if bytes.get(idx..idx + 3) == Some(b"mut") {
        let after_mut = idx + 3;
        if bytes
            .get(after_mut)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            idx = after_mut;
            while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
                idx += 1;
            }
        }
    }

    let first = bytes.get(idx)?;
    if !(first.is_ascii_alphabetic() || *first == b'_') {
        return None;
    }

    let name_start = idx;
    idx += 1;
    while idx < bytes.len() && is_ident_continue(bytes[idx]) {
        idx += 1;
    }
    let name = source[name_start..idx].to_string();
    if name == "_" {
        return None;
    }

    Some((name, name_start, idx))
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

    #[test]
    fn ignores_rebinding_after_nested_scope_closes_on_same_line() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn main() { { let value = 1; } let value = 2; assert(value == 2); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir002ShadowingRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_same_binding_name_in_different_functions() {
        let project = ProjectModel::default();
        let context = RuleContext::from_sources(
            &project,
            vec![(
                "src/main.nr".to_string(),
                "fn a() { let notes = 1; assert(notes == 1); } fn b() { let notes = 2; assert(notes == 2); }".to_string(),
            )],
        );

        let mut diagnostics = Vec::new();
        Noir002ShadowingRule.run(&context, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }
}
