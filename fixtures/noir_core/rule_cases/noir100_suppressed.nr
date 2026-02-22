#[allow(NOIR100)]
fn main() {
    let fee = 42;
    if fee > 40 {
        assert(fee > 0);
    }
}
