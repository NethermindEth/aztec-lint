#[aztec]
pub contract C {
    #[external("private")]
    fn read_only() {
        let notes = self.notes.get_notes();
        emit(notes);
    }
}
