#[aztec]
pub contract C {
    #[external("private")]
    fn hash_field(amount: Field) {
        let digest = hash(amount);
        emit(digest);
    }
}
