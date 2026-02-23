#[aztec]
pub contract C {
    #[initializer]
    #[external("public")]
    fn init(owner: Field) {
        emit(owner);
    }
}
