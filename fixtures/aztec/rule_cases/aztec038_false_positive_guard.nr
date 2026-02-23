#[aztec]
pub contract C {
    #[external("private")]
    fn spend(note: Field, nonce: Field) {
        let fresh_randomness = derive_randomness(note, nonce);
        let change_note = ChangeNote { randomness: fresh_randomness };
        emit(change_note);
    }
}
