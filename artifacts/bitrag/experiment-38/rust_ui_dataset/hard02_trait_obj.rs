trait Animal { fn sound(&self) -> &str; }
struct Dog; struct Cat;
impl Animal for Dog { fn sound(&self) -> &str { "woof" } }
impl Animal for Cat { fn sound(&self) -> &str { "meow" } }
fn make_sound(a: &dyn Animal) { println!("{}", a.sound()); }
fn main() {
    let d = Dog; let c = Cat;
    make_sound(&d); make_sound(&c);
}
