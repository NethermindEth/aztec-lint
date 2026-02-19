#[aztec]
pub contract C {
    #[external("private")]
    fn bridge(secret: Field) {
        let witness = get_merkle_witness(secret);
        emit(witness);
    }
}
