use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::config::ConfigError;
use crate::lints::{all_lints, find_lint};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawConfig {
    #[serde(default)]
    pub profile: BTreeMap<String, Profile>,
    #[serde(default)]
    pub aztec: AztecConfig,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub profile: BTreeMap<String, Profile>,
    pub aztec: AztecConfig,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResolvedProfile {
    pub name: String,
    pub rulesets: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub extends: Vec<String>,
    #[serde(default)]
    pub ruleset: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleLevel {
    Allow,
    Warn,
    Deny,
}

impl Display for RuleLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allow => write!(f, "allow"),
            Self::Warn => write!(f, "warn"),
            Self::Deny => write!(f, "deny"),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuleOverrides {
    pub deny: Vec<String>,
    pub warn: Vec<String>,
    pub allow: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AztecConfig {
    #[serde(default = "default_contract_attribute")]
    pub contract_attribute: String,
    #[serde(default = "default_external_attribute")]
    pub external_attribute: String,
    #[serde(default = "default_external_kinds")]
    pub external_kinds: Vec<String>,
    #[serde(default = "default_only_self_attribute")]
    pub only_self_attribute: String,
    #[serde(default = "default_initializer_attribute")]
    pub initializer_attribute: String,
    #[serde(default = "default_storage_attribute")]
    pub storage_attribute: String,
    #[serde(default = "default_imports_prefixes")]
    pub imports_prefixes: Vec<String>,
    #[serde(default = "default_note_getter_fns")]
    pub note_getter_fns: Vec<String>,
    #[serde(default = "default_nullifier_fns")]
    pub nullifier_fns: Vec<String>,
    #[serde(default = "default_enqueue_fn")]
    pub enqueue_fn: String,
    #[serde(default = "default_contract_at_fn")]
    pub contract_at_fn: String,
    #[serde(default)]
    pub domain_separation: DomainSeparationConfig,
}

impl Default for AztecConfig {
    fn default() -> Self {
        Self {
            contract_attribute: default_contract_attribute(),
            external_attribute: default_external_attribute(),
            external_kinds: default_external_kinds(),
            only_self_attribute: default_only_self_attribute(),
            initializer_attribute: default_initializer_attribute(),
            storage_attribute: default_storage_attribute(),
            imports_prefixes: default_imports_prefixes(),
            note_getter_fns: default_note_getter_fns(),
            nullifier_fns: default_nullifier_fns(),
            enqueue_fn: default_enqueue_fn(),
            contract_at_fn: default_contract_at_fn(),
            domain_separation: DomainSeparationConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DomainSeparationConfig {
    #[serde(default = "default_nullifier_requires")]
    pub nullifier_requires: Vec<String>,
    #[serde(default = "default_commitment_requires")]
    pub commitment_requires: Vec<String>,
}

impl Default for DomainSeparationConfig {
    fn default() -> Self {
        Self {
            nullifier_requires: default_nullifier_requires(),
            commitment_requires: default_commitment_requires(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            profile: builtin_profiles(),
            aztec: AztecConfig::default(),
        }
    }
}

impl Config {
    pub fn from_raw(raw: RawConfig) -> Self {
        let mut profile = builtin_profiles();
        for (name, profile_config) in raw.profile {
            profile.insert(name, profile_config);
        }
        Self {
            profile,
            aztec: raw.aztec,
        }
    }

    pub fn resolve_profile(&self, profile_name: &str) -> Result<ResolvedProfile, ConfigError> {
        if !self.profile.contains_key(profile_name) {
            return Err(ConfigError::ProfileNotFound {
                profile: profile_name.to_string(),
            });
        }

        let mut stack = Vec::<String>::new();
        let mut cache = BTreeMap::<String, Vec<String>>::new();
        let rulesets = self.resolve_profile_inner(profile_name, &mut stack, &mut cache)?;
        Ok(ResolvedProfile {
            name: profile_name.to_string(),
            rulesets,
        })
    }

    pub fn effective_rule_levels(
        &self,
        profile_name: &str,
        overrides: &RuleOverrides,
    ) -> Result<BTreeMap<String, RuleLevel>, ConfigError> {
        let resolved = self.resolve_profile(profile_name)?;
        let mut levels = BTreeMap::<String, RuleLevel>::new();

        for ruleset in &resolved.rulesets {
            let defaults =
                ruleset_defaults(ruleset).ok_or_else(|| ConfigError::UnknownRuleset {
                    ruleset: ruleset.clone(),
                })?;
            for (rule_id, level) in defaults {
                levels.insert(rule_id.to_string(), level);
            }
        }

        apply_rule_overrides(&mut levels, overrides)?;
        Ok(levels)
    }

    fn resolve_profile_inner(
        &self,
        profile_name: &str,
        stack: &mut Vec<String>,
        cache: &mut BTreeMap<String, Vec<String>>,
    ) -> Result<Vec<String>, ConfigError> {
        if let Some(existing) = cache.get(profile_name) {
            return Ok(existing.clone());
        }

        if let Some(start) = stack.iter().position(|item| item == profile_name) {
            let mut cycle = stack[start..].to_vec();
            cycle.push(profile_name.to_string());
            return Err(ConfigError::ProfileCycle { cycle });
        }

        let profile =
            self.profile
                .get(profile_name)
                .ok_or_else(|| ConfigError::ProfileNotFound {
                    profile: profile_name.to_string(),
                })?;

        stack.push(profile_name.to_string());

        let mut merged_rulesets = Vec::<String>::new();
        for parent in &profile.extends {
            if !self.profile.contains_key(parent) {
                return Err(ConfigError::ParentProfileNotFound {
                    profile: profile_name.to_string(),
                    parent: parent.clone(),
                });
            }

            let parent_rulesets = self.resolve_profile_inner(parent, stack, cache)?;
            append_unique(&mut merged_rulesets, &parent_rulesets);
        }
        append_unique(&mut merged_rulesets, &profile.ruleset);

        stack.pop();
        cache.insert(profile_name.to_string(), merged_rulesets.clone());
        Ok(merged_rulesets)
    }
}

pub fn builtin_profiles() -> BTreeMap<String, Profile> {
    BTreeMap::from([
        (
            "default".to_string(),
            Profile {
                extends: Vec::new(),
                ruleset: vec!["noir_core".to_string()],
            },
        ),
        (
            "noir".to_string(),
            Profile {
                extends: vec!["default".to_string()],
                ruleset: Vec::new(),
            },
        ),
        (
            "aztec".to_string(),
            Profile {
                extends: vec!["default".to_string()],
                ruleset: vec!["aztec_pack".to_string()],
            },
        ),
    ])
}

pub fn normalize_rule_id(rule_id: &str) -> String {
    rule_id.trim().to_ascii_uppercase()
}

fn append_unique(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        if !target.contains(value) {
            target.push(value.clone());
        }
    }
}

fn apply_rule_overrides(
    levels: &mut BTreeMap<String, RuleLevel>,
    overrides: &RuleOverrides,
) -> Result<(), ConfigError> {
    let mut seen = BTreeMap::<String, RuleLevel>::new();
    register_override(&mut seen, &overrides.allow, RuleLevel::Allow, "--allow")?;
    register_override(&mut seen, &overrides.warn, RuleLevel::Warn, "--warn")?;
    register_override(&mut seen, &overrides.deny, RuleLevel::Deny, "--deny")?;

    for (rule_id, level) in seen {
        levels.insert(rule_id, level);
    }
    Ok(())
}

fn register_override(
    seen: &mut BTreeMap<String, RuleLevel>,
    rules: &[String],
    requested: RuleLevel,
    source: &str,
) -> Result<(), ConfigError> {
    for rule in rules {
        let normalized = normalize_rule_id(rule);
        if find_lint(&normalized).is_none() {
            return Err(ConfigError::UnknownRuleId {
                rule_id: normalized,
                source: source.to_string(),
            });
        }

        if let Some(existing) = seen.get(&normalized) {
            if *existing != requested {
                return Err(ConfigError::ConflictingRuleOverride {
                    rule_id: normalized,
                    existing: *existing,
                    requested,
                });
            }
            continue;
        }
        seen.insert(normalized, requested);
    }
    Ok(())
}

fn ruleset_defaults(ruleset: &str) -> Option<Vec<(&'static str, RuleLevel)>> {
    let mut defaults = all_lints()
        .iter()
        .filter(|lint| lint.pack == ruleset && lint.lifecycle.is_active())
        .map(|lint| (lint.id, lint.default_level))
        .collect::<Vec<_>>();

    if defaults.is_empty() {
        return None;
    }

    defaults.sort_unstable_by_key(|(rule_id, _)| *rule_id);
    Some(defaults)
}

fn default_contract_attribute() -> String {
    "aztec".to_string()
}

fn default_external_attribute() -> String {
    "external".to_string()
}

fn default_external_kinds() -> Vec<String> {
    vec!["public".to_string(), "private".to_string()]
}

fn default_only_self_attribute() -> String {
    "only_self".to_string()
}

fn default_initializer_attribute() -> String {
    "initializer".to_string()
}

fn default_storage_attribute() -> String {
    "storage".to_string()
}

fn default_imports_prefixes() -> Vec<String> {
    vec!["aztec".to_string(), "::aztec".to_string()]
}

fn default_note_getter_fns() -> Vec<String> {
    vec!["get_notes".to_string()]
}

fn default_nullifier_fns() -> Vec<String> {
    vec!["emit_nullifier".to_string(), "nullify".to_string()]
}

fn default_enqueue_fn() -> String {
    "enqueue".to_string()
}

fn default_contract_at_fn() -> String {
    "at".to_string()
}

fn default_nullifier_requires() -> Vec<String> {
    vec!["contract_address".to_string(), "nonce".to_string()]
}

fn default_commitment_requires() -> Vec<String> {
    vec!["contract_address".to_string(), "note_type".to_string()]
}

#[cfg(test)]
mod tests {
    use super::{Config, RawConfig, RuleLevel, RuleOverrides};
    use crate::config::ConfigError;

    const SPEC_SAMPLE_CONFIG: &str = r#"
[profile.default]
ruleset = ["noir_core"]

[profile.aztec]
extends = ["default"]
ruleset = ["noir_core", "aztec_pack"]

[aztec]
contract_attribute = "aztec"
external_attribute = "external"
external_kinds = ["public", "private"]
only_self_attribute = "only_self"
initializer_attribute = "initializer"
storage_attribute = "storage"

imports_prefixes = ["aztec", "::aztec"]

note_getter_fns = ["get_notes"]
nullifier_fns = ["emit_nullifier", "nullify"]
enqueue_fn = "enqueue"
contract_at_fn = "at"

[aztec.domain_separation]
nullifier_requires = ["contract_address", "nonce"]
commitment_requires = ["contract_address", "note_type"]
"#;

    #[test]
    fn parses_spec_sample_config() {
        let raw: RawConfig = toml::from_str(SPEC_SAMPLE_CONFIG).expect("sample config must parse");
        let config = Config::from_raw(raw);

        let default_profile = config
            .profile
            .get("default")
            .expect("default profile should exist");
        assert_eq!(default_profile.ruleset, vec!["noir_core"]);

        let resolved = config
            .resolve_profile("aztec")
            .expect("aztec profile should resolve");
        assert_eq!(resolved.rulesets, vec!["noir_core", "aztec_pack"]);

        let noir_resolved = config
            .resolve_profile("noir")
            .expect("noir profile should resolve");
        assert_eq!(noir_resolved.rulesets, vec!["noir_core"]);

        assert_eq!(config.aztec.contract_attribute, "aztec");
        assert_eq!(config.aztec.external_attribute, "external");
        assert_eq!(config.aztec.imports_prefixes, vec!["aztec", "::aztec"]);
        assert_eq!(
            config.aztec.domain_separation.nullifier_requires,
            vec!["contract_address", "nonce"]
        );
    }

    #[test]
    fn detects_profile_cycle() {
        let cycle = r#"
[profile.a]
extends = ["b"]

[profile.b]
extends = ["a"]
"#;

        let raw: RawConfig = toml::from_str(cycle).expect("cycle config must parse");
        let config = Config::from_raw(raw);
        let err = config
            .resolve_profile("a")
            .expect_err("cycle must be detected");

        match err {
            ConfigError::ProfileCycle { cycle } => {
                assert_eq!(
                    cycle,
                    vec!["a".to_string(), "b".to_string(), "a".to_string()]
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn cli_overrides_take_precedence_over_profile_defaults() {
        let config = Config::default();
        let overrides = RuleOverrides {
            deny: vec!["NOIR001".to_string()],
            warn: vec!["AZTEC001".to_string()],
            allow: vec!["noir100".to_string()],
        };

        let levels = config
            .effective_rule_levels("aztec", &overrides)
            .expect("effective levels should resolve");

        assert_eq!(levels.get("NOIR001"), Some(&RuleLevel::Deny));
        assert_eq!(levels.get("NOIR100"), Some(&RuleLevel::Allow));
        assert_eq!(levels.get("AZTEC001"), Some(&RuleLevel::Warn));
    }

    #[test]
    fn conflicting_cli_overrides_are_rejected() {
        let config = Config::default();
        let overrides = RuleOverrides {
            deny: vec!["AZTEC010".to_string()],
            warn: Vec::new(),
            allow: vec!["aztec010".to_string()],
        };

        let err = config
            .effective_rule_levels("aztec", &overrides)
            .expect_err("conflicting CLI overrides should fail");

        match err {
            ConfigError::ConflictingRuleOverride { rule_id, .. } => {
                assert_eq!(rule_id, "AZTEC010");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn unknown_cli_rule_overrides_are_rejected() {
        let config = Config::default();
        let overrides = RuleOverrides {
            deny: vec!["does_not_exist".to_string()],
            warn: Vec::new(),
            allow: Vec::new(),
        };

        let err = config
            .effective_rule_levels("aztec", &overrides)
            .expect_err("unknown CLI override should fail");

        match err {
            ConfigError::UnknownRuleId { rule_id, source } => {
                assert_eq!(rule_id, "DOES_NOT_EXIST");
                assert_eq!(source, "--deny");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
