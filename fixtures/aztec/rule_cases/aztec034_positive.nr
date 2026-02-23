#[aztec]
pub contract C {
    #[external("private")]
    fn hash_amount(amount: u128) {
        let digest = hash(amount as Field);
        emit(digest);
    }
}
