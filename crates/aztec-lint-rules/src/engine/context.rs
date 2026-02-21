use std::cmp::min;
use std::collections::BTreeSet;
use std::io;
use std::path::Path;

use aztec_lint_core::config::AztecConfig;
use aztec_lint_core::diagnostics::{Confidence, Diagnostic, Severity, normalize_file_path};
use aztec_lint_core::model::AztecModel;
use aztec_lint_core::model::{ProjectModel, SemanticModel, Span};

use super::query::RuleQuery;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFile {
    path: String,
    text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(path: impl Into<String>, text: impl Into<String>) -> Self {
        let path = normalize_file_path(&path.into());
        let text = text.into();
        let mut line_starts = vec![0usize];
        for (idx, byte) in text.as_bytes().iter().enumerate() {
            if *byte == b'\n' {
                line_starts.push(idx + 1);
            }
        }
        Self {
            path,
            text,
            line_starts,
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn span_for_range(&self, start: usize, end: usize) -> Span {
        let bounded_start = min(start, self.text.len());
        let bounded_end = min(end.max(bounded_start), self.text.len());
        let (line, col) = self.line_col_for_offset(bounded_start);
        Span::new(
            self.path.clone(),
            u32::try_from(bounded_start).unwrap_or(u32::MAX),
            u32::try_from(bounded_end).unwrap_or(u32::MAX),
            line,
            col,
        )
    }

    pub fn line_col_for_offset(&self, offset: usize) -> (u32, u32) {
        let bounded = min(offset, self.text.len());
        let index = match self.line_starts.binary_search(&bounded) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        let line = u32::try_from(index + 1).unwrap_or(u32::MAX);
        let col =
            u32::try_from(bounded.saturating_sub(self.line_starts[index]) + 1).unwrap_or(u32::MAX);
        (line, col)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SuppressionScope {
    rule_id: String,
    file: String,
    start: u32,
    end: u32,
    reason: String,
}

#[derive(Debug)]
pub struct RuleContext<'a> {
    project: &'a ProjectModel,
    files: Vec<SourceFile>,
    suppressions: Vec<SuppressionScope>,
    semantic_model: Option<SemanticModel>,
    aztec_model: Option<AztecModel>,
    aztec_config: Option<AztecConfig>,
}

impl<'a> RuleContext<'a> {
    pub fn from_project_root(root: &Path, project: &'a ProjectModel) -> io::Result<Self> {
        let mut files = project
            .ast_ids
            .iter()
            .map(|path| {
                let full_path = root.join(path);
                let text = std::fs::read_to_string(&full_path)?;
                Ok(SourceFile::new(path, text))
            })
            .collect::<io::Result<Vec<_>>>()?;
        files.sort_by_key(|file| file.path.clone());
        files.dedup_by(|left, right| left.path == right.path);
        Ok(Self::from_source_files(project, files))
    }

    pub fn from_sources(project: &'a ProjectModel, files: Vec<(String, String)>) -> Self {
        let mut files = files
            .into_iter()
            .map(|(path, source)| SourceFile::new(path, source))
            .collect::<Vec<_>>();
        files.sort_by_key(|file| file.path.clone());
        files.dedup_by(|left, right| left.path == right.path);
        Self::from_source_files(project, files)
    }

    fn from_source_files(project: &'a ProjectModel, files: Vec<SourceFile>) -> Self {
        let mut suppressions = files
            .iter()
            .flat_map(parse_file_suppressions)
            .collect::<Vec<_>>();
        suppressions.sort_by_key(|scope| {
            (
                scope.file.clone(),
                scope.start,
                scope.end,
                scope.rule_id.clone(),
            )
        });
        Self {
            project,
            files,
            suppressions,
            semantic_model: None,
            aztec_model: None,
            aztec_config: None,
        }
    }

    pub fn project(&self) -> &ProjectModel {
        self.project
    }

    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    pub fn semantic_model(&self) -> &SemanticModel {
        self.semantic_model
            .as_ref()
            .unwrap_or(&self.project.semantic)
    }

    pub fn set_semantic_model(&mut self, mut model: SemanticModel) {
        model.normalize();
        self.semantic_model = Some(model);
    }

    pub fn query(&self) -> RuleQuery<'_> {
        RuleQuery::new(self.semantic_model())
    }

    pub fn aztec_model(&self) -> Option<&AztecModel> {
        self.aztec_model.as_ref()
    }

    pub fn set_aztec_model(&mut self, model: AztecModel) {
        self.aztec_model = Some(model);
    }

    pub fn aztec_config(&self) -> AztecConfig {
        self.aztec_config.clone().unwrap_or_default()
    }

    pub fn set_aztec_config(&mut self, config: AztecConfig) {
        self.aztec_config = Some(config);
    }

    pub fn suppression_reason(&self, rule_id: &str, span: &Span) -> Option<&str> {
        let normalized_rule = normalize_rule_id(rule_id);
        let normalized_file = normalize_file_path(&span.file);
        let start = span.start;
        self.suppressions
            .iter()
            .find(|scope| {
                scope.rule_id == normalized_rule
                    && scope.file == normalized_file
                    && start >= scope.start
                    && start < scope.end
            })
            .map(|scope| scope.reason.as_str())
    }

    pub fn diagnostic(
        &self,
        rule_id: &str,
        policy: &'static str,
        message: impl Into<String>,
        primary_span: Span,
    ) -> Diagnostic {
        Diagnostic {
            rule_id: normalize_rule_id(rule_id),
            severity: Severity::Warning,
            confidence: Confidence::Low,
            policy: policy.to_string(),
            message: message.into(),
            primary_span,
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
}

fn normalize_rule_id(rule_id: &str) -> String {
    rule_id.trim().to_ascii_uppercase()
}

fn parse_file_suppressions(source: &SourceFile) -> Vec<SuppressionScope> {
    let mut scopes = Vec::<SuppressionScope>::new();
    let mut pending_rule_ids = BTreeSet::<String>::new();
    let mut offset = 0usize;

    for line in source.text().lines() {
        let trimmed = line.trim();
        for rule_id in extract_allow_rule_ids(trimmed) {
            pending_rule_ids.insert(rule_id);
        }

        if line_contains_item_start(trimmed) {
            if !pending_rule_ids.is_empty() {
                let scope_end = find_item_scope_end(source.text(), offset, offset + line.len());
                for rule_id in pending_rule_ids.iter() {
                    scopes.push(SuppressionScope {
                        rule_id: rule_id.clone(),
                        file: source.path().to_string(),
                        start: u32::try_from(offset).unwrap_or(u32::MAX),
                        end: u32::try_from(scope_end).unwrap_or(u32::MAX),
                        reason: format!("allow({rule_id})"),
                    });
                }
                pending_rule_ids.clear();
            }
        } else if !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("#[") {
            pending_rule_ids.clear();
        }

        offset += line.len() + 1;
    }

    scopes
}

fn extract_allow_rule_ids(input: &str) -> Vec<String> {
    let mut cursor = 0usize;
    let mut matched = BTreeSet::<String>::new();

    while let Some(start) = input[cursor..].find("#[allow(") {
        let content_start = cursor + start + "#[allow(".len();
        let Some(close_rel) = input[content_start..].find(")]") else {
            break;
        };
        let content_end = content_start + close_rel;
        let content = &input[content_start..content_end];

        for raw_token in content.split(',') {
            let token = raw_token.trim();
            if token.is_empty() {
                continue;
            }
            let candidate = token
                .split("::")
                .last()
                .unwrap_or(token)
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            if candidate.is_empty() {
                continue;
            }
            let normalized = normalize_rule_id(candidate);
            let looks_like_rule = normalized
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
                && normalized.chars().any(|ch| ch.is_ascii_alphabetic())
                && normalized.chars().any(|ch| ch.is_ascii_digit());
            if looks_like_rule {
                matched.insert(normalized);
            }
        }

        cursor = content_end + ")]".len();
    }

    matched.into_iter().collect()
}

fn is_item_start(line: &str) -> bool {
    const ITEM_PREFIXES: &[&str] = &[
        "fn ",
        "pub fn ",
        "unconstrained fn ",
        "pub unconstrained fn ",
        "struct ",
        "pub struct ",
        "contract ",
        "pub contract ",
        "impl ",
        "trait ",
        "enum ",
        "mod ",
        "pub mod ",
    ];
    ITEM_PREFIXES.iter().any(|prefix| line.starts_with(prefix))
}

fn line_contains_item_start(line: &str) -> bool {
    if is_item_start(line) {
        return true;
    }

    let mut remaining = line.trim_start();
    loop {
        if !remaining.starts_with("#[") {
            return false;
        }
        let Some(close) = remaining.find(']') else {
            return false;
        };
        remaining = remaining[close + 1..].trim_start();
        if is_item_start(remaining) {
            return true;
        }
    }
}

fn find_item_scope_end(source: &str, item_start: usize, line_end: usize) -> usize {
    let bytes = source.as_bytes();
    let Some(open_offset) = source[item_start..].find('{') else {
        return min(line_end, source.len());
    };
    let mut cursor = item_start + open_offset;
    let mut depth = 0usize;

    while cursor < bytes.len() {
        match bytes[cursor] {
            b'{' => depth += 1,
            b'}' => {
                if depth == 0 {
                    return cursor + 1;
                }
                depth -= 1;
                if depth == 0 {
                    return cursor + 1;
                }
            }
            _ => {}
        }
        cursor += 1;
    }

    source.len()
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::model::{
        CfgBlock, CfgEdge, CfgEdgeKind, DfgEdge, DfgEdgeKind, ExpressionCategory, ProjectModel,
        SemanticExpression, SemanticFunction, SemanticModel, SemanticStatement, Span,
        StatementCategory, TypeCategory,
    };

    use super::RuleContext;

    fn sample_semantic_model() -> SemanticModel {
        SemanticModel {
            functions: vec![
                SemanticFunction {
                    symbol_id: "fn::b".to_string(),
                    name: "b".to_string(),
                    module_symbol_id: "module::main".to_string(),
                    return_type_repr: "Field".to_string(),
                    return_type_category: TypeCategory::Field,
                    parameter_types: vec!["Field".to_string()],
                    is_entrypoint: false,
                    is_unconstrained: false,
                    span: Span::new("src/main.nr", 40, 41, 4, 1),
                },
                SemanticFunction {
                    symbol_id: "fn::a".to_string(),
                    name: "a".to_string(),
                    module_symbol_id: "module::main".to_string(),
                    return_type_repr: "Field".to_string(),
                    return_type_category: TypeCategory::Field,
                    parameter_types: vec!["Field".to_string()],
                    is_entrypoint: true,
                    is_unconstrained: false,
                    span: Span::new("src/main.nr", 20, 21, 2, 1),
                },
            ],
            expressions: vec![
                SemanticExpression {
                    expr_id: "expr::2".to_string(),
                    function_symbol_id: "fn::a".to_string(),
                    category: ExpressionCategory::Index,
                    type_category: TypeCategory::Field,
                    type_repr: "Field".to_string(),
                    span: Span::new("src/main.nr", 60, 61, 6, 1),
                },
                SemanticExpression {
                    expr_id: "expr::1".to_string(),
                    function_symbol_id: "fn::a".to_string(),
                    category: ExpressionCategory::Index,
                    type_category: TypeCategory::Field,
                    type_repr: "Field".to_string(),
                    span: Span::new("src/main.nr", 50, 51, 5, 1),
                },
            ],
            statements: vec![
                SemanticStatement {
                    stmt_id: "stmt::2".to_string(),
                    function_symbol_id: "fn::a".to_string(),
                    category: StatementCategory::Constrain,
                    span: Span::new("src/main.nr", 80, 81, 8, 1),
                },
                SemanticStatement {
                    stmt_id: "stmt::1".to_string(),
                    function_symbol_id: "fn::a".to_string(),
                    category: StatementCategory::Assert,
                    span: Span::new("src/main.nr", 70, 71, 7, 1),
                },
            ],
            cfg_blocks: vec![
                CfgBlock {
                    function_symbol_id: "fn::a".to_string(),
                    block_id: "bb1".to_string(),
                    statement_ids: vec!["stmt::2".to_string()],
                },
                CfgBlock {
                    function_symbol_id: "fn::a".to_string(),
                    block_id: "bb0".to_string(),
                    statement_ids: vec!["stmt::1".to_string()],
                },
            ],
            cfg_edges: vec![
                CfgEdge {
                    function_symbol_id: "fn::a".to_string(),
                    from_block_id: "bb1".to_string(),
                    to_block_id: "bb0".to_string(),
                    kind: CfgEdgeKind::FalseBranch,
                },
                CfgEdge {
                    function_symbol_id: "fn::a".to_string(),
                    from_block_id: "bb0".to_string(),
                    to_block_id: "bb1".to_string(),
                    kind: CfgEdgeKind::TrueBranch,
                },
            ],
            dfg_edges: vec![
                DfgEdge {
                    function_symbol_id: "fn::a".to_string(),
                    from_node_id: "stmt::1".to_string(),
                    to_node_id: "def::2".to_string(),
                    kind: DfgEdgeKind::DefUse,
                },
                DfgEdge {
                    function_symbol_id: "fn::a".to_string(),
                    from_node_id: "stmt::1".to_string(),
                    to_node_id: "def::1".to_string(),
                    kind: DfgEdgeKind::DefUse,
                },
                DfgEdge {
                    function_symbol_id: "fn::a".to_string(),
                    from_node_id: "stmt::1".to_string(),
                    to_node_id: "def::1".to_string(),
                    kind: DfgEdgeKind::DefUse,
                },
            ],
            ..SemanticModel::default()
        }
    }

    #[test]
    fn allow_attributes_apply_to_next_item_scope() {
        let project = ProjectModel::default();
        let source = r#"
#[allow(noir_core::NOIR100)]
fn main() {
    let value = 42;
}

fn helper() {
    let value = 7;
}
"#;
        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );

        let file = &context.files()[0];
        let main_offset = source.find("value = 42").expect("main marker should exist");
        let helper_offset = source
            .find("value = 7")
            .expect("helper marker should exist");
        let main_span = file.span_for_range(main_offset, main_offset + 5);
        let helper_span = file.span_for_range(helper_offset, helper_offset + 5);

        assert_eq!(
            context.suppression_reason("NOIR100", &main_span),
            Some("allow(NOIR100)")
        );
        assert_eq!(context.suppression_reason("NOIR100", &helper_span), None);
    }

    #[test]
    fn supports_short_form_allow_syntax() {
        let project = ProjectModel::default();
        let source = r#"
#[allow(NOIR001)]
fn main() {
    let unused = 7;
}
"#;
        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );

        let marker = source.find("unused").expect("unused marker should exist");
        let span = context.files()[0].span_for_range(marker, marker + 6);

        assert_eq!(
            context.suppression_reason("NOIR001", &span),
            Some("allow(NOIR001)")
        );
    }

    #[test]
    fn supports_same_line_allow_and_item() {
        let project = ProjectModel::default();
        let source = r#"
#[allow(NOIR001)] fn main() {
    let unused = 7;
}
"#;
        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), source.to_string())],
        );

        let marker = source.find("unused").expect("unused marker should exist");
        let span = context.files()[0].span_for_range(marker, marker + 6);

        assert_eq!(
            context.suppression_reason("NOIR001", &span),
            Some("allow(NOIR001)")
        );
    }

    #[test]
    fn query_uses_project_semantic_model_by_default() {
        let mut project = ProjectModel {
            semantic: sample_semantic_model(),
            ..ProjectModel::default()
        };
        project.normalize();

        let context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), "fn main() {}".to_string())],
        );

        let query = context.query();
        let function_ids = query
            .functions()
            .iter()
            .map(|function| function.symbol_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(function_ids, vec!["fn::a", "fn::b"]);
    }

    #[test]
    fn set_semantic_model_overrides_project_semantic() {
        let mut project = ProjectModel::default();
        project.semantic.functions.push(SemanticFunction {
            symbol_id: "fn::project".to_string(),
            name: "project".to_string(),
            module_symbol_id: "module::main".to_string(),
            return_type_repr: "Field".to_string(),
            return_type_category: TypeCategory::Field,
            parameter_types: vec![],
            is_entrypoint: true,
            is_unconstrained: false,
            span: Span::new("src/main.nr", 10, 11, 1, 1),
        });
        project.normalize();

        let mut context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), "fn main() {}".to_string())],
        );
        assert_eq!(context.query().functions()[0].symbol_id, "fn::project");

        context.set_semantic_model(sample_semantic_model());

        let query = context.query();
        let function_ids = query
            .functions()
            .iter()
            .map(|function| function.symbol_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(function_ids, vec!["fn::a", "fn::b"]);
    }

    #[test]
    fn query_results_are_deterministically_ordered() {
        let project = ProjectModel::default();
        let mut context = RuleContext::from_sources(
            &project,
            vec![("src/main.nr".to_string(), "fn main() {}".to_string())],
        );
        context.set_semantic_model(sample_semantic_model());

        let locals = context.query().locals_in_function("fn::a");
        assert_eq!(locals.len(), 2);
        assert_eq!(locals[0].definition_node_id, "def::1");
        assert_eq!(locals[1].definition_node_id, "def::2");

        let index_expr_ids = context
            .query()
            .index_accesses(Some("fn::a"))
            .iter()
            .map(|expression| expression.expr_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(index_expr_ids, vec!["expr::1", "expr::2"]);

        let assertion_stmt_ids = context
            .query()
            .assertions(Some("fn::a"))
            .iter()
            .map(|statement| statement.stmt_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(assertion_stmt_ids, vec!["stmt::1", "stmt::2"]);

        let cfg = context.query().cfg("fn::a");
        assert_eq!(cfg.blocks.len(), 2);
        assert_eq!(cfg.blocks[0].block_id, "bb0");
        assert_eq!(cfg.blocks[1].block_id, "bb1");
        assert_eq!(cfg.edges.len(), 2);
        assert_eq!(cfg.edges[0].from_block_id, "bb0");
        assert_eq!(cfg.edges[1].from_block_id, "bb1");

        let dfg = context.query().dfg("fn::a");
        assert_eq!(dfg.edges.len(), 2);
        assert_eq!(dfg.edges[0].to_node_id, "def::1");
        assert_eq!(dfg.edges[1].to_node_id, "def::2");
    }
}
