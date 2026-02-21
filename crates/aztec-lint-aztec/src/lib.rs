#![forbid(unsafe_code)]

pub mod detect;
pub mod model_builder;
pub mod patterns;
pub mod taint;

pub use detect::{SourceUnit, should_activate_aztec};
pub use model_builder::{build_aztec_model, build_aztec_model_with_semantic};

pub fn profile_name() -> &'static str {
    "aztec"
}

pub fn core_version() -> &'static str {
    aztec_lint_core::VERSION
}
