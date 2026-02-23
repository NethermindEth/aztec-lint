#[aztec]
pub contract C {
    #[external("private")]
    fn distinct(owner: Field, spender: Field) {
        let slot = self.storage.balances.at(owner).at(spender);
        emit(slot);
    }
}
