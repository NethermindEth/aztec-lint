#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        if secret > 10 {
            self.storage.notes.insert(secret).deliver(0);
        } else {
            emit(1);
        }
    }
}
