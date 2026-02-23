#[aztec]
pub contract C {
    #[external("private")]
    fn spend(note: Field, mut remaining: Field, spent: Field) {
        assert(remaining >= note.amount);
        remaining -= note.amount;
        let change = note.amount - remaining;
        assert(spent + change == note.amount);
        emit(change);
    }
}
