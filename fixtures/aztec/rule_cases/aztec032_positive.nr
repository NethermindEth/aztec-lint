#[aztec]
pub contract C {
    #[external("private")]
    fn commitment_bad(value: Field) {
        emit_commitment(hash(value));
    }
}
