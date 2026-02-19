#[aztec]
pub contract C {
    #[external("private")]
    #[allow(AZTEC003)]
    fn bridge(secret: Field) {
        debug_log(secret);
    }
}
