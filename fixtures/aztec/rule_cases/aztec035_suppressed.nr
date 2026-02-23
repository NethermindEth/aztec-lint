#[aztec]
pub contract C {
    #[external("private")]
    #[allow(aztec_pack::AZTEC035)]
    fn repeated(owner: Field) {
        let slot = self.storage.balances.at(owner).at(owner);
        emit(slot);
    }
}
