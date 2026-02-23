use std::collections::{BTreeMap, BTreeSet};

use aztec_lint_aztec::SourceUnit;
use aztec_lint_aztec::taint::{
    TaintSinkKind, TaintSourceKind, analyze_intra_procedural, build_def_use_graph_with_semantic,
};
use aztec_lint_core::diagnostics::Diagnostic;
use aztec_lint_core::model::Span;
use aztec_lint_core::policy::PRIVACY;

use crate::Rule;
use crate::engine::context::RuleContext;

pub struct Aztec036SecretBranchAffectsEnqueueRule;

impl Rule for Aztec036SecretBranchAffectsEnqueueRule {
    fn id(&self) -> &'static str {
        "AZTEC036"
    }

    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(model) = ctx.aztec_model() else {
            return;
        };

        let config = ctx.aztec_config();
        let sources = ctx
            .files()
            .iter()
            .map(|file| SourceUnit::new(file.path().to_string(), file.text().to_string()))
            .collect::<Vec<_>>();
        let graph =
            build_def_use_graph_with_semantic(&sources, model, Some(ctx.semantic_model()), &config);
        let analysis = analyze_intra_procedural(&graph);
        let file_texts = ctx
            .files()
            .iter()
            .map(|file| (file.path().to_string(), file.text().to_string()))
            .collect::<BTreeMap<_, _>>();

        let enqueue_flows = analysis
            .flows
            .iter()
            .filter(|flow| flow.sink_kind == TaintSinkKind::EnqueuePublicCall)
            .filter(|flow| is_secret_source(flow.source_kind))
            .collect::<Vec<_>>();

        let mut emitted = BTreeSet::<(String, u32, u32, String)>::new();

        for flow in &analysis.flows {
            if flow.sink_kind != TaintSinkKind::BranchCondition {
                continue;
            }
            if !is_secret_source(flow.source_kind) {
                continue;
            }
            if !enqueue_flows.iter().any(|enqueue_flow| {
                enqueue_flow.function_symbol_id == flow.function_symbol_id
                    && enqueue_flow.variable == flow.variable
                    && branch_controls_enqueue(
                        &flow.sink_span,
                        &enqueue_flow.sink_span,
                        file_texts.get(flow.sink_span.file.as_str()),
                    )
            }) {
                continue;
            }

            let key = (
                flow.function_symbol_id.clone(),
                flow.sink_span.start,
                flow.sink_span.end,
                flow.variable.clone(),
            );
            if !emitted.insert(key) {
                continue;
            }

            out.push(ctx.diagnostic(
                self.id(),
                PRIVACY,
                format!(
                    "secret-derived value `{}` controls a branch that affects enqueue behavior",
                    flow.variable
                ),
                flow.sink_span.clone(),
            ));
        }
    }
}

fn is_secret_source(kind: TaintSourceKind) -> bool {
    matches!(
        kind,
        TaintSourceKind::NoteRead
            | TaintSourceKind::PrivateEntrypointParam
            | TaintSourceKind::SecretState
    )
}

fn branch_controls_enqueue(
    branch_span: &Span,
    enqueue_span: &Span,
    file_text: Option<&String>,
) -> bool {
    if branch_span.file != enqueue_span.file || enqueue_span.start <= branch_span.start {
        return false;
    }
    if span_contains(branch_span, enqueue_span) {
        return true;
    }

    let Some(source) = file_text else {
        return false;
    };
    enqueue_in_branch_block(source, branch_span, enqueue_span)
}

