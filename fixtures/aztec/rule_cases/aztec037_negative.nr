#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        if 1 == 1 {
            self.storage.notes.insert(secret).deliver(0);
        }
    }
}
