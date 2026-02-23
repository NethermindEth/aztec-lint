#[aztec]
pub contract C {
    #[initializer]
    #[external("public")]
    fn constructor_with_asset(owner: Field) {
        emit(owner);
    }
}
