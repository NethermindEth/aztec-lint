#[aztec]
pub contract C {
    #[external("private")]
    fn spend(note: Field, remaining: Field) {
        let change = remaining - note.amount;
        emit(change);
    }
}
