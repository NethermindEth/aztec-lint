use crate::config::RuleLevel;
use crate::diagnostics::Confidence;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LintCategory {
    Correctness,
    Maintainability,
    Privacy,
    Protocol,
    Soundness,
}

impl LintCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Correctness => "correctness",
            Self::Maintainability => "maintainability",
            Self::Privacy => "privacy",
            Self::Protocol => "protocol",
            Self::Soundness => "soundness",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LintLifecycleState {
    Active,
    Deprecated {
        since: &'static str,
        replacement: Option<&'static str>,
        note: &'static str,
    },
    Renamed {
        since: &'static str,
        to: &'static str,
    },
    Removed {
        since: &'static str,
        note: &'static str,
    },
}

impl LintLifecycleState {
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    pub const fn has_required_metadata(self) -> bool {
        match self {
            Self::Active => true,
            Self::Deprecated {
                since,
                replacement,
                note,
            } => {
                !since.is_empty()
                    && !note.is_empty()
                    && match replacement {
                        Some(rule_id) => !rule_id.is_empty(),
                        None => true,
                    }
            }
            Self::Renamed { since, to } => !since.is_empty() && !to.is_empty(),
            Self::Removed { since, note } => !since.is_empty() && !note.is_empty(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LintDocs {
    pub summary: &'static str,
    pub what_it_does: &'static str,
    pub why_this_matters: &'static str,
    pub known_limitations: &'static str,
    pub how_to_fix: &'static str,
    pub examples: &'static [&'static str],
    pub references: &'static [&'static str],
}

impl LintDocs {
    pub const fn has_required_fields(self) -> bool {
        !self.summary.is_empty()
            && !self.what_it_does.is_empty()
            && !self.why_this_matters.is_empty()
            && !self.known_limitations.is_empty()
            && !self.how_to_fix.is_empty()
            && !self.examples.is_empty()
            && !self.references.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LintSpec {
    pub id: &'static str,
    pub pack: &'static str,
    pub policy: &'static str,
    pub category: LintCategory,
    pub introduced_in: &'static str,
    pub default_level: RuleLevel,
    pub confidence: Confidence,
    pub lifecycle: LintLifecycleState,
    pub docs: LintDocs,
}
