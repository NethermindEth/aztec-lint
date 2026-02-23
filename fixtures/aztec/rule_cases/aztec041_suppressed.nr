#[aztec]
pub contract C {
    #[external("private")]
    #[allow(aztec_pack::AZTEC041)]
    fn settle(raw: Field, amount: u256) {
        let narrowed = raw as u128;
        let canonical = amount as Field;
        emit(narrowed as Field + canonical);
    }
}
