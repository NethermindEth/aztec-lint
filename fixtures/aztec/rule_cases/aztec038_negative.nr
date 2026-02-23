#[aztec]
pub contract C {
    #[external("private")]
    fn spend(note: Field, nonce: Field) {
        let change_note = ChangeNote { randomness: hash(note, nonce) };
        emit(change_note);
    }
}
