#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        if 1 == 1 {
            emit(secret);
        }
    }
}
