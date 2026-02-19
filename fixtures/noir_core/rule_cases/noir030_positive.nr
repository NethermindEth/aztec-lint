unconstrained fn read_secret() -> Field {
    7
}

fn main() {
    let secret = read_secret();
    assert(secret == 7);
}
