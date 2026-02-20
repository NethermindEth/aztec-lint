use std::collections::BTreeMap;

use aztec_lint_sdk::{
    PluginConfidence, PluginDescriptor, PluginInput, PluginOutput, PluginRuleMetadata,
    PluginSeverity, RULE_API_VERSION, RulePlugin,
};

struct MockPlugin;

impl RulePlugin for MockPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        PluginDescriptor {
            plugin_id: "mock.plugin".to_string(),
            display_name: "Mock Plugin".to_string(),
            plugin_version: "0.1.0".to_string(),
            api_version: RULE_API_VERSION,
            description: Some("Compile-only plugin used for API integration checks".to_string()),
        }
    }

    fn rules(&self) -> Vec<PluginRuleMetadata> {
        vec![PluginRuleMetadata {
            rule_id: "MOCK001".to_string(),
            summary: "Mock rule".to_string(),
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
fn mock_plugin_crate_compiles_against_sdk_contract() {
    let plugin = MockPlugin;
    let descriptor = plugin.descriptor();
    assert_eq!(descriptor.plugin_id, "mock.plugin");

    let input = PluginInput {
        files: Vec::new(),
        config: BTreeMap::new(),
        include_suppressed: false,
    };
    let output = plugin.analyze(&input);
    assert!(output.diagnostics.is_empty());
}
