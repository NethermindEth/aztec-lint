#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        if secret > 10 {
            self.enqueue(Contract::at(self.context.this_address()).mint_public(secret));
        } else {
            self.enqueue(Contract::at(self.context.this_address()).mint_public(secret + 1));
        }
    }

    #[external("public")]
    #[only_self]
    fn mint_public(value: Field) {
        emit(value);
    }
}
