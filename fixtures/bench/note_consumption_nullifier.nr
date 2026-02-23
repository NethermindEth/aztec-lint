#[aztec]
pub contract NoteNullifierBench {
    #[external("private")]
    fn consume_note() {
        let notes = self.notes.get_notes();
        let note_hash = hash(notes);
        emit_nullifier(note_hash);
    }
}
