pub const PRIVACY: &str = "privacy";
pub const PROTOCOL: &str = "protocol";
pub const SOUNDNESS: &str = "soundness";
pub const CORRECTNESS: &str = "correctness";
pub const MAINTAINABILITY: &str = "maintainability";
pub const PERFORMANCE: &str = "performance";

pub fn is_supported_policy(policy: &str) -> bool {
    matches!(
        policy,
        PRIVACY | PROTOCOL | SOUNDNESS | CORRECTNESS | MAINTAINABILITY | PERFORMANCE
    )
}

#[cfg(test)]
mod tests {
    use super::{PRIVACY, is_supported_policy};

    #[test]
    fn policy_names_are_whitelisted() {
        assert!(is_supported_policy(PRIVACY));
        assert!(is_supported_policy("protocol"));
        assert!(!is_supported_policy("unknown"));
    }
}
