use aztec_lint_core::config::AztecConfig;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceUnit {
    pub path: String,
    pub text: String,
}

impl SourceUnit {
    pub fn new(path: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            text: text.into(),
        }
    }
}

pub fn should_activate_aztec(profile: &str, sources: &[SourceUnit], config: &AztecConfig) -> bool {
    if profile.eq_ignore_ascii_case("aztec") {
        return true;
    }

    let contract_attr = format!("#[{}]", config.contract_attribute);
    let import_prefixes = config
        .imports_prefixes
        .iter()
        .map(|prefix| format!("{}::", prefix.trim().trim_end_matches("::")))
        .collect::<Vec<_>>();

    sources.iter().any(|source| {
        source.text.contains(&contract_attr)
            || source.text.lines().any(|line| {
                let trimmed = line.trim_start();
                let import_clause = trimmed
                    .strip_prefix("use ")
                    .or_else(|| trimmed.strip_prefix("pub use "))
                    .map(str::trim_start);
                let Some(import_clause) = import_clause else {
                    return false;
                };

                import_prefixes.iter().any(|prefix| {
                    import_clause.starts_with(prefix)
                        || import_clause.starts_with(prefix.trim_start_matches("::"))
                })
            })
    })
}

#[cfg(test)]
mod tests {
    use aztec_lint_core::config::AztecConfig;

    use super::{SourceUnit, should_activate_aztec};

    #[test]
    fn activates_for_aztec_profile() {
        let config = AztecConfig::default();
        let sources = vec![SourceUnit::new("src/main.nr", "fn main() {}")];
        assert!(should_activate_aztec("aztec", &sources, &config));
    }

    #[test]
    fn activates_for_contract_attribute() {
        let config = AztecConfig::default();
        let sources = vec![SourceUnit::new(
            "src/main.nr",
            "#[aztec]\npub contract C {}",
        )];
        assert!(should_activate_aztec("default", &sources, &config));
    }

    #[test]
    fn activates_for_aztec_import() {
        let config = AztecConfig::default();
        let sources = vec![SourceUnit::new("src/main.nr", "use aztec::prelude::*;")];
        assert!(should_activate_aztec("default", &sources, &config));
    }

    #[test]
    fn activates_for_absolute_aztec_import() {
        let config = AztecConfig::default();
        let sources = vec![SourceUnit::new("src/main.nr", "use ::aztec::prelude::*;")];
        assert!(should_activate_aztec("default", &sources, &config));
    }

    #[test]
    fn does_not_activate_for_non_root_aztec_segment() {
        let config = AztecConfig::default();
        let sources = vec![SourceUnit::new(
            "src/main.nr",
            "use other::aztec::helpers::x;",
        )];
        assert!(!should_activate_aztec("default", &sources, &config));
    }

    #[test]
    fn stays_inactive_without_trigger() {
        let config = AztecConfig::default();
        let sources = vec![SourceUnit::new("src/main.nr", "fn main() { let x = 1; }")];
        assert!(!should_activate_aztec("default", &sources, &config));
    }
}
