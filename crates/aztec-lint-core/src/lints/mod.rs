use std::fmt::Write;

use crate::config::RuleLevel;
use crate::diagnostics::Confidence;
use crate::policy::{CORRECTNESS, MAINTAINABILITY, PRIVACY, PROTOCOL, SOUNDNESS};

pub mod types;

pub use types::{LintCategory, LintDocs, LintLifecycleState, LintSpec};

const INTRODUCED_IN_V0_1_0: &str = "0.1.0";

const DOCS_REFERENCE_RULE_AUTHORING: &str = "docs/rule-authoring.md";
const DOCS_REFERENCE_SUPPRESSION: &str = "docs/suppression.md";
const DOCS_REFERENCE_DECISION_0001: &str = "docs/decisions/0001-aztec010-scope.md";
const DOCS_REFERENCE_DECISION_0003: &str = "docs/decisions/0003-confidence-model.md";

const ALL_LINT_SPECS: &[LintSpec] = &[
    LintSpec {
        id: "AZTEC001",
        pack: "aztec_pack",
        policy: PRIVACY,
        category: LintCategory::Privacy,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Deny,
        confidence: Confidence::Medium,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Private data reaches a public sink.",
            what_it_does: "Flags flows where secret or note-derived values are emitted through public channels.",
            why_this_matters: "Leaking private values through public outputs can permanently expose sensitive state.",
            known_limitations: "Flow analysis is conservative and may miss leaks routed through unsupported abstractions.",
            how_to_fix: "Keep private values in constrained private paths and sanitize or avoid public emission points.",
            examples: &["Avoid emitting note-derived values from public entrypoints."],
            references: &[DOCS_REFERENCE_SUPPRESSION, DOCS_REFERENCE_RULE_AUTHORING],
        },
    },
    LintSpec {
        id: "AZTEC002",
        pack: "aztec_pack",
        policy: PRIVACY,
        category: LintCategory::Privacy,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Deny,
        confidence: Confidence::Low,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Secret-dependent branching affects public state.",
            what_it_does: "Detects control flow where secret inputs influence public behavior.",
            why_this_matters: "Secret-dependent branching can reveal private information through observable behavior.",
            known_limitations: "Heuristic path tracking may report false positives in complex guard patterns.",
            how_to_fix: "Refactor logic so branch predicates for public effects are independent of private data.",
            examples: &["Compute public decisions from public inputs only."],
            references: &[DOCS_REFERENCE_RULE_AUTHORING, DOCS_REFERENCE_DECISION_0003],
        },
    },
    LintSpec {
        id: "AZTEC003",
        pack: "aztec_pack",
        policy: PRIVACY,
        category: LintCategory::Privacy,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Deny,
        confidence: Confidence::Medium,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Private entrypoint uses debug logging.",
            what_it_does: "Reports debug logging in private contexts where logging may leak sensitive state.",
            why_this_matters: "Debug output can disclose values intended to remain private.",
            known_limitations: "Custom logging wrappers are only detected when call patterns are recognizable.",
            how_to_fix: "Remove debug logging from private code paths or replace it with safe telemetry patterns.",
            examples: &["Do not print private witnesses in private functions."],
            references: &[DOCS_REFERENCE_SUPPRESSION, DOCS_REFERENCE_RULE_AUTHORING],
        },
    },
    LintSpec {
        id: "AZTEC010",
        pack: "aztec_pack",
        policy: PROTOCOL,
        category: LintCategory::Protocol,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Deny,
        confidence: Confidence::High,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Private to public bridge requires #[only_self].",
            what_it_does: "Checks enqueue-based private-to-public transitions enforce self-only invocation constraints.",
            why_this_matters: "Missing self-only restrictions can allow unauthorized cross-context execution.",
            known_limitations: "Rule coverage is scoped to known enqueue bridge patterns.",
            how_to_fix: "Apply the configured only-self attribute and ensure bridge entrypoints enforce it.",
            examples: &["Annotate private-to-public bridge functions with #[only_self]."],
            references: &[DOCS_REFERENCE_DECISION_0001, DOCS_REFERENCE_RULE_AUTHORING],
        },
    },
    LintSpec {
        id: "AZTEC020",
        pack: "aztec_pack",
        policy: SOUNDNESS,
        category: LintCategory::Soundness,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Deny,
        confidence: Confidence::High,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Unconstrained influence reaches commitments, storage, or nullifiers.",
            what_it_does: "Detects unconstrained values that affect constrained Aztec protocol artifacts.",
            why_this_matters: "Unconstrained influence can break proof soundness and on-chain validity assumptions.",
            known_limitations: "Transitive influence through unsupported helper layers may be missed.",
            how_to_fix: "Introduce explicit constraints before values affect commitments, storage, or nullifiers.",
            examples: &["Constrain intermediate values before writing storage commitments."],
            references: &[DOCS_REFERENCE_RULE_AUTHORING, DOCS_REFERENCE_DECISION_0003],
        },
    },
    LintSpec {
        id: "AZTEC021",
        pack: "aztec_pack",
        policy: SOUNDNESS,
        category: LintCategory::Soundness,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Deny,
        confidence: Confidence::Medium,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Missing range constraints before hashing or serialization.",
            what_it_does: "Reports values hashed or serialized without proving required numeric bounds first.",
            why_this_matters: "Unchecked ranges can make hash and encoding logic semantically ambiguous.",
            known_limitations: "The rule cannot infer all user-defined range proof helper conventions.",
            how_to_fix: "Apply explicit range constraints before hashing, packing, or serialization boundaries.",
            examples: &["Add a range check before converting a field to a bounded integer."],
            references: &[DOCS_REFERENCE_RULE_AUTHORING, DOCS_REFERENCE_DECISION_0003],
        },
    },
    LintSpec {
        id: "AZTEC022",
        pack: "aztec_pack",
        policy: SOUNDNESS,
        category: LintCategory::Soundness,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Deny,
        confidence: Confidence::Medium,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Suspicious Merkle witness usage.",
            what_it_does: "Finds witness handling patterns that likely violate expected Merkle proof semantics.",
            why_this_matters: "Incorrect witness usage can invalidate inclusion guarantees.",
            known_limitations: "Complex custom witness manipulation may produce conservative warnings.",
            how_to_fix: "Verify witness ordering and path semantics against the target Merkle API contract.",
            examples: &[
                "Ensure witness paths and leaf values are paired using the expected order.",
            ],
            references: &[DOCS_REFERENCE_RULE_AUTHORING, DOCS_REFERENCE_DECISION_0003],
        },
    },
    LintSpec {
        id: "NOIR001",
        pack: "noir_core",
        policy: CORRECTNESS,
        category: LintCategory::Correctness,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Deny,
        confidence: Confidence::High,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Unused variable or import.",
            what_it_does: "Detects declared bindings and imports that are not used.",
            why_this_matters: "Unused items can indicate dead code, mistakes, or incomplete refactors.",
            known_limitations: "Generated code and macro-like patterns may trigger noisy diagnostics.",
            how_to_fix: "Remove unused bindings or prefix intentionally unused values with an underscore.",
            examples: &["Delete unused imports after refactoring call sites."],
            references: &[DOCS_REFERENCE_RULE_AUTHORING],
        },
    },
    LintSpec {
        id: "NOIR002",
        pack: "noir_core",
        policy: CORRECTNESS,
        category: LintCategory::Correctness,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Deny,
        confidence: Confidence::Medium,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Suspicious shadowing.",
            what_it_does: "Reports variable declarations that shadow earlier bindings in the same function scope.",
            why_this_matters: "Shadowing can hide logic bugs by silently changing which binding is referenced.",
            known_limitations: "Intentional narrow-scope shadowing may be flagged when context is ambiguous.",
            how_to_fix: "Rename inner bindings to make value flow explicit.",
            examples: &["Use descriptive names instead of reusing accumulator variables."],
            references: &[DOCS_REFERENCE_RULE_AUTHORING],
        },
    },
    LintSpec {
        id: "NOIR010",
        pack: "noir_core",
        policy: CORRECTNESS,
        category: LintCategory::Correctness,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Deny,
        confidence: Confidence::High,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Boolean computed but not asserted.",
            what_it_does: "Flags boolean expressions that appear intended for checks but never drive an assertion.",
            why_this_matters: "Forgotten assertions can leave critical invariants unenforced.",
            known_limitations: "Rules cannot always infer whether an unasserted boolean is intentionally stored for later use.",
            how_to_fix: "Use assert-style checks where the boolean is intended as a safety or validity guard.",
            examples: &["Convert an unconsumed `is_valid` expression into an assertion."],
            references: &[DOCS_REFERENCE_RULE_AUTHORING],
        },
    },
    LintSpec {
        id: "NOIR020",
        pack: "noir_core",
        policy: CORRECTNESS,
        category: LintCategory::Correctness,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Deny,
        confidence: Confidence::High,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Array indexing without bounds validation.",
            what_it_does: "Detects index operations lacking an obvious preceding range constraint.",
            why_this_matters: "Unchecked indexing can cause invalid behavior and proof failures.",
            known_limitations: "Complex index sanitization paths may not always be recognized.",
            how_to_fix: "Establish and assert index bounds before indexing operations.",
            examples: &["Assert `idx < arr.len()` before reading `arr[idx]`."],
            references: &[DOCS_REFERENCE_RULE_AUTHORING],
        },
    },
    LintSpec {
        id: "NOIR030",
        pack: "noir_core",
        policy: CORRECTNESS,
        category: LintCategory::Correctness,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Deny,
        confidence: Confidence::Medium,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Unconstrained value influences constrained logic.",
            what_it_does: "Reports suspicious influence of unconstrained data over constrained computation paths.",
            why_this_matters: "Mixing unconstrained and constrained logic can invalidate proof assumptions.",
            known_limitations: "Inference can be conservative for deeply indirect data flow.",
            how_to_fix: "Constrain values before they participate in constrained branches or outputs.",
            examples: &["Introduce explicit constraints at trust boundaries."],
            references: &[DOCS_REFERENCE_RULE_AUTHORING, DOCS_REFERENCE_DECISION_0003],
        },
    },
    LintSpec {
        id: "NOIR100",
        pack: "noir_core",
        policy: MAINTAINABILITY,
        category: LintCategory::Maintainability,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Warn,
        confidence: Confidence::High,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Magic number literal should be named.",
            what_it_does: "Detects high-signal numeric literals used in branch/assert/hash/serialization and related protocol-sensitive contexts.",
            why_this_matters: "Named constants improve readability and reduce accidental misuse.",
            known_limitations: "Low-signal plain local initializer literals are intentionally excluded from this rule.",
            how_to_fix: "Define a constant with domain meaning and use it in place of the literal.",
            examples: &["Replace `42` with `MAX_NOTES_PER_BATCH`."],
            references: &[DOCS_REFERENCE_RULE_AUTHORING],
        },
    },
    LintSpec {
        id: "NOIR101",
        pack: "noir_core",
        policy: MAINTAINABILITY,
        category: LintCategory::Maintainability,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Warn,
        confidence: Confidence::Low,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Repeated local initializer magic number should be named.",
            what_it_does: "Reports repeated literal values used in plain local initializer assignments within the same function/module scope.",
            why_this_matters: "Repeated unexplained initializer literals are often copy-pasted constants that should be named for clarity.",
            known_limitations: "Single local initializer literals are intentionally skipped to reduce noise.",
            how_to_fix: "Extract the repeated literal into a named constant and reuse it.",
            examples: &["Replace repeated `let fee = 42; let limit = 42;` with a shared constant."],
            references: &[DOCS_REFERENCE_RULE_AUTHORING, DOCS_REFERENCE_DECISION_0003],
        },
    },
    LintSpec {
        id: "NOIR110",
        pack: "noir_core",
        policy: MAINTAINABILITY,
        category: LintCategory::Maintainability,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Warn,
        confidence: Confidence::Low,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Function complexity exceeds threshold.",
            what_it_does: "Flags functions whose control flow complexity passes the configured limit.",
            why_this_matters: "High complexity makes correctness and audits harder.",
            known_limitations: "Simple metric thresholds cannot capture all maintainability nuances.",
            how_to_fix: "Split large functions and isolate complex branches into focused helpers.",
            examples: &["Extract nested decision trees into named helper functions."],
            references: &[DOCS_REFERENCE_RULE_AUTHORING],
        },
    },
    LintSpec {
        id: "NOIR120",
        pack: "noir_core",
        policy: MAINTAINABILITY,
        category: LintCategory::Maintainability,
        introduced_in: INTRODUCED_IN_V0_1_0,
        default_level: RuleLevel::Warn,
        confidence: Confidence::Low,
        lifecycle: LintLifecycleState::Active,
        docs: LintDocs {
            summary: "Function nesting depth exceeds threshold.",
            what_it_does: "Flags deeply nested control flow that reduces readability and maintainability.",
            why_this_matters: "Deep nesting increases cognitive load and maintenance risk.",
            known_limitations: "Certain generated or domain-specific patterns can be naturally nested.",
            how_to_fix: "Use early returns and helper functions to flatten nested control flow.",
            examples: &["Refactor nested conditionals into guard clauses."],
            references: &[DOCS_REFERENCE_RULE_AUTHORING],
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

pub fn render_lints_reference_markdown() -> String {
    let mut output = String::new();
    let _ = writeln!(output, "# Lint Reference");
    let _ = writeln!(output);
    let _ = writeln!(
        output,
        "This document lists active enforced lints in `aztec-lint` and explains what each lint checks, why it matters, known limitations, and typical remediation."
    );
    let _ = writeln!(output);
    let _ = writeln!(
        output,
        "Source of truth for this data is the canonical lint metadata catalog in `crates/aztec-lint-core/src/lints/mod.rs`."
    );
    let _ = writeln!(output);

    for pack in ["aztec_pack", "noir_core"] {
        let heading = match pack {
            "aztec_pack" => "## AZTEC Pack",
            "noir_core" => "## Noir Core Pack",
            _ => continue,
        };
        let _ = writeln!(output, "{heading}");
        let _ = writeln!(output);

        for lint in all_lints()
            .iter()
            .filter(|lint| lint.lifecycle.is_active() && lint.pack == pack)
        {
            let _ = writeln!(output, "### {}", lint.id);
            let _ = writeln!(output);
            let _ = writeln!(output, "- Pack: `{}`", lint.pack);
            let _ = writeln!(output, "- Category: `{}`", lint.category.as_str());
            let _ = writeln!(output, "- Policy: `{}`", lint.policy);
            let _ = writeln!(output, "- Default Level: `{}`", lint.default_level);
            let _ = writeln!(
                output,
                "- Confidence: `{}`",
                confidence_label(lint.confidence)
            );
            let _ = writeln!(output, "- Introduced In: `{}`", lint.introduced_in);
            let _ = writeln!(output, "- Lifecycle: `active`");
            let _ = writeln!(output, "- Summary: {}", lint.docs.summary);
            let _ = writeln!(output);
            let _ = writeln!(output, "What it does:");
            let _ = writeln!(output, "{}", lint.docs.what_it_does);
            let _ = writeln!(output);
            let _ = writeln!(output, "Why this matters:");
            let _ = writeln!(output, "{}", lint.docs.why_this_matters);
            let _ = writeln!(output);
            let _ = writeln!(output, "Known limitations:");
            let _ = writeln!(output, "{}", lint.docs.known_limitations);
            let _ = writeln!(output);
            let _ = writeln!(output, "How to fix:");
            let _ = writeln!(output, "{}", lint.docs.how_to_fix);
            let _ = writeln!(output);
            let _ = writeln!(output, "Examples:");
            for example in lint.docs.examples {
                let _ = writeln!(output, "- {example}");
            }
            let _ = writeln!(output);
            let _ = writeln!(output, "References:");
            for reference in lint.docs.references {
                let _ = writeln!(output, "- `{reference}`");
            }
            let _ = writeln!(output);
        }
    }

    output
}

const fn confidence_label(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::Low => "low",
        Confidence::Medium => "medium",
        Confidence::High => "high",
    }
}

#[cfg(test)]
fn validate_catalog_integrity(catalog: &[LintSpec]) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::<&str>::new();

    for lint in catalog {
        if !seen.insert(lint.id) {
            return Err(format!("duplicate lint id '{}'", lint.id));
        }
        if lint.id != lint.id.trim() {
            return Err(format!("lint id '{}' has surrounding whitespace", lint.id));
        }
        if !lint
            .id
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
        {
            return Err(format!(
                "lint id '{}' contains unsupported characters",
                lint.id
            ));
        }
        if !is_semver_like(lint.introduced_in) {
            return Err(format!(
                "lint '{}' has invalid introduced_in '{}'",
                lint.id, lint.introduced_in
            ));
        }
        if !lint.lifecycle.has_required_metadata() {
            return Err(format!(
                "lint '{}' has lifecycle metadata missing required fields",
                lint.id
            ));
        }
        if lint.lifecycle.is_active() && !lint.docs.has_required_fields() {
            return Err(format!(
                "active lint '{}' is missing required docs fields",
                lint.id
            ));
        }
    }

    for lint in catalog {
        if let LintLifecycleState::Renamed { since, to } = lint.lifecycle {
            if !is_semver_like(since) {
                return Err(format!(
                    "renamed lint '{}' has invalid lifecycle since '{}'",
                    lint.id, since
                ));
            }
            let Some(target) = catalog.iter().find(|candidate| candidate.id == to) else {
                return Err(format!(
                    "renamed lint '{}' points to missing target '{}'",
                    lint.id, to
                ));
            };
            if !target.lifecycle.is_active() {
                return Err(format!(
                    "renamed lint '{}' points to non-active target '{}'",
                    lint.id, to
                ));
            }
        }
        if let LintLifecycleState::Deprecated {
            since,
            replacement: Some(to),
            ..
        } = lint.lifecycle
        {
            if !is_semver_like(since) {
                return Err(format!(
                    "deprecated lint '{}' has invalid lifecycle since '{}'",
                    lint.id, since
                ));
            }
            let Some(target) = catalog.iter().find(|candidate| candidate.id == to) else {
                return Err(format!(
                    "deprecated lint '{}' points to missing replacement '{}'",
                    lint.id, to
                ));
            };
            if !target.lifecycle.is_active() {
                return Err(format!(
                    "deprecated lint '{}' points to non-active replacement '{}'",
                    lint.id, to
                ));
            }
        }
        if let LintLifecycleState::Deprecated { since, .. }
        | LintLifecycleState::Removed { since, .. } = lint.lifecycle
            && !is_semver_like(since)
        {
            return Err(format!(
                "lint '{}' has invalid lifecycle since '{}'",
                lint.id, since
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
fn is_semver_like(version: &str) -> bool {
    let mut parts = version.split('.');
    let Some(major) = parts.next() else {
        return false;
    };
    let Some(minor) = parts.next() else {
        return false;
    };
    let Some(patch) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    [major, minor, patch]
        .iter()
        .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::{
        LintCategory, LintDocs, LintLifecycleState, LintSpec, all_lints, find_lint,
        render_lints_reference_markdown, validate_catalog_integrity,
    };
    use crate::config::RuleLevel;
    use crate::diagnostics::Confidence;
    use crate::policy::CORRECTNESS;

    #[test]
    fn lint_catalog_invariants_hold() {
        validate_catalog_integrity(all_lints()).expect("canonical lint catalog should be valid");
    }

    #[test]
    fn lints_reference_doc_matches_catalog() {
        let expected = render_lints_reference_markdown();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/lints-reference.md");
        let actual = fs::read_to_string(&path).expect("lints reference doc should be readable");
        assert_eq!(
            actual, expected,
            "docs/lints-reference.md is out of date; regenerate from canonical lint metadata"
        );
    }

    #[test]
    fn find_lint_accepts_non_canonical_input() {
        let by_canonical = find_lint("NOIR100").expect("NOIR100 should exist");
        let by_non_canonical =
            find_lint("  noir100 ").expect("case-insensitive lookup should work");
        assert_eq!(by_canonical.id, by_non_canonical.id);
    }

    #[test]
    fn renamed_lints_must_target_existing_active_lints() {
        let mut catalog = vec![sample_active_lint("NOIR001"), sample_active_lint("NOIR100")];
        catalog.push(LintSpec {
            id: "NOIR001_OLD",
            pack: "noir_core",
            policy: CORRECTNESS,
            category: LintCategory::Correctness,
            introduced_in: "0.2.0",
            default_level: RuleLevel::Deny,
            confidence: Confidence::Low,
            lifecycle: LintLifecycleState::Renamed {
                since: "0.3.0",
                to: "NOIR404",
            },
            docs: sample_docs(),
        });

        let err = validate_catalog_integrity(&catalog).expect_err("invalid rename target");
        assert!(
            err.contains("points to missing target"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn removed_and_deprecated_lints_require_metadata() {
        let catalog = vec![
            sample_active_lint("NOIR001"),
            LintSpec {
                id: "NOIR_OLD_DEPRECATED",
                pack: "noir_core",
                policy: CORRECTNESS,
                category: LintCategory::Correctness,
                introduced_in: "0.2.0",
                default_level: RuleLevel::Warn,
                confidence: Confidence::Low,
                lifecycle: LintLifecycleState::Deprecated {
                    since: "",
                    replacement: Some("NOIR001"),
                    note: "",
                },
                docs: sample_docs(),
            },
            LintSpec {
                id: "NOIR_OLD_REMOVED",
                pack: "noir_core",
                policy: CORRECTNESS,
                category: LintCategory::Correctness,
                introduced_in: "0.2.0",
                default_level: RuleLevel::Warn,
                confidence: Confidence::Low,
                lifecycle: LintLifecycleState::Removed {
                    since: "",
                    note: "",
                },
                docs: sample_docs(),
            },
        ];

        let err = validate_catalog_integrity(&catalog)
            .expect_err("deprecated/removed states without metadata should fail");
        assert!(
            err.contains("lifecycle metadata missing required fields"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn deprecated_lints_with_replacements_must_target_existing_active_lints() {
        let mut catalog = vec![sample_active_lint("NOIR001"), sample_active_lint("NOIR100")];
        catalog.push(LintSpec {
            id: "NOIR001_DEPRECATED",
            pack: "noir_core",
            policy: CORRECTNESS,
            category: LintCategory::Correctness,
            introduced_in: "0.2.0",
            default_level: RuleLevel::Warn,
            confidence: Confidence::Low,
            lifecycle: LintLifecycleState::Deprecated {
                since: "0.3.0",
                replacement: Some("NOIR404"),
                note: "use replacement",
            },
            docs: sample_docs(),
        });

        let err = validate_catalog_integrity(&catalog)
            .expect_err("deprecated lint replacement target should exist");
        assert!(
            err.contains("points to missing replacement"),
            "unexpected error: {err}"
        );
    }

    fn sample_active_lint(id: &'static str) -> LintSpec {
        LintSpec {
            id,
            pack: "noir_core",
            policy: CORRECTNESS,
            category: LintCategory::Correctness,
            introduced_in: "0.1.0",
            default_level: RuleLevel::Deny,
            confidence: Confidence::High,
            lifecycle: LintLifecycleState::Active,
            docs: sample_docs(),
        }
    }

    const fn sample_docs() -> LintDocs {
        LintDocs {
            summary: "summary",
            what_it_does: "what it does",
            why_this_matters: "why this matters",
            known_limitations: "known limitations",
            how_to_fix: "how to fix",
            examples: &["example"],
            references: &["docs/reference.md"],
        }
    }
}
