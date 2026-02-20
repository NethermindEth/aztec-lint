#![forbid(unsafe_code)]

pub mod config;
pub mod diagnostics;
pub mod fix;
pub mod lints;
pub mod model;
pub mod noir;
pub mod output;
#[cfg(feature = "plugin-api")]
pub mod plugin;
pub mod policy;
pub mod vcs;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CORE_API_STABILITY: &str = "phase-2-stable";

pub fn crate_name() -> &'static str {
    "aztec-lint-core"
}
