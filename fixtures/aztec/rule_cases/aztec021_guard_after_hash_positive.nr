#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        let digest = hash(secret);
        assert(secret < 100);
        emit(digest);
    }
}
