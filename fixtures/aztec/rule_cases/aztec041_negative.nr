#[aztec]
pub contract C {
    #[external("private")]
    fn settle(raw: Field, amount: u256) {
        assert(raw < MAX_U128);
        let narrowed = raw as u128;
        assert(amount < MAX_FIELD_SAFE);
        let canonical = amount as Field;
        emit(canonical + narrowed as Field);
    }
}
