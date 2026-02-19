#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(other: Field) {
        self.enqueue(Contract::at(other).mint_public(1));
    }

    #[external("public")]
    fn mint_public(value: Field) {
        emit(value);
    }
}
