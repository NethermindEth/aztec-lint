#![forbid(unsafe_code)]

pub fn profile_name() -> &'static str {
    "aztec"
}

pub fn core_version() -> &'static str {
    aztec_lint_core::VERSION
}
