#[aztec]
pub contract C {
    #[external("public")]
    fn bridge(value: Field) {
        debug_log(value);
    }
}
