#![forbid(unsafe_code)]

fn main() {
    let _core = aztec_lint_core::crate_name();
    let _rules = aztec_lint_rules::pack_name();
    let _aztec = aztec_lint_aztec::profile_name();
    println!("aztec-lint CLI scaffold");
}
