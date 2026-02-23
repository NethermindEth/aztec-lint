#[aztec]
pub contract LoopHashMerkleBench {
    #[external("private")]
    fn verify(secret: Field) {
        let mut acc = secret;
        for i in 0..16 {
            let hashed = hash(acc + i);
            let witness = merkle_verify(hashed);
            acc = witness;
        }
        emit(acc);
    }
}
