#[aztec]
pub contract C {
    #[external("private")]
    fn spend(note: Field) {
        let change_note = ChangeNote { randomness: note.randomness };
        emit(change_note);
    }
}
