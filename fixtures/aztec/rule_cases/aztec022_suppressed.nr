#[aztec]
pub contract C {
    #[external("private")]
    #[allow(aztec_pack::AZTEC022)]
    fn bridge(secret: Field) {
        let path = get_merkle_witness(secret);
        emit(path);
    }
}
