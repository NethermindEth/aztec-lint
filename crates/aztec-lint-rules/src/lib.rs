#![forbid(unsafe_code)]

pub fn pack_name() -> &'static str {
    "noir_core"
}

pub fn core_version() -> &'static str {
    aztec_lint_core::VERSION
}
