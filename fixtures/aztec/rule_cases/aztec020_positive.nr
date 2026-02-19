#[aztec]
pub contract C {
    unconstrained fn read_secret() -> Field {
        7
    }

    #[external("private")]
    fn bridge() {
        let secret = read_secret();
        emit_nullifier(secret);
    }
}
