#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        if secret > 10 {
            emit(1);
        }

        self.enqueue(Contract::at(self.context.this_address()).mint_public(42));
    }

    #[external("public")]
    #[only_self]
    fn mint_public(value: Field) {
        emit(value);
    }
}
