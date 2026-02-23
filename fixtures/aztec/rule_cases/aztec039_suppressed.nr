#[aztec]
pub contract C {
    #[external("private")]
    #[allow(aztec_pack::AZTEC039)]
    fn spend(note: Field, mut remaining: Field, spent: Field) {
        remaining -= note.amount;
        let change = note.amount - remaining;
        emit(change + spent);
    }
}
