#[allow(NOIR002)]
fn main() {
    let value = 1;
    {
        let value = 2;
        assert(value == 2);
    }
}
