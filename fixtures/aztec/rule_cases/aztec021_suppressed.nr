#[aztec]
pub contract C {
    #[external("private")]
    #[allow(aztec_pack::AZTEC021)]
    fn bridge(secret: Field) {
        let digest = hash(secret);
        emit(digest);
    }
}
