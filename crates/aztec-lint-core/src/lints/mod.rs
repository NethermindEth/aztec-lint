use crate::config::RuleLevel;
use crate::diagnostics::Confidence;
use crate::policy::{CORRECTNESS, MAINTAINABILITY, PRIVACY, PROTOCOL, SOUNDNESS};

pub mod types;

pub use types::{LintCategory, LintDocs, LintLifecycleState, LintSpec};

const ALL_LINT_SPECS: &[LintSpec] = &[
    LintSpec {
        id: "AZTEC001",
        pack: "aztec_pack",
        policy: PRIVACY,
        category: LintCategory::Privacy,
        default_level: RuleLevel::Deny,
        confidence: Confidence::Medium,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Private data reaches a public sink.",
            details: "Flags flows where secret or note-derived values are emitted through public channels.",
        },
    },
    LintSpec {
        id: "AZTEC002",
        pack: "aztec_pack",
        policy: PRIVACY,
        category: LintCategory::Privacy,
        default_level: RuleLevel::Deny,
        confidence: Confidence::Low,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Secret-dependent branching affects public state.",
            details: "Detects control flow where secret inputs influence public behavior.",
        },
    },
    LintSpec {
        id: "AZTEC003",
        pack: "aztec_pack",
        policy: PRIVACY,
        category: LintCategory::Privacy,
        default_level: RuleLevel::Deny,
        confidence: Confidence::Medium,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Private entrypoint uses debug logging.",
            details: "Reports debug logging in private contexts where logging may leak sensitive state.",
        },
    },
    LintSpec {
        id: "AZTEC010",
        pack: "aztec_pack",
        policy: PROTOCOL,
        category: LintCategory::Protocol,
        default_level: RuleLevel::Deny,
        confidence: Confidence::High,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Private to public bridge requires #[only_self].",
            details: "Checks enqueue-based private-to-public transitions enforce self-only invocation constraints.",
        },
    },
    LintSpec {
        id: "AZTEC020",
        pack: "aztec_pack",
        policy: SOUNDNESS,
        category: LintCategory::Soundness,
        default_level: RuleLevel::Deny,
        confidence: Confidence::High,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Unconstrained influence reaches commitments, storage, or nullifiers.",
            details: "Detects unconstrained values that affect constrained Aztec protocol artifacts.",
        },
    },
    LintSpec {
        id: "AZTEC021",
        pack: "aztec_pack",
        policy: SOUNDNESS,
        category: LintCategory::Soundness,
        default_level: RuleLevel::Deny,
        confidence: Confidence::Medium,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Missing range constraints before hashing or serialization.",
            details: "Reports values hashed or serialized without proving required numeric bounds first.",
        },
    },
    LintSpec {
        id: "AZTEC022",
        pack: "aztec_pack",
        policy: SOUNDNESS,
        category: LintCategory::Soundness,
        default_level: RuleLevel::Deny,
        confidence: Confidence::Medium,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Suspicious Merkle witness usage.",
            details: "Finds witness handling patterns that likely violate expected Merkle proof semantics.",
        },
    },
    LintSpec {
        id: "NOIR001",
        pack: "noir_core",
        policy: CORRECTNESS,
        category: LintCategory::Correctness,
        default_level: RuleLevel::Deny,
        confidence: Confidence::High,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Unused variable or import.",
            details: "Detects declared bindings and imports that are not used.",
        },
    },
    LintSpec {
        id: "NOIR002",
        pack: "noir_core",
        policy: CORRECTNESS,
        category: LintCategory::Correctness,
        default_level: RuleLevel::Deny,
        confidence: Confidence::Medium,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Suspicious shadowing.",
            details: "Reports variable declarations that shadow earlier bindings in the same function scope.",
        },
    },
    LintSpec {
        id: "NOIR010",
        pack: "noir_core",
        policy: CORRECTNESS,
        category: LintCategory::Correctness,
        default_level: RuleLevel::Deny,
        confidence: Confidence::High,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Boolean computed but not asserted.",
            details: "Flags boolean expressions that appear intended for checks but never drive an assertion.",
        },
    },
    LintSpec {
        id: "NOIR020",
        pack: "noir_core",
        policy: CORRECTNESS,
        category: LintCategory::Correctness,
        default_level: RuleLevel::Deny,
        confidence: Confidence::Medium,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Array indexing without bounds validation.",
            details: "Detects index operations lacking an obvious preceding range constraint.",
        },
    },
    LintSpec {
        id: "NOIR030",
        pack: "noir_core",
        policy: CORRECTNESS,
        category: LintCategory::Correctness,
        default_level: RuleLevel::Deny,
        confidence: Confidence::Medium,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Unconstrained value influences constrained logic.",
            details: "Reports suspicious influence of unconstrained data over constrained computation paths.",
        },
    },
    LintSpec {
        id: "NOIR100",
        pack: "noir_core",
        policy: MAINTAINABILITY,
        category: LintCategory::Maintainability,
        default_level: RuleLevel::Warn,
        confidence: Confidence::Low,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Magic number literal should be named.",
            details: "Encourages replacing unexplained numeric constants with named constants.",
        },
    },
    LintSpec {
        id: "NOIR110",
        pack: "noir_core",
        policy: MAINTAINABILITY,
        category: LintCategory::Maintainability,
        default_level: RuleLevel::Warn,
        confidence: Confidence::Low,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Function complexity exceeds threshold.",
            details: "Flags functions whose control flow complexity passes the configured limit.",
        },
    },
    LintSpec {
        id: "NOIR120",
        pack: "noir_core",
        policy: MAINTAINABILITY,
        category: LintCategory::Maintainability,
        default_level: RuleLevel::Warn,
        confidence: Confidence::Low,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Function nesting depth exceeds threshold.",
            details: "Flags deeply nested control flow that reduces readability and maintainability.",
        },
    },
];

pub fn all_lints() -> &'static [LintSpec] {
    ALL_LINT_SPECS
}

pub fn find_lint(rule_id: &str) -> Option<&'static LintSpec> {
    let canonical = normalize_lint_id(rule_id);
    ALL_LINT_SPECS.iter().find(|lint| lint.id == canonical)
}

pub fn normalize_lint_id(rule_id: &str) -> String {
    rule_id.trim().to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{all_lints, find_lint};
    use crate::lints::LintLifecycleState;

    #[test]
    fn lint_catalog_rule_ids_are_unique() {
        let mut seen = BTreeSet::new();
        for lint in all_lints() {
            assert!(seen.insert(lint.id), "duplicate lint id '{}'", lint.id);
        }
    }

    #[test]
    fn lint_catalog_rule_ids_are_canonical_uppercase() {
        for lint in all_lints() {
            assert_eq!(
                lint.id,
                lint.id.trim(),
                "lint id '{}' has surrounding whitespace",
                lint.id
            );
            assert!(
                lint.id
                    .chars()
                    .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_'),
                "lint id '{}' contains unsupported characters",
                lint.id
            );
        }
    }

    #[test]
    fn active_lints_have_required_docs_fields() {
        for lint in all_lints() {
            if matches!(lint.lifecycle, LintLifecycleState::Active) {
                assert!(
                    lint.docs.has_required_fields(),
                    "active lint '{}' is missing required docs fields",
                    lint.id
                );
            }
        }
    }

    #[test]
    fn find_lint_accepts_non_canonical_input() {
        let by_canonical = find_lint("NOIR100").expect("NOIR100 should exist");
        let by_non_canonical =
            find_lint("  noir100 ").expect("case-insensitive lookup should work");
        assert_eq!(by_canonical.id, by_non_canonical.id);
    }
}
