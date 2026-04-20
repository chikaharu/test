use std::fmt::Display;
fn print_largest<T: PartialOrd + Display>(list: &[T]) {
    let mut largest = &list[0];
    for item in list { if item > largest { largest = item; } }
    println!("{}", largest);
}
fn main() {
    print_largest(&[34, 50, 25, 100, 65]);
}
