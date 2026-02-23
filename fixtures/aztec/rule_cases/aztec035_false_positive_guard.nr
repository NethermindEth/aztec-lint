#[aztec]
pub contract C {
    #[external("private")]
    fn distinct(owner: Field) {
        let slot = self.storage.balances.at(owner).at(self.context.msg_sender());
        emit(slot);
    }
}
