#[aztec]
pub contract C {
    #[external("private")]
    fn bridge() {
        self.enqueue(Contract::at(self.context.this_address()).mint_public(1));
    }

    #[external("public")]
    fn mint_public(value: Field) {
        emit(value);
    }
}
