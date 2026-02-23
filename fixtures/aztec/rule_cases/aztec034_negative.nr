#[aztec]
pub contract C {
    #[external("private")]
    fn hash_amount(amount: u128) {
        let max_amount = 1_000_000;
        assert(amount < max_amount);
        let digest = hash(amount as Field);
        emit(digest);
    }
}
