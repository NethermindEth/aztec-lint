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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LintDocs {
    pub summary: &'static str,
    pub details: &'static str,
}

impl LintDocs {
    pub const fn has_required_fields(self) -> bool {
        !self.summary.is_empty() && !self.details.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LintSpec {
    pub id: &'static str,
    pub pack: &'static str,
    pub policy: &'static str,
    pub category: LintCategory,
    pub default_level: RuleLevel,
    pub confidence: Confidence,
    pub lifecycle: LintLifecycleState,
    pub docs: LintDocs,
}
