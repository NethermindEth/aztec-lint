#![cfg(feature = "plugin-api")]

use std::collections::BTreeMap;

use aztec_lint_core::plugin::api::{
    PluginConfidence, PluginDescriptor, PluginInput, PluginOutput, PluginRegistry,
    PluginRuleMetadata, PluginSeverity, RulePlugin,
};

struct MockExternalPlugin;

impl RulePlugin for MockExternalPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        PluginDescriptor {
            plugin_id: "integration.mock".to_string(),
            display_name: "Integration Mock".to_string(),
            plugin_version: "0.1.0".to_string(),
            api_version: aztec_lint_core::plugin::api::HOST_RULE_API_VERSION,
            description: Some("Integration-test plugin crate".to_string()),
        }
    }

    fn rules(&self) -> Vec<PluginRuleMetadata> {
        vec![PluginRuleMetadata {
            rule_id: "IMOCK001".to_string(),
            summary: "integration mock".to_string(),
            policy: "maintainability".to_string(),
            default_severity: PluginSeverity::Warning,
            confidence: PluginConfidence::Low,
        }]
    }

    fn analyze(&self, _input: &PluginInput) -> PluginOutput {
        PluginOutput::default()
    }
}

#[test]
fn mock_plugin_crate_compiles_and_registers() {
    let mut registry = PluginRegistry::new();
    registry
        .register(Box::new(MockExternalPlugin))
        .expect("plugin registration should succeed");

    let input = PluginInput {
        files: Vec::new(),
        config: BTreeMap::new(),
        include_suppressed: false,
    };
    let output = registry
        .descriptors()
        .first()
        .expect("descriptor should be present")
        .plugin_id
        .clone();

    assert_eq!(output, "integration.mock");
    let analyzed = MockExternalPlugin.analyze(&input);
    assert!(analyzed.diagnostics.is_empty());
}
