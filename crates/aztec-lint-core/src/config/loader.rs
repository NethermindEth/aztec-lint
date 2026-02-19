use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{Config, ConfigError, RawConfig};

pub const CONFIG_FILE_PRIMARY: &str = "aztec-lint.toml";
pub const CONFIG_FILE_FALLBACK: &str = "noir-lint.toml";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigSource {
    File(PathBuf),
    Default,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedConfig {
    pub config: Config,
    pub source: ConfigSource,
}

pub fn load_from_dir(dir: &Path) -> Result<LoadedConfig, ConfigError> {
    let primary = dir.join(CONFIG_FILE_PRIMARY);
    if primary.is_file() {
        let config = load_from_path(&primary)?;
        return Ok(LoadedConfig {
            config,
            source: ConfigSource::File(primary),
        });
    }

    let fallback = dir.join(CONFIG_FILE_FALLBACK);
    if fallback.is_file() {
        let config = load_from_path(&fallback)?;
        return Ok(LoadedConfig {
            config,
            source: ConfigSource::File(fallback),
        });
    }

    Ok(LoadedConfig {
        config: Config::default(),
        source: ConfigSource::Default,
    })
}

pub fn load_from_path(path: &Path) -> Result<Config, ConfigError> {
    let raw = fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let parsed = toml::from_str::<RawConfig>(&raw).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(Config::from_raw(parsed))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{CONFIG_FILE_FALLBACK, CONFIG_FILE_PRIMARY, ConfigSource, load_from_dir};

    #[test]
    fn prefers_aztec_lint_file_when_both_exist() {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let root = temp_dir.path();
        fs::write(
            root.join(CONFIG_FILE_PRIMARY),
            "[profile.default]\nruleset=[\"aztec_pack\"]\n",
        )
        .expect("primary config should be written");
        fs::write(
            root.join(CONFIG_FILE_FALLBACK),
            "[profile.default]\nruleset=[\"noir_core\"]\n",
        )
        .expect("fallback config should be written");

        let loaded = load_from_dir(root).expect("config should load");

        match loaded.source {
            ConfigSource::File(path) => {
                assert_eq!(
                    path.file_name().and_then(|v| v.to_str()),
                    Some(CONFIG_FILE_PRIMARY)
                )
            }
            ConfigSource::Default => panic!("expected file-based config source"),
        }
    }

    #[test]
    fn falls_back_to_legacy_file_name() {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let root = temp_dir.path();
        fs::write(
            root.join(CONFIG_FILE_FALLBACK),
            "[profile.default]\nruleset=[\"noir_core\"]\n",
        )
        .expect("fallback config should be written");

        let loaded = load_from_dir(root).expect("config should load");

        match loaded.source {
            ConfigSource::File(path) => {
                assert_eq!(
                    path.file_name().and_then(|v| v.to_str()),
                    Some(CONFIG_FILE_FALLBACK)
                );
            }
            ConfigSource::Default => panic!("expected file-based config source"),
        }
    }

    #[test]
    fn returns_default_config_when_no_files_exist() {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let loaded = load_from_dir(temp_dir.path()).expect("config should load");
        assert_eq!(loaded.source, ConfigSource::Default);
        assert!(loaded.config.profile.contains_key("default"));
    }
}
