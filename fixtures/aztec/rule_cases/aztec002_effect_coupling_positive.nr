#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        if secret > 10 {
            let local = secret + 1;
            assert(local > secret);
        }

        emit(1);
    }
}
