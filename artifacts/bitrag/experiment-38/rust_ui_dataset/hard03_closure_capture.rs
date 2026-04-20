fn apply<F: Fn(i32) -> i32>(f: F, x: i32) -> i32 { f(x) }
fn main() {
    let factor = 3;
    let triple = |x| x * factor;
    println!("{}", apply(triple, 7));
}
