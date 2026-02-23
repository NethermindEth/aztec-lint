#[aztec]
pub contract C {
    #[external("private")]
    #[allow(aztec_pack::AZTEC031)]
    fn nullifier_bad(value: Field) {
        emit_nullifier(hash(value));
    }
}
