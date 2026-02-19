use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

pub mod loader;
pub mod types;

pub use loader::{
    CONFIG_FILE_FALLBACK, CONFIG_FILE_PRIMARY, ConfigSource, LoadedConfig, load_from_dir,
};
pub use types::{
    AztecConfig, Config, DomainSeparationConfig, Profile, RawConfig, ResolvedProfile, RuleLevel,
    RuleOverrides,
};

#[derive(Debug)]
pub enum ConfigError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    ProfileNotFound {
        profile: String,
    },
    ParentProfileNotFound {
        profile: String,
        parent: String,
    },
    ProfileCycle {
        cycle: Vec<String>,
    },
    UnknownRuleset {
        ruleset: String,
    },
    ConflictingRuleOverride {
        rule_id: String,
        existing: RuleLevel,
        requested: RuleLevel,
    },
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    f,
                    "failed to read config file '{}': {source}",
                    path.display()
                )
            }
            Self::Parse { path, source } => {
                write!(
                    f,
                    "failed to parse config file '{}': {source}",
                    path.display()
                )
            }
            Self::ProfileNotFound { profile } => {
                write!(f, "profile '{profile}' was not found in configuration")
            }
            Self::ParentProfileNotFound { profile, parent } => write!(
                f,
                "profile '{profile}' extends unknown parent profile '{parent}'"
            ),
            Self::ProfileCycle { cycle } => {
                write!(
                    f,
                    "profile inheritance cycle detected: {}",
                    cycle.join(" -> ")
                )
            }
            Self::UnknownRuleset { ruleset } => {
                write!(f, "unknown ruleset '{ruleset}' in profile configuration")
            }
            Self::ConflictingRuleOverride {
                rule_id,
                existing,
                requested,
            } => write!(
                f,
                "conflicting CLI override for rule '{rule_id}': {existing} vs {requested}"
            ),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::ProfileNotFound { .. }
            | Self::ParentProfileNotFound { .. }
            | Self::ProfileCycle { .. }
            | Self::UnknownRuleset { .. }
            | Self::ConflictingRuleOverride { .. } => None,
        }
    }
}
