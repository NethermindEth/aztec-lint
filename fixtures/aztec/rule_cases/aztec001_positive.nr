#[aztec]
pub contract C {
    #[external("private")]
    fn bridge() {
        let notes = self.notes.get_notes();
        emit(notes);
    }
}
