#[aztec]
pub contract C {
    #[initializer]
    #[external("public")]
    #[only_self]
    fn init(owner: Field) {
        emit(owner);
    }
}
