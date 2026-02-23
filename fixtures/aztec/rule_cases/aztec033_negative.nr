#[aztec]
pub contract C {
    #[external("public")]
    #[only_self]
    fn mutate_private() {
        self.storage.notes.insert(1).deliver(0);
    }
}
