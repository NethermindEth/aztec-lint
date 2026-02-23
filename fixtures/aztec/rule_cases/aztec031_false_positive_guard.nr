#[aztec]
pub contract C {
    #[external("private")]
    fn nullifier_non_hash(value: Field) {
        emit_nullifier(value);
    }
}
