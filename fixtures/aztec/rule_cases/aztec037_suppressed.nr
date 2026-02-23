#[aztec]
pub contract C {
    #[external("private")]
    #[allow(aztec_pack::AZTEC037)]
    fn bridge(secret: Field) {
        if secret > 10 {
            self.storage.notes.insert(secret).deliver(0);
        } else {
            emit(1);
        }
    }
}
