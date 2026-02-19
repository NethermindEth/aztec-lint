use std::collections::{BTreeMap, BTreeSet};

use aztec_lint_core::config::AztecConfig;
use aztec_lint_core::diagnostics::normalize_file_path;
use aztec_lint_core::model::{AztecModel, EntrypointKind, Span};

use crate::detect::SourceUnit;
use crate::patterns::{contains_note_read, is_contract_start, is_function_start, normalize_line};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TaintSourceKind {
    NoteRead,
    PrivateEntrypointParam,
    SecretState,
    UnconstrainedCall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TaintSinkKind {
    PublicOutput,
    PublicStorageWrite,
    EnqueuePublicCall,
    OracleArgument,
    LogEvent,
    NullifierOrCommitment,
    BranchCondition,
    HashOrSerialize,
    MerkleWitness,
    DebugLog,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineRecord {
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaintSource {
    pub variable: String,
    pub kind: TaintSourceKind,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Definition {
    pub variable: String,
    pub dependencies: BTreeSet<String>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuardSite {
    pub variable: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SinkSite {
    pub kind: TaintSinkKind,
    pub identifiers: BTreeSet<String>,
    pub span: Span,
    pub line: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionGraph {
    pub contract_id: String,
    pub function_symbol_id: String,
    pub is_private_entrypoint: bool,
    pub is_public_entrypoint: bool,
    pub lines: Vec<LineRecord>,
    pub definitions: Vec<Definition>,
    pub sources: Vec<TaintSource>,
    pub guards: Vec<GuardSite>,
    pub sinks: Vec<SinkSite>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DefUseGraph {
    pub functions: Vec<FunctionGraph>,
}

pub fn build_def_use_graph(
    sources: &[SourceUnit],
    model: &AztecModel,
    config: &AztecConfig,
) -> DefUseGraph {
    let mut graph = DefUseGraph::default();
    let entrypoint_kinds = model.entrypoints.iter().fold(
        BTreeMap::<String, Vec<EntrypointKind>>::new(),
        |mut map, item| {
            map.entry(item.function_symbol_id.clone())
                .or_default()
                .push(item.kind.clone());
            map
        },
    );
    let unconstrained_functions = collect_unconstrained_functions(sources);

    for source in sources {
        graph.functions.extend(build_source_functions(
            source,
            &entrypoint_kinds,
            &unconstrained_functions,
            config,
        ));
    }

    for function in &mut graph.functions {
        function
            .lines
            .sort_by_key(|line| (line.span.file.clone(), line.span.start, line.text.clone()));
        function.lines.dedup();
        function.definitions.sort_by_key(|item| {
            (
                item.span.file.clone(),
                item.span.start,
                item.variable.clone(),
            )
        });
        function.sources.sort_by_key(|item| {
            (
                item.span.file.clone(),
                item.span.start,
                item.variable.clone(),
            )
        });
        function.guards.sort_by_key(|item| {
            (
                item.span.file.clone(),
                item.span.start,
                item.variable.clone(),
            )
        });
        function.sinks.sort_by_key(|item| {
            (
                item.span.file.clone(),
                item.span.start,
                format!("{:?}", item.kind),
            )
        });
    }

    graph.functions.sort_by_key(|function| {
        (
            function.contract_id.clone(),
            function.function_symbol_id.clone(),
        )
    });

    graph
}

fn build_source_functions(
    source: &SourceUnit,
    entrypoint_kinds: &BTreeMap<String, Vec<EntrypointKind>>,
    unconstrained_functions: &BTreeSet<String>,
    config: &AztecConfig,
) -> Vec<FunctionGraph> {
    let mut functions = Vec::<FunctionGraph>::new();

    let normalized_file = normalize_file_path(&source.path);
    let mut current_contract: Option<(String, usize)> = None;
    let mut current_function: Option<FunctionBuilder> = None;
    let mut brace_depth = 0usize;
    let mut offset = 0usize;

    for line in source.text.lines() {
        let trimmed = line.trim();
        let (_, code_after_attributes) = split_inline_attributes(trimmed);

        if current_contract.is_none()
            && let Some(contract_name) = is_contract_start(code_after_attributes)
        {
            let contract_id = format!("{normalized_file}::{contract_name}");
            current_contract = Some((contract_id, brace_depth + line.matches('{').count()));
        }

        if let Some((contract_id, _)) = current_contract.clone()
            && let Some(function_name) = is_function_start(code_after_attributes)
        {
            let function_symbol_id = format!("{contract_id}::fn::{function_name}");
            let kinds = entrypoint_kinds
                .get(&function_symbol_id)
                .cloned()
                .unwrap_or_default();
            let is_private_entrypoint = kinds.contains(&EntrypointKind::Private);
            let is_public_entrypoint = kinds.contains(&EntrypointKind::Public);
            let span = line_span(source, offset, line.len());
            let mut builder = FunctionBuilder {
                function: FunctionGraph {
                    contract_id: contract_id.clone(),
                    function_symbol_id: function_symbol_id.clone(),
                    is_private_entrypoint,
                    is_public_entrypoint,
                    lines: vec![LineRecord {
                        text: normalize_line(line).to_string(),
                        span: span.clone(),
                    }],
                    definitions: Vec::new(),
                    sources: Vec::new(),
                    guards: Vec::new(),
                    sinks: Vec::new(),
                },
                end_depth: brace_depth + line.matches('{').count(),
            };

            if is_private_entrypoint {
                for param in parse_params(code_after_attributes) {
                    builder.function.sources.push(TaintSource {
                        variable: param,
                        kind: TaintSourceKind::PrivateEntrypointParam,
                        span: span.clone(),
                    });
                }
            }

            current_function = Some(builder);
        }

        if let Some(builder) = current_function.as_mut() {
            analyze_line(
                source,
                line,
                offset,
                builder,
                unconstrained_functions,
                config,
            );
        }

        brace_depth = update_depth(brace_depth, line);

        if let Some(builder) = current_function.take() {
            if brace_depth < builder.end_depth {
                functions.push(builder.function);
            } else {
                current_function = Some(builder);
            }
        }

        if let Some((_, contract_depth)) = current_contract.clone()
            && brace_depth < contract_depth
        {
            current_contract = None;
        }

        offset += line.len() + 1;
    }

    functions
}

struct FunctionBuilder {
    function: FunctionGraph,
    end_depth: usize,
}

fn analyze_line(
    source: &SourceUnit,
    line: &str,
    offset: usize,
    builder: &mut FunctionBuilder,
    unconstrained_functions: &BTreeSet<String>,
    config: &AztecConfig,
) {
    let normalized = normalize_line(line);
    if normalized.is_empty() {
        return;
    }

    let span = line_span(source, offset, line.len());
    builder.function.lines.push(LineRecord {
        text: normalized.to_string(),
        span: span.clone(),
    });

    if let Some((name, rhs, name_start)) = parse_let_binding(normalized) {
        let dependencies = extract_identifiers(rhs)
            .into_iter()
            .filter(|identifier| identifier != &name)
            .collect::<BTreeSet<_>>();
        builder.function.definitions.push(Definition {
            variable: name.clone(),
            dependencies,
            span: Span::new(
                span.file.clone(),
                span.start + u32::try_from(name_start).unwrap_or(0),
                span.start + u32::try_from(name_start + name.len()).unwrap_or(0),
                span.line,
                span.col + u32::try_from(name_start).unwrap_or(0),
            ),
        });

        if contains_note_read(normalized, config) {
            builder.function.sources.push(TaintSource {
                variable: name.clone(),
                kind: TaintSourceKind::NoteRead,
                span: span.clone(),
            });
        }
        if rhs.contains("self.storage") || rhs.contains("secret") {
            builder.function.sources.push(TaintSource {
                variable: name.clone(),
                kind: TaintSourceKind::SecretState,
                span: span.clone(),
            });
        }
        if unconstrained_functions
            .iter()
            .any(|function| rhs.contains(&format!("{function}(")))
        {
            builder.function.sources.push(TaintSource {
                variable: name,
                kind: TaintSourceKind::UnconstrainedCall,
                span: span.clone(),
            });
        }
    }

    if is_guard_line(normalized) {
        for variable in extract_identifiers(normalized) {
            builder.function.guards.push(GuardSite {
                variable,
                span: span.clone(),
            });
        }
    }

    for sink_kind in detect_sink_kinds(normalized) {
        builder.function.sinks.push(SinkSite {
            kind: sink_kind,
            identifiers: sink_identifiers(sink_kind, normalized),
            span: span.clone(),
            line: normalized.to_string(),
        });
    }
}

fn collect_unconstrained_functions(sources: &[SourceUnit]) -> BTreeSet<String> {
    let mut names = BTreeSet::<String>::new();

    for source in sources {
        for line in source.text.lines() {
            let normalized = normalize_line(line);
            if let Some(index) = normalized.find("unconstrained fn ") {
                let tail = &normalized[index + "unconstrained fn ".len()..];
                if let Some(name) = extract_first_identifier(tail) {
                    names.insert(name);
                }
            }
        }
    }

    names
}

fn parse_let_binding(line: &str) -> Option<(String, &str, usize)> {
    let bytes = line.as_bytes();
    let marker = line.find("let ")?;
    if marker > 0 && is_ident_continue(bytes[marker - 1]) {
        return None;
    }

    let mut cursor = marker + "let ".len();
    if line[cursor..].starts_with("mut ") {
        cursor += "mut ".len();
    }

    let name_start = cursor;
    while cursor < bytes.len() && is_ident_continue(bytes[cursor]) {
        cursor += 1;
    }
    if name_start == cursor {
        return None;
    }
    let name = line[name_start..cursor].to_string();
    let rhs_start = line[cursor..].find('=')? + cursor + 1;

    Some((name, line[rhs_start..].trim(), name_start))
}

fn parse_params(line: &str) -> Vec<String> {
    let Some(open) = line.find('(') else {
        return Vec::new();
    };
    let Some(close) = line[open + 1..].find(')') else {
        return Vec::new();
    };
    let body = &line[open + 1..open + 1 + close];
    body.split(',')
        .filter_map(|segment| {
            let name = segment
                .split(':')
                .next()?
                .trim()
                .trim_start_matches("mut ")
                .trim();
            if name.is_empty() || name == "self" {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

fn split_inline_attributes(line: &str) -> (Vec<String>, &str) {
    let mut attrs = Vec::<String>::new();
    let mut remaining = line.trim_start();

    while remaining.starts_with("#[") {
        let Some(close) = remaining.find(']') else {
            break;
        };
        attrs.push(remaining[..=close].trim().to_string());
        remaining = remaining[close + 1..].trim_start();
    }

    (attrs, remaining)
}

fn line_span(source: &SourceUnit, offset: usize, line_len: usize) -> Span {
    let line = source.text[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    Span::new(
        normalize_file_path(&source.path),
        u32::try_from(offset).unwrap_or(u32::MAX),
        u32::try_from(offset + line_len).unwrap_or(u32::MAX),
        u32::try_from(line).unwrap_or(u32::MAX),
        1,
    )
}

fn update_depth(current: usize, line: &str) -> usize {
    let opens = line.bytes().filter(|byte| *byte == b'{').count();
    let closes = line.bytes().filter(|byte| *byte == b'}').count();
    current.saturating_add(opens).saturating_sub(closes)
}

fn detect_sink_kinds(line: &str) -> Vec<TaintSinkKind> {
    let mut sinks = Vec::<TaintSinkKind>::new();

    let trimmed = line.trim_start();
    if trimmed.starts_with("if ")
        || trimmed.starts_with("if(")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("match ")
    {
        sinks.push(TaintSinkKind::BranchCondition);
    }
    if line.contains("emit(") || line.contains("return ") {
        sinks.push(TaintSinkKind::PublicOutput);
    }
    if line.contains("debug_log(") {
        sinks.push(TaintSinkKind::DebugLog);
        sinks.push(TaintSinkKind::LogEvent);
    }
    if line.contains("self.storage")
        && (line.contains("=") || line.contains(".write(") || line.contains(".set("))
    {
        sinks.push(TaintSinkKind::PublicStorageWrite);
    }
    if line.contains("enqueue(") || line.contains("enqueue_self") {
        sinks.push(TaintSinkKind::EnqueuePublicCall);
    }
    if line.contains("oracle") && line.contains('(') {
        sinks.push(TaintSinkKind::OracleArgument);
    }
    if line.contains("emit_nullifier(") || line.contains("nullify(") || line.contains(".insert(") {
        sinks.push(TaintSinkKind::NullifierOrCommitment);
    }
    if line.contains("hash(")
        || line.contains("pedersen(")
        || line.contains("serialize(")
        || line.contains("to_bytes(")
    {
        sinks.push(TaintSinkKind::HashOrSerialize);
    }
    if line.contains("merkle") || line.contains("witness") {
        sinks.push(TaintSinkKind::MerkleWitness);
    }

    sinks
}

fn sink_identifiers(kind: TaintSinkKind, line: &str) -> BTreeSet<String> {
    match kind {
        TaintSinkKind::HashOrSerialize => {
            let segment = line
                .split_once('=')
                .map_or(line, |(_, rhs)| rhs)
                .trim_start();
            extract_identifiers(segment)
        }
        TaintSinkKind::BranchCondition => {
            let segment = line
                .split_once('{')
                .map_or(line, |(head, _)| head)
                .trim_end();
            extract_identifiers(segment)
        }
        _ => extract_identifiers(line),
    }
}

fn is_guard_line(line: &str) -> bool {
    (line.contains("assert(") || line.contains("constrain(") || line.contains("range"))
        && (line.contains('<') || line.contains("<="))
}

fn extract_identifiers(line: &str) -> BTreeSet<String> {
    let bytes = line.as_bytes();
    let mut cursor = 0usize;
    let mut identifiers = BTreeSet::<String>::new();

    while cursor < bytes.len() {
        if !is_ident_start(bytes[cursor]) {
            cursor += 1;
            continue;
        }
        let start = cursor;
        cursor += 1;
        while cursor < bytes.len() && is_ident_continue(bytes[cursor]) {
            cursor += 1;
        }
        let candidate = line[start..cursor].to_string();
        if !is_keyword(&candidate) {
            identifiers.insert(candidate);
        }
    }

    identifiers
}

fn extract_first_identifier(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if !is_ident_start(bytes[cursor]) {
            cursor += 1;
            continue;
        }
        let start = cursor;
        cursor += 1;
        while cursor < bytes.len() && is_ident_continue(bytes[cursor]) {
            cursor += 1;
        }
        let candidate = &line[start..cursor];
        if !is_keyword(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_keyword(token: &str) -> bool {
    matches!(
        token,
        "let"
            | "mut"
            | "fn"
            | "if"
            | "while"
            | "match"
            | "return"
            | "assert"
            | "constrain"
            | "self"
            | "true"
            | "false"
            | "pub"
            | "contract"
    )
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::config::AztecConfig;

    use crate::detect::SourceUnit;
    use crate::model_builder::build_aztec_model;

    use super::{TaintSinkKind, TaintSourceKind, build_def_use_graph};

    #[test]
    fn captures_private_params_and_note_read_sources() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        let notes = self.notes.get_notes();
        let combined = secret + notes;
        emit(combined);
    }
}
"#;
        let sources = vec![SourceUnit::new("src/main.nr", source)];
        let model = build_aztec_model(&sources, &AztecConfig::default());
        let graph = build_def_use_graph(&sources, &model, &AztecConfig::default());

        let function = &graph.functions[0];
        assert!(
            function
                .sources
                .iter()
                .any(|item| item.kind == TaintSourceKind::PrivateEntrypointParam)
        );
        assert!(
            function
                .sources
                .iter()
                .any(|item| item.kind == TaintSourceKind::NoteRead)
        );
        assert!(
            function
                .sinks
                .iter()
                .any(|item| item.kind == TaintSinkKind::PublicOutput)
        );
    }

    #[test]
    fn parses_mut_private_params_as_sources() {
        let source = r#"
#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(mut secret: Field) {
        emit(secret);
    }
}
"#;
        let sources = vec![SourceUnit::new("src/main.nr", source)];
        let model = build_aztec_model(&sources, &AztecConfig::default());
        let graph = build_def_use_graph(&sources, &model, &AztecConfig::default());

        let function = &graph.functions[0];
        assert!(function.sources.iter().any(|item| {
            item.kind == TaintSourceKind::PrivateEntrypointParam && item.variable == "secret"
        }));
    }
}
