#[aztec]
pub contract C {
    #[external("private")]
    #[allow(aztec_pack::AZTEC032)]
    fn commitment_bad(value: Field) {
        emit_commitment(hash(value));
    }
}
