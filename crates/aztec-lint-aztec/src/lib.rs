#![forbid(unsafe_code)]

pub mod detect;
pub mod model_builder;
pub mod patterns;

pub use detect::{SourceUnit, should_activate_aztec};
pub use model_builder::build_aztec_model;

pub fn profile_name() -> &'static str {
    "aztec"
}

pub fn core_version() -> &'static str {
    aztec_lint_core::VERSION
}
