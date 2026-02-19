#![forbid(unsafe_code)]

pub mod engine;
pub mod noir_core;

pub use engine::{Rule, RuleEngine, RuleRunSettings};

pub fn pack_name() -> &'static str {
    "noir_core"
}

pub fn core_version() -> &'static str {
    aztec_lint_core::VERSION
}
