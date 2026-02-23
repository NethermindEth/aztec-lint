#[aztec]
pub contract C {
    #[external("private")]
    fn nullifier_bad(value: Field) {
        emit_nullifier(hash(value));
    }
}
