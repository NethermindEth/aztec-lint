#[aztec]
pub contract C {
    #[external("private")]
    #[allow(aztec_pack::AZTEC038)]
    fn spend(note: Field) {
        let change_note = ChangeNote { randomness: note.randomness };
        emit(change_note);
    }
}
