#[aztec]
pub contract C {
    #[external("public")]
    fn mutate_private() {
        self.storage.notes.insert(1).deliver(0);
    }
}
