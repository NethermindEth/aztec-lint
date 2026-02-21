fn main(arr: [Field; 4], idx: u32) {
    let value = arr[idx];
    assert(idx < arr.len());
    assert(value == value);
}
