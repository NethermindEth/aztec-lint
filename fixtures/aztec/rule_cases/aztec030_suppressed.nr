#[aztec]
pub contract C {
    #[external("private")]
    #[allow(aztec_pack::AZTEC030)]
    fn consume_note() {
        let note = self.notes.pop_note();
        emit(note);
    }
}
