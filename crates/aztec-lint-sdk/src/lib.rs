#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const RULE_API_VERSION: ApiVersion = ApiVersion::new(0, 1);
pub const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ApiVersion {
    pub major: u16,
    pub minor: u16,
}

impl ApiVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    pub const fn is_compatible_with_host(self, host: Self) -> bool {
        self.major == host.major && self.minor <= host.minor
    }
}

pub const fn host_accepts_plugin(host: ApiVersion, plugin: ApiVersion) -> bool {
    plugin.is_compatible_with_host(host)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginDescriptor {
    pub plugin_id: String,
    pub display_name: String,
    pub plugin_version: String,
    pub api_version: ApiVersion,
    pub description: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginSeverity {
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginConfidence {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginFixSafety {
    Safe,
    NeedsReview,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginSpan {
    pub file: String,
    pub start: u32,
    pub end: u32,
    pub line: u32,
    pub col: u32,
}

impl PluginSpan {
    pub fn new(file: impl Into<String>, start: u32, end: u32, line: u32, col: u32) -> Self {
        Self {
            file: file.into(),
            start,
            end,
            line,
            col,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginFix {
    pub description: String,
    pub span: PluginSpan,
    pub replacement: String,
    pub safety: PluginFixSafety,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginDiagnostic {
    pub rule_id: String,
    pub severity: PluginSeverity,
    pub confidence: PluginConfidence,
    pub policy: String,
    pub message: String,
    pub primary_span: PluginSpan,
    pub secondary_spans: Vec<PluginSpan>,
    pub suggestions: Vec<String>,
    pub fixes: Vec<PluginFix>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginRuleMetadata {
    pub rule_id: String,
    pub summary: String,
    pub policy: String,
    pub default_severity: PluginSeverity,
    pub confidence: PluginConfidence,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginSourceFile {
    pub path: String,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginInput {
    pub files: Vec<PluginSourceFile>,
    pub config: BTreeMap<String, String>,
    pub include_suppressed: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginOutput {
    pub diagnostics: Vec<PluginDiagnostic>,
}

pub trait RulePlugin: Send + Sync {
    fn descriptor(&self) -> PluginDescriptor;
    fn rules(&self) -> Vec<PluginRuleMetadata>;
    fn analyze(&self, input: &PluginInput) -> PluginOutput;
}

#[cfg(test)]
mod tests {
    use super::{
        ApiVersion, PluginConfidence, PluginDiagnostic, PluginOutput, PluginSeverity, PluginSpan,
        RULE_API_VERSION, host_accepts_plugin,
    };

    #[test]
    fn api_compatibility_requires_same_major_and_supported_minor() {
        let host = RULE_API_VERSION;
        assert!(host_accepts_plugin(
            host,
            ApiVersion::new(host.major, host.minor)
        ));
        assert!(host_accepts_plugin(host, ApiVersion::new(host.major, 0)));
        assert!(!host_accepts_plugin(
            host,
            ApiVersion::new(host.major + 1, 0)
        ));
        assert!(!host_accepts_plugin(
            host,
            ApiVersion::new(host.major, host.minor + 1)
        ));
    }

    #[test]
    fn plugin_diagnostic_json_contract_is_stable() {
        let diagnostic = PluginDiagnostic {
            rule_id: "PLUGIN001".to_string(),
            severity: PluginSeverity::Warning,
            confidence: PluginConfidence::Medium,
            policy: "privacy".to_string(),
            message: "message".to_string(),
            primary_span: PluginSpan::new("src/main.nr", 1, 2, 1, 1),
            secondary_spans: Vec::new(),
            suggestions: vec!["suggestion".to_string()],
            fixes: Vec::new(),
        };

        let output = PluginOutput {
            diagnostics: vec![diagnostic],
        };

        let rendered = serde_json::to_string_pretty(&output)
            .expect("plugin output serialization should succeed");
        let expected = r#"{
  "diagnostics": [
    {
      "rule_id": "PLUGIN001",
      "severity": "warning",
      "confidence": "medium",
      "policy": "privacy",
      "message": "message",
      "primary_span": {
        "file": "src/main.nr",
        "start": 1,
        "end": 2,
        "line": 1,
        "col": 1
      },
      "secondary_spans": [],
      "suggestions": [
        "suggestion"
      ],
      "fixes": []
    }
  ]
}"#;

        assert_eq!(rendered, expected);
    }
}
