#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleDoc {
    pub id: &'static str,
    pub pack: &'static str,
    pub policy: &'static str,
    pub confidence: &'static str,
    pub summary: &'static str,
}

const RULES: &[RuleDoc] = &[
    RuleDoc {
        id: "AZTEC001",
        pack: "aztec_pack",
        policy: "privacy",
        confidence: "medium",
        summary: "Private data reaches a public sink.",
    },
    RuleDoc {
        id: "AZTEC002",
        pack: "aztec_pack",
        policy: "privacy",
        confidence: "high",
        summary: "Secret-dependent branching affects public state.",
    },
    RuleDoc {
        id: "AZTEC003",
        pack: "aztec_pack",
        policy: "privacy",
        confidence: "high",
        summary: "Private entrypoint uses debug logging.",
    },
    RuleDoc {
        id: "AZTEC010",
        pack: "aztec_pack",
        policy: "protocol",
        confidence: "high",
        summary: "Private to public bridge requires #[only_self].",
    },
    RuleDoc {
        id: "AZTEC011",
        pack: "aztec_pack",
        policy: "protocol",
        confidence: "high",
        summary: "Nullifier domain separation fields are missing.",
    },
    RuleDoc {
        id: "AZTEC012",
        pack: "aztec_pack",
        policy: "protocol",
        confidence: "high",
        summary: "Commitment domain separation fields are missing.",
    },
    RuleDoc {
        id: "AZTEC020",
        pack: "aztec_pack",
        policy: "soundness",
        confidence: "high",
        summary: "Unconstrained influence reaches commitments, storage, or nullifiers.",
    },
    RuleDoc {
        id: "AZTEC021",
        pack: "aztec_pack",
        policy: "soundness",
        confidence: "high",
        summary: "Missing range constraints before hashing or serialization.",
    },
    RuleDoc {
        id: "AZTEC022",
        pack: "aztec_pack",
        policy: "soundness",
        confidence: "high",
        summary: "Suspicious Merkle witness usage.",
    },
    RuleDoc {
        id: "AZTEC040",
        pack: "aztec_pack",
        policy: "constraint_cost",
        confidence: "medium",
        summary: "Expensive primitive appears inside a loop.",
    },
    RuleDoc {
        id: "AZTEC041",
        pack: "aztec_pack",
        policy: "constraint_cost",
        confidence: "medium",
        summary: "Repeated membership proofs detected.",
    },
    RuleDoc {
        id: "NOIR001",
        pack: "noir_core",
        policy: "correctness",
        confidence: "high",
        summary: "Detects trivially unreachable branch conditions.",
    },
    RuleDoc {
        id: "NOIR002",
        pack: "noir_core",
        policy: "correctness",
        confidence: "high",
        summary: "Detects suspicious variable shadowing.",
    },
    RuleDoc {
        id: "NOIR010",
        pack: "noir_core",
        policy: "correctness",
        confidence: "high",
        summary: "Boolean value computed but never asserted.",
    },
    RuleDoc {
        id: "NOIR020",
        pack: "noir_core",
        policy: "correctness",
        confidence: "high",
        summary: "Array indexing appears without bounds validation.",
    },
    RuleDoc {
        id: "NOIR030",
        pack: "noir_core",
        policy: "correctness",
        confidence: "high",
        summary: "Unconstrained value influences constrained logic.",
    },
    RuleDoc {
        id: "NOIR100",
        pack: "noir_core",
        policy: "maintainability",
        confidence: "low",
        summary: "Detects magic-number literals that should be named.",
    },
    RuleDoc {
        id: "NOIR110",
        pack: "noir_core",
        policy: "maintainability",
        confidence: "medium",
        summary: "Function complexity exceeds the recommended limit.",
    },
    RuleDoc {
        id: "NOIR120",
        pack: "noir_core",
        policy: "maintainability",
        confidence: "medium",
        summary: "Excessive nesting reduces code readability.",
    },
    RuleDoc {
        id: "NOIR200",
        pack: "noir_core",
        policy: "performance",
        confidence: "medium",
        summary: "Heavy operation appears inside a loop.",
    },
];

pub fn all_rules() -> &'static [RuleDoc] {
    RULES
}

pub fn find_rule(rule_id: &str) -> Option<&'static RuleDoc> {
    let canonical = rule_id.trim().to_ascii_uppercase();
    RULES.iter().find(|rule| rule.id == canonical)
}
