fn main(arr: [Field; 4], idx: u32, checked: bool) {
    if checked {
        assert(idx < arr.len());
    }
    let value = arr[idx];
    assert(value == value);
}
