mod math;

fn helper(x: Field) -> Field {
    math::double(x)
}

fn main() {
    let y = helper(3);
    assert(y == 6);
}
