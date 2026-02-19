fn main(arr: [Field; 4], idx: u32) {
    assert(idx < arr.len());
    let value = arr[idx];
    assert(value == value);
}
