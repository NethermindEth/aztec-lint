#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        let witness = get_merkle_witness(secret);
        let ok = verify_merkle(witness, secret);
        assert(ok);
        emit(secret);
    }
}
