#[aztec]
pub contract C {
    #[external("private")]
    fn settle(raw: Field, amount: u256) {
        let narrowed = checked_to_u128(raw as u128);
        let canonical = checked_to_field(amount as Field);
        emit(narrowed as Field + canonical);
    }
}
