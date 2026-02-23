#[aztec]
pub contract C {
    #[external("private")]
    fn commitment_ok(value: Field, note_type: Field) {
        emit_commitment(hash(value + self.context.this_address() + note_type));
    }
}
