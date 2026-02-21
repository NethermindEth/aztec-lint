#[allow(NOIR020)]
fn main(arr: [Field; 4], idx: u32) {
    let value = arr[idx];
    assert(value == value);
}
