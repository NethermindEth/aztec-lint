#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        let digest = hash(secret);
        emit(digest);
    }
}
