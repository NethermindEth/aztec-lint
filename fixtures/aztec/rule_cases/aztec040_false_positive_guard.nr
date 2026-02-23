#[aztec]
pub contract C {
    #[external("public")]
    fn rotate(owner: Field) {
        emit(owner);
    }
}
