#[aztec]
pub contract C {
    #[storage]
    struct Storage<Context> {
        nft_exists: Map<Field, PublicMutable<bool, Context>, Context>,
    }

    #[external("public")]
    fn update_public_flag() {
        self.storage.nft_exists.at(1).write(true);
    }
}
