#[aztec]
pub contract DomainSeparationBench {
    #[external("private")]
    fn private_bridge(secret: Field, domain: Field) {
        let digest = hash(secret + domain);
        emit(digest);
    }
}
