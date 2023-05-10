#[path = "../src/addor.rs"]
mod addor;

#[test]
fn add_two_and_two() {
    assert_eq!(4, addor::add_two(2));
}

#[test]
fn add_three_and_two() {
    assert_eq!(5, addor::add_two(3));
}

#[test]
fn one_hundred() {
    assert_eq!(102, addor::add_two(100));
}
