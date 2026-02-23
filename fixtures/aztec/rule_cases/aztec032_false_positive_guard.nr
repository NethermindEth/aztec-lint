#[aztec]
pub contract C {
    #[external("private")]
    fn commitment_non_hash(value: Field) {
        emit_commitment(value);
    }
}
