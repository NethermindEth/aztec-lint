#[aztec]
pub contract C {
    #[initializer]
    #[external("public")]
    #[allow(aztec_pack::AZTEC040)]
    fn init(owner: Field) {
        emit(owner);
    }
}
