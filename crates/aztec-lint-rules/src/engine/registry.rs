use aztec_lint_core::config::RuleLevel;
use aztec_lint_core::diagnostics::Confidence;
use aztec_lint_core::policy::{CORRECTNESS, MAINTAINABILITY, PRIVACY, PROTOCOL, SOUNDNESS};

use crate::Rule;
use crate::aztec::{
    aztec001_privacy_leak::Aztec001PrivacyLeakRule,
    aztec010_only_self_enqueue::Aztec010OnlySelfEnqueueRule,
    aztec020_unconstrained_influence::Aztec020UnconstrainedInfluenceRule,
};
use crate::noir_core::{
    noir001_unused::Noir001UnusedRule, noir002_shadowing::Noir002ShadowingRule,
    noir010_bool_not_asserted::Noir010BoolNotAssertedRule, noir020_bounds::Noir020BoundsRule,
    noir030_unconstrained_influence::Noir030UnconstrainedInfluenceRule,
    noir100_magic_numbers::Noir100MagicNumbersRule, noir110_complexity::Noir110ComplexityRule,
    noir120_nesting::Noir120NestingRule,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleMetadata {
    pub id: &'static str,
    pub pack: &'static str,
    pub policy: &'static str,
    pub default_level: RuleLevel,
    pub confidence: Confidence,
}

pub struct RuleRegistration {
    pub metadata: RuleMetadata,
    pub rule: Box<dyn Rule>,
}

pub fn full_registry() -> Vec<RuleRegistration> {
    let mut registry = vec![
        RuleRegistration {
            metadata: RuleMetadata {
                id: "NOIR001",
                pack: "noir_core",
                policy: CORRECTNESS,
                default_level: RuleLevel::Deny,
                confidence: Confidence::High,
            },
            rule: Box::new(Noir001UnusedRule),
        },
        RuleRegistration {
            metadata: RuleMetadata {
                id: "NOIR002",
                pack: "noir_core",
                policy: CORRECTNESS,
                default_level: RuleLevel::Deny,
                confidence: Confidence::Medium,
            },
            rule: Box::new(Noir002ShadowingRule),
        },
        RuleRegistration {
            metadata: RuleMetadata {
                id: "NOIR010",
                pack: "noir_core",
                policy: CORRECTNESS,
                default_level: RuleLevel::Deny,
                confidence: Confidence::High,
            },
            rule: Box::new(Noir010BoolNotAssertedRule),
        },
        RuleRegistration {
            metadata: RuleMetadata {
                id: "NOIR020",
                pack: "noir_core",
                policy: CORRECTNESS,
                default_level: RuleLevel::Deny,
                confidence: Confidence::Medium,
            },
            rule: Box::new(Noir020BoundsRule),
        },
        RuleRegistration {
            metadata: RuleMetadata {
                id: "NOIR030",
                pack: "noir_core",
                policy: CORRECTNESS,
                default_level: RuleLevel::Deny,
                confidence: Confidence::Medium,
            },
            rule: Box::new(Noir030UnconstrainedInfluenceRule),
        },
        RuleRegistration {
            metadata: RuleMetadata {
                id: "NOIR100",
                pack: "noir_core",
                policy: MAINTAINABILITY,
                default_level: RuleLevel::Warn,
                confidence: Confidence::Low,
            },
            rule: Box::new(Noir100MagicNumbersRule),
        },
        RuleRegistration {
            metadata: RuleMetadata {
                id: "NOIR110",
                pack: "noir_core",
                policy: MAINTAINABILITY,
                default_level: RuleLevel::Warn,
                confidence: Confidence::Low,
            },
            rule: Box::new(Noir110ComplexityRule),
        },
        RuleRegistration {
            metadata: RuleMetadata {
                id: "NOIR120",
                pack: "noir_core",
                policy: MAINTAINABILITY,
                default_level: RuleLevel::Warn,
                confidence: Confidence::Low,
            },
            rule: Box::new(Noir120NestingRule),
        },
    ];

    registry.extend([
        RuleRegistration {
            metadata: RuleMetadata {
                id: "AZTEC001",
                pack: "aztec_pack",
                policy: PRIVACY,
                default_level: RuleLevel::Deny,
                confidence: Confidence::Medium,
            },
            rule: Box::new(Aztec001PrivacyLeakRule),
        },
        RuleRegistration {
            metadata: RuleMetadata {
                id: "AZTEC010",
                pack: "aztec_pack",
                policy: PROTOCOL,
                default_level: RuleLevel::Deny,
                confidence: Confidence::High,
            },
            rule: Box::new(Aztec010OnlySelfEnqueueRule),
        },
        RuleRegistration {
            metadata: RuleMetadata {
                id: "AZTEC020",
                pack: "aztec_pack",
                policy: SOUNDNESS,
                default_level: RuleLevel::Deny,
                confidence: Confidence::High,
            },
            rule: Box::new(Aztec020UnconstrainedInfluenceRule),
        },
    ]);

    registry
}