fn span_contains(outer: &Span, inner: &Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn enqueue_in_branch_block(source: &str, branch_span: &Span, enqueue_span: &Span) -> bool {
    let Ok(branch_start) = usize::try_from(branch_span.start) else {
        return false;
    };
    let Ok(enqueue_start) = usize::try_from(enqueue_span.start) else {
        return false;
    };
    if enqueue_start <= branch_start || branch_start >= source.len() {
        return false;
    }

    let bytes = source.as_bytes();
    let Some(primary_open) = find_byte(bytes, b'{', branch_start) else {
        return false;
    };
    let Some(primary_close) = matching_brace(bytes, primary_open) else {
        return false;
    };
    if enqueue_start > primary_open && enqueue_start < primary_close {
        return true;
    }

    let mut cursor = skip_ascii_whitespace(bytes, primary_close + 1);
    if !slice_eq(bytes, cursor, b"else") {
        return false;
    }
    cursor += 4;
    cursor = skip_ascii_whitespace(bytes, cursor);
    let Some(else_open) = find_byte(bytes, b'{', cursor) else {
        return false;
    };
    let Some(else_close) = matching_brace(bytes, else_open) else {
        return false;
    };
    enqueue_start > else_open && enqueue_start < else_close
}

fn find_byte(bytes: &[u8], needle: u8, start: usize) -> Option<usize> {
    bytes
        .iter()
        .enumerate()
        .skip(start)
        .find_map(|(idx, byte)| (*byte == needle).then_some(idx))
}

fn matching_brace(bytes: &[u8], open_index: usize) -> Option<usize> {
    let mut depth = 0u32;
    for (idx, byte) in bytes.iter().enumerate().skip(open_index) {
        if *byte == b'{' {
            depth = depth.saturating_add(1);
            continue;
        }
        if *byte == b'}' {
            if depth == 0 {
                return None;
            }
            depth -= 1;
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

fn skip_ascii_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

fn slice_eq(bytes: &[u8], index: usize, expected: &[u8]) -> bool {
    bytes
        .get(index..index.saturating_add(expected.len()))
        .is_some_and(|slice| slice == expected)
}

#[cfg(test)]
mod tests {
    use aztec_lint_aztec::build_aztec_model;
    use aztec_lint_aztec::detect::SourceUnit;
    use aztec_lint_core::config::AztecConfig;
    use aztec_lint_core::model::ProjectModel;

    use crate::Rule;
    use crate::engine::context::RuleContext;

    use super::Aztec036SecretBranchAffectsEnqueueRule;

    #[test]
    fn reports_secret_branch_that_also_taints_enqueue_argument() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        if secret > 10 {
            self.enqueue(Contract::at(self.context.this_address()).mint_public(secret));
        } else {
            self.enqueue(Contract::at(self.context.this_address()).mint_public(secret + 1));
        }
    }
}
"#;
        let project = ProjectModel::default();
        let mut context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        context.set_aztec_config(AztecConfig::default());
        let model = build_aztec_model(
            &[SourceUnit::new("src/main.nr", source)],
            &AztecConfig::default(),
        );
        context.set_aztec_model(model);

        let mut diagnostics = Vec::new();
        Aztec036SecretBranchAffectsEnqueueRule.run(&context, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn ignores_secret_branch_when_enqueue_is_not_secret_tainted() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        if secret > 10 {
            emit(1);
        }

        self.enqueue(Contract::at(self.context.this_address()).mint_public(42));
    }
}
"#;
        let project = ProjectModel::default();
        let mut context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        context.set_aztec_config(AztecConfig::default());
        let model = build_aztec_model(
            &[SourceUnit::new("src/main.nr", source)],
            &AztecConfig::default(),
        );
        context.set_aztec_model(model);

        let mut diagnostics = Vec::new();
        Aztec036SecretBranchAffectsEnqueueRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_secret_tainted_enqueue_outside_secret_branch() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        if secret > 10 {
            emit(1);
        }

        self.enqueue(Contract::at(self.context.this_address()).mint_public(secret));
    }
}
"#;
        let project = ProjectModel::default();
        let mut context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );
        context.set_aztec_config(AztecConfig::default());
        let model = build_aztec_model(
            &[SourceUnit::new("src/main.nr", source)],
            &AztecConfig::default(),
        );
        context.set_aztec_model(model);

        let mut diagnostics = Vec::new();
        Aztec036SecretBranchAffectsEnqueueRule.run(&context, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }
}
