#[aztec]
pub contract C {
    unconstrained fn read_secret() -> Field {
        7
    }

    #[external("private")]
    #[allow(aztec_pack::AZTEC020)]
    fn bridge() {
        let secret = read_secret();
        emit_nullifier(secret);
    }
}
