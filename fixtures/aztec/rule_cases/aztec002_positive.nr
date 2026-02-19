#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        if secret > 10 {
            emit(secret);
        }
    }
}
