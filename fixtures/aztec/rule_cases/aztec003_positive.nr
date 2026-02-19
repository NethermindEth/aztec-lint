#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        debug_log(secret);
    }
}
