#[aztec]
pub contract C {
    #[external("private")]
    fn repeated(owner: Field) {
        let slot = self.storage.balances.at(owner).at(owner);
        emit(slot);
    }
}
