#![forbid(unsafe_code)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn crate_name() -> &'static str {
    "aztec-lint-core"
}
