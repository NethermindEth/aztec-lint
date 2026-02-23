#[aztec]
pub contract C {
    #[external("private")]
    fn nullifier_ok(value: Field, nonce: Field) {
        emit_nullifier(hash(value + self.context.this_address() + nonce));
    }
}
