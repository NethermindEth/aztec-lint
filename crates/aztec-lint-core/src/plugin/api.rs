use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

pub use aztec_lint_sdk::{
    ApiVersion, PluginConfidence, PluginDescriptor, PluginDiagnostic, PluginFix, PluginFixSafety,
    PluginInput, PluginOutput, PluginRuleMetadata, PluginSeverity, PluginSourceFile, PluginSpan,
    RULE_API_VERSION, RulePlugin, host_accepts_plugin,
};

pub const HOST_RULE_API_VERSION: ApiVersion = RULE_API_VERSION;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginLoadSource {
    WasmFile(PathBuf),
    WasmBytes(Vec<u8>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SandboxFilesystemPolicy {
    DenyAll,
    ReadOnlyWorkspace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SandboxNetworkPolicy {
    DenyAll,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SandboxClockPolicy {
    MonotonicOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxPolicy {
    pub max_memory_bytes: u64,
    pub max_instructions: u64,
    pub max_execution_ms: u64,
    pub filesystem: SandboxFilesystemPolicy,
    pub network: SandboxNetworkPolicy,
    pub clock: SandboxClockPolicy,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            max_memory_bytes: 64 * 1024 * 1024,
            max_instructions: 100_000_000,
            max_execution_ms: 2_000,
            filesystem: SandboxFilesystemPolicy::ReadOnlyWorkspace,
            network: SandboxNetworkPolicy::DenyAll,
            clock: SandboxClockPolicy::MonotonicOnly,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PluginApiError {
    EmptyPluginId,
    InvalidPluginId {
        plugin_id: String,
    },
    DuplicatePluginId {
        plugin_id: String,
    },
    IncompatibleApiVersion {
        plugin_id: String,
        plugin_api: ApiVersion,
        host_api: ApiVersion,
    },
}

impl Display for PluginApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPluginId => write!(f, "plugin id must not be empty"),
            Self::InvalidPluginId { plugin_id } => write!(
                f,
                "plugin id '{plugin_id}' is invalid (use lowercase ascii letters, digits, '.', '_' or '-', without surrounding whitespace)"
            ),
            Self::DuplicatePluginId { plugin_id } => {
                write!(f, "plugin id '{plugin_id}' is already registered")
            }
            Self::IncompatibleApiVersion {
                plugin_id,
                plugin_api,
                host_api,
            } => write!(
                f,
                "plugin '{plugin_id}' targets API {}.{} but host supports {}.{}",
                plugin_api.major, plugin_api.minor, host_api.major, host_api.minor
            ),
        }
    }
}

impl Error for PluginApiError {}

pub trait PluginLoader {
    fn load_plugin(
        &self,
        source: &PluginLoadSource,
        sandbox: &SandboxPolicy,
    ) -> Result<Box<dyn RulePlugin>, PluginApiError>;
}

#[derive(Default)]
pub struct PluginRegistry {
    descriptors: Vec<PluginDescriptor>,
    plugins: Vec<Box<dyn RulePlugin>>,
    sandbox_policy: SandboxPolicy,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_sandbox_policy(sandbox_policy: SandboxPolicy) -> Self {
        Self {
            descriptors: Vec::new(),
            plugins: Vec::new(),
            sandbox_policy,
        }
    }

    pub fn register(&mut self, plugin: Box<dyn RulePlugin>) -> Result<(), PluginApiError> {
        let descriptor = plugin.descriptor();
        validate_descriptor(&descriptor)?;
        if self
            .descriptors
            .iter()
            .any(|existing| existing.plugin_id == descriptor.plugin_id)
        {
            return Err(PluginApiError::DuplicatePluginId {
                plugin_id: descriptor.plugin_id,
            });
        }
        self.descriptors.push(descriptor);
        self.plugins.push(plugin);
        Ok(())
    }

    pub fn load_and_register<L: PluginLoader>(
        &mut self,
        loader: &L,
        source: &PluginLoadSource,
    ) -> Result<(), PluginApiError> {
        let plugin = loader.load_plugin(source, &self.sandbox_policy)?;
        self.register(plugin)
    }

    pub fn descriptors(&self) -> &[PluginDescriptor] {
        &self.descriptors
    }

    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    pub fn sandbox_policy(&self) -> &SandboxPolicy {
        &self.sandbox_policy
    }
}

pub fn validate_descriptor(descriptor: &PluginDescriptor) -> Result<(), PluginApiError> {
    let plugin_id = descriptor.plugin_id.as_str();
    let trimmed = plugin_id.trim();
    if plugin_id.is_empty() {
        return Err(PluginApiError::EmptyPluginId);
    }
    if trimmed != plugin_id || !is_valid_plugin_id(trimmed) {
        return Err(PluginApiError::InvalidPluginId {
            plugin_id: plugin_id.to_string(),
        });
    }
    if !host_accepts_plugin(HOST_RULE_API_VERSION, descriptor.api_version) {
        return Err(PluginApiError::IncompatibleApiVersion {
            plugin_id: trimmed.to_string(),
            plugin_api: descriptor.api_version,
            host_api: HOST_RULE_API_VERSION,
        });
    }
    Ok(())
}

pub fn host_accepts_api_version(plugin_api: ApiVersion) -> bool {
    host_accepts_plugin(HOST_RULE_API_VERSION, plugin_api)
}

fn is_valid_plugin_id(plugin_id: &str) -> bool {
    !plugin_id.is_empty()
        && plugin_id.chars().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '-')
        })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        ApiVersion, PluginApiError, PluginDescriptor, PluginInput, PluginOutput, PluginRegistry,
        PluginRuleMetadata, PluginSeverity, RulePlugin, SandboxFilesystemPolicy,
        SandboxNetworkPolicy, host_accepts_api_version,
    };

    struct MockPlugin {
        id: &'static str,
        api: ApiVersion,
    }

    impl RulePlugin for MockPlugin {
        fn descriptor(&self) -> PluginDescriptor {
            PluginDescriptor {
                plugin_id: self.id.to_string(),
                display_name: "Mock Plugin".to_string(),
                plugin_version: "0.1.0".to_string(),
                api_version: self.api,
                description: None,
            }
        }

        fn rules(&self) -> Vec<PluginRuleMetadata> {
            vec![PluginRuleMetadata {
                rule_id: "MOCK001".to_string(),
                summary: "mock".to_string(),
                policy: "maintainability".to_string(),
                default_severity: PluginSeverity::Warning,
                confidence: aztec_lint_sdk::PluginConfidence::Low,
            }]
        }

        fn analyze(&self, _input: &PluginInput) -> PluginOutput {
            PluginOutput::default()
        }
    }

    #[test]
    fn registry_accepts_compatible_plugin_version() {
        let mut registry = PluginRegistry::new();
        registry
            .register(Box::new(MockPlugin {
                id: "mock.plugin",
                api: super::HOST_RULE_API_VERSION,
            }))
            .expect("compatible plugin should register");

        assert_eq!(registry.plugin_count(), 1);
        assert_eq!(registry.descriptors()[0].plugin_id, "mock.plugin");
    }

    #[test]
    fn registry_rejects_incompatible_major_version() {
        let mut registry = PluginRegistry::new();
        let err = registry
            .register(Box::new(MockPlugin {
                id: "mock.plugin",
                api: ApiVersion::new(super::HOST_RULE_API_VERSION.major + 1, 0),
            }))
            .expect_err("incompatible plugin must fail");

        assert!(matches!(err, PluginApiError::IncompatibleApiVersion { .. }));
    }

    #[test]
    fn registry_rejects_duplicate_plugin_ids() {
        let mut registry = PluginRegistry::new();
        registry
            .register(Box::new(MockPlugin {
                id: "mock.plugin",
                api: super::HOST_RULE_API_VERSION,
            }))
            .expect("first registration should pass");

        let err = registry
            .register(Box::new(MockPlugin {
                id: "mock.plugin",
                api: super::HOST_RULE_API_VERSION,
            }))
            .expect_err("duplicate id should fail");
        assert!(matches!(err, PluginApiError::DuplicatePluginId { .. }));
    }

    #[test]
    fn registry_rejects_plugin_id_with_surrounding_whitespace() {
        let mut registry = PluginRegistry::new();
        let err = registry
            .register(Box::new(MockPlugin {
                id: " mock.plugin ",
                api: super::HOST_RULE_API_VERSION,
            }))
            .expect_err("whitespace-wrapped id should fail");
        assert!(matches!(err, PluginApiError::InvalidPluginId { .. }));
    }

    #[test]
    fn registry_rejects_plugin_id_with_invalid_characters() {
        let mut registry = PluginRegistry::new();
        let err = registry
            .register(Box::new(MockPlugin {
                id: "mock plugin",
                api: super::HOST_RULE_API_VERSION,
            }))
            .expect_err("id with spaces should fail");
        assert!(matches!(err, PluginApiError::InvalidPluginId { .. }));
    }

    #[test]
    fn default_sandbox_policy_is_restrictive() {
        let registry = PluginRegistry::new();
        assert_eq!(
            registry.sandbox_policy().filesystem,
            SandboxFilesystemPolicy::ReadOnlyWorkspace
        );
        assert_eq!(
            registry.sandbox_policy().network,
            SandboxNetworkPolicy::DenyAll
        );
    }

    #[test]
    fn compatibility_contract_rejects_future_minor_version() {
        let future = ApiVersion::new(
            super::HOST_RULE_API_VERSION.major,
            super::HOST_RULE_API_VERSION.minor + 1,
        );
        assert!(!host_accepts_api_version(future));
        assert!(host_accepts_api_version(ApiVersion::new(
            super::HOST_RULE_API_VERSION.major,
            0
        )));
    }

    #[test]
    fn mock_plugin_input_shape_is_sdk_decoupled() {
        let input = PluginInput {
            files: Vec::new(),
            config: BTreeMap::new(),
            include_suppressed: false,
        };
        let output = MockPlugin {
            id: "mock.plugin",
            api: super::HOST_RULE_API_VERSION,
        }
        .analyze(&input);
        assert!(output.diagnostics.is_empty());
    }
}
