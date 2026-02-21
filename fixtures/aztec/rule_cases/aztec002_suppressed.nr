#[aztec]
pub contract C {
    #[external("private")]
    #[allow(AZTEC002)]
    fn bridge(secret: Field) {
        if secret > 10 {
            emit(secret);
        }
    }
}
