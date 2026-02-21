unconstrained fn read_secret() -> Field {
    7
}

#[allow(NOIR030)]
fn main() {
    let secret = read_secret();
    assert(secret == 7);
}
