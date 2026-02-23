#[aztec]
pub contract C {
    #[external("public")]
    fn view_only() {
        emit(1);
    }
}
