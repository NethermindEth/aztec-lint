use aztec_lint_core::lints::{LintSpec, find_lint};

use crate::Rule;
use crate::aztec::{
    aztec001_privacy_leak::Aztec001PrivacyLeakRule,
    aztec002_secret_branching::Aztec002SecretBranchingRule,
    aztec003_private_debug_log::Aztec003PrivateDebugLogRule,
    aztec010_only_self_enqueue::Aztec010OnlySelfEnqueueRule,
    aztec020_unconstrained_influence::Aztec020UnconstrainedInfluenceRule,
    aztec021_range_before_hash::Aztec021RangeBeforeHashRule,
    aztec022_merkle_witness::Aztec022MerkleWitnessRule,
};
use crate::noir_core::{
    noir001_unused::Noir001UnusedRule, noir002_shadowing::Noir002ShadowingRule,
    noir010_bool_not_asserted::Noir010BoolNotAssertedRule, noir020_bounds::Noir020BoundsRule,
    noir030_unconstrained_influence::Noir030UnconstrainedInfluenceRule,
    noir100_magic_numbers::Noir100MagicNumbersRule,
    noir101_repeated_local_inits::Noir101RepeatedLocalInitMagicNumbersRule,
    noir110_complexity::Noir110ComplexityRule, noir120_nesting::Noir120NestingRule,
};

pub struct RuleRegistration {
    pub lint: &'static LintSpec,
    pub rule: Box<dyn Rule>,
}

pub fn full_registry() -> Vec<RuleRegistration> {
    vec![
        register(Box::new(Noir001UnusedRule)),
        register(Box::new(Noir002ShadowingRule)),
        register(Box::new(Noir010BoolNotAssertedRule)),
        register(Box::new(Noir020BoundsRule)),
        register(Box::new(Noir030UnconstrainedInfluenceRule)),
        register(Box::new(Noir100MagicNumbersRule)),
        register(Box::new(Noir101RepeatedLocalInitMagicNumbersRule)),
        register(Box::new(Noir110ComplexityRule)),
        register(Box::new(Noir120NestingRule)),
        register(Box::new(Aztec001PrivacyLeakRule)),
        register(Box::new(Aztec002SecretBranchingRule)),
        register(Box::new(Aztec003PrivateDebugLogRule)),
        register(Box::new(Aztec010OnlySelfEnqueueRule)),
        register(Box::new(Aztec020UnconstrainedInfluenceRule)),
        register(Box::new(Aztec021RangeBeforeHashRule)),
        register(Box::new(Aztec022MerkleWitnessRule)),
    ]
}

fn register(rule: Box<dyn Rule>) -> RuleRegistration {
    let rule_id = rule.id();
    let lint = find_lint(rule_id).unwrap_or_else(|| {
        panic!(
            "runtime rule '{}' is missing from canonical lint catalog",
            rule_id
        )
    });
    assert!(
        lint.lifecycle.is_active(),
        "runtime rule '{}' maps to non-active canonical lint metadata",
        rule_id
    );

    RuleRegistration { lint, rule }
}
