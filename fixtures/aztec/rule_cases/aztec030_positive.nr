#[aztec]
pub contract C {
    #[external("private")]
    fn consume_note() {
        let note = self.notes.pop_note();
        emit(note);
    }
}
