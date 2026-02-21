use serde::{Deserialize, Serialize};

use crate::model::{SemanticModel, Span};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectModel {
    pub ast_ids: Vec<String>,
    pub symbols: Vec<SymbolRef>,
    pub type_refs: Vec<TypeRef>,
    pub call_graph: Vec<CallEdge>,
    pub module_graph: Vec<ModuleEdge>,
    #[serde(default)]
    pub semantic: SemanticModel,
}

impl ProjectModel {
    pub fn normalize(&mut self) {
        self.ast_ids.sort();
        self.ast_ids.dedup();

        self.symbols.sort_by_key(|symbol| {
            (
                symbol.span.file.clone(),
                symbol.span.start,
                symbol.span.end,
                symbol.name.clone(),
                symbol.symbol_id.clone(),
            )
        });
        self.symbols
            .dedup_by(|left, right| left.symbol_id == right.symbol_id);

        self.type_refs
            .sort_by_key(|type_ref| (type_ref.symbol_id.clone(), type_ref.type_repr.clone()));
        self.type_refs.dedup();

        self.call_graph.sort_by_key(|edge| {
            (
                edge.caller_symbol_id.clone(),
                edge.callee_symbol_id.clone(),
                edge.span.file.clone(),
                edge.span.start,
                edge.span.end,
            )
        });
        self.call_graph.dedup();

        self.module_graph
            .sort_by_key(|edge| (edge.from_module.clone(), edge.to_module.clone()));
        self.module_graph.dedup();

        self.semantic.normalize();
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SymbolRef {
    pub symbol_id: String,
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Struct,
    Module,
    Import,
    Local,
    Global,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TypeRef {
    pub symbol_id: String,
    pub type_repr: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CallEdge {
    pub caller_symbol_id: String,
    pub callee_symbol_id: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModuleEdge {
    pub from_module: String,
    pub to_module: String,
}

#[cfg(test)]
mod tests {
    use serde_json::from_str;
    use serde_json::to_vec;

    use super::{ModuleEdge, ProjectModel, SymbolKind, SymbolRef, TypeRef};
    use crate::model::Span;
    use crate::model::{ExpressionCategory, SemanticExpression, SemanticFunction, TypeCategory};

    #[test]
    fn deserialize_legacy_project_model_defaults_semantic() {
        let project = from_str::<ProjectModel>(
            r#"{
  "ast_ids": ["src/main.nr"],
  "symbols": [],
  "type_refs": [],
  "call_graph": [],
  "module_graph": []
}"#,
        )
        .expect("legacy shape should deserialize");
        assert!(project.semantic.functions.is_empty());
        assert!(project.semantic.expressions.is_empty());
    }

    #[test]
    fn normalize_orders_legacy_and_semantic_sections_deterministically() {
        let mut left = ProjectModel {
            ast_ids: vec![
                "src/b.nr".to_string(),
                "src/a.nr".to_string(),
                "src/a.nr".to_string(),
            ],
            symbols: vec![
                SymbolRef {
                    symbol_id: "fn::b".to_string(),
                    name: "b".to_string(),
                    kind: SymbolKind::Function,
                    span: Span::new("src/main.nr", 20, 21, 2, 1),
                },
                SymbolRef {
                    symbol_id: "fn::a".to_string(),
                    name: "a".to_string(),
                    kind: SymbolKind::Function,
                    span: Span::new("src/main.nr", 10, 11, 1, 1),
                },
                SymbolRef {
                    symbol_id: "fn::a".to_string(),
                    name: "a".to_string(),
                    kind: SymbolKind::Function,
                    span: Span::new("src/main.nr", 10, 11, 1, 1),
                },
            ],
            type_refs: vec![
                TypeRef {
                    symbol_id: "fn::b".to_string(),
                    type_repr: "Field".to_string(),
                },
                TypeRef {
                    symbol_id: "fn::a".to_string(),
                    type_repr: "Field".to_string(),
                },
                TypeRef {
                    symbol_id: "fn::a".to_string(),
                    type_repr: "Field".to_string(),
                },
            ],
            call_graph: Vec::new(),
            module_graph: vec![
                ModuleEdge {
                    from_module: "z".to_string(),
                    to_module: "a".to_string(),
                },
                ModuleEdge {
                    from_module: "a".to_string(),
                    to_module: "z".to_string(),
                },
                ModuleEdge {
                    from_module: "a".to_string(),
                    to_module: "z".to_string(),
                },
            ],
            semantic: crate::model::SemanticModel {
                functions: vec![
                    SemanticFunction {
                        symbol_id: "fn::b".to_string(),
                        name: "b".to_string(),
                        module_symbol_id: "module::z".to_string(),
                        return_type_repr: "Field".to_string(),
                        return_type_category: TypeCategory::Field,
                        parameter_types: vec!["u32".to_string(), "Field".to_string()],
                        is_entrypoint: false,
                        is_unconstrained: false,
                        span: Span::new("src/main.nr", 20, 21, 2, 1),
                    },
                    SemanticFunction {
                        symbol_id: "fn::a".to_string(),
                        name: "a".to_string(),
                        module_symbol_id: "module::z".to_string(),
                        return_type_repr: "Field".to_string(),
                        return_type_category: TypeCategory::Field,
                        parameter_types: vec!["Field".to_string()],
                        is_entrypoint: true,
                        is_unconstrained: false,
                        span: Span::new("src/main.nr", 10, 11, 1, 1),
                    },
                    SemanticFunction {
                        symbol_id: "fn::a".to_string(),
                        name: "a".to_string(),
                        module_symbol_id: "module::z".to_string(),
                        return_type_repr: "Field".to_string(),
                        return_type_category: TypeCategory::Field,
                        parameter_types: vec!["Field".to_string()],
                        is_entrypoint: true,
                        is_unconstrained: false,
                        span: Span::new("src/main.nr", 10, 11, 1, 1),
                    },
                ],
                expressions: vec![
                    SemanticExpression {
                        expr_id: "expr::2".to_string(),
                        function_symbol_id: "fn::a".to_string(),
                        category: ExpressionCategory::Call,
                        type_category: TypeCategory::Field,
                        type_repr: "Field".to_string(),
                        span: Span::new("src/main.nr", 30, 31, 3, 1),
                    },
                    SemanticExpression {
                        expr_id: "expr::1".to_string(),
                        function_symbol_id: "fn::a".to_string(),
                        category: ExpressionCategory::Literal,
                        type_category: TypeCategory::Field,
                        type_repr: "Field".to_string(),
                        span: Span::new("src/main.nr", 29, 30, 3, 1),
                    },
                    SemanticExpression {
                        expr_id: "expr::1".to_string(),
                        function_symbol_id: "fn::a".to_string(),
                        category: ExpressionCategory::Literal,
                        type_category: TypeCategory::Field,
                        type_repr: "Field".to_string(),
                        span: Span::new("src/main.nr", 29, 30, 3, 1),
                    },
                ],
                ..Default::default()
            },
        };

        let mut right = left.clone();
        right.ast_ids.reverse();
        right.symbols.reverse();
        right.semantic.functions.reverse();
        right.semantic.expressions.reverse();
        right.module_graph.reverse();

        left.normalize();
        right.normalize();

        assert_eq!(left, right);
        assert_eq!(
            left.ast_ids,
            vec!["src/a.nr".to_string(), "src/b.nr".to_string()]
        );
        assert_eq!(left.symbols.len(), 2);
        assert_eq!(left.semantic.functions.len(), 2);
        assert_eq!(left.semantic.expressions.len(), 2);
        assert_eq!(left.module_graph.len(), 2);

        let left_json = to_vec(&left).expect("serialization should succeed");
        let right_json = to_vec(&right).expect("serialization should succeed");
        assert_eq!(left_json, right_json);
    }
}
