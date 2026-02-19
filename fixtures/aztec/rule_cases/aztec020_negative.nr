#[aztec]
pub contract C {
    #[external("private")]
    fn bridge() {
        let value = 7;
        emit_nullifier(value);
    }
}
