#[aztec]
pub contract C {
    #[external("public")]
    #[allow(aztec_pack::AZTEC033)]
    fn mutate_private() {
        self.storage.notes.insert(1).deliver(0);
    }
}
