#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        assert(secret < 100);
        let digest = hash(secret);
        emit(digest);
    }
}
