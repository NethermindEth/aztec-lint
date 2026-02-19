#[aztec]
pub contract C {
    #[external("private")]
    #[allow(AZTEC001)]
    fn bridge() {
        let notes = self.notes.get_notes();
        emit(notes);
    }
}
