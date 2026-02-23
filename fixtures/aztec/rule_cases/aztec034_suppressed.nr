#[aztec]
pub contract C {
    #[external("private")]
    #[allow(aztec_pack::AZTEC034)]
    fn hash_amount(amount: u128) {
        let digest = hash(amount as Field);
        emit(digest);
    }
}
