#[derive(Debug)]
struct Point { x: f64, y: f64 }
impl Point {
    fn distance(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}
fn main() {
    let p = Point { x: 0.0, y: 0.0 };
    let q = Point { x: 3.0, y: 4.0 };
    println!("{}", p.distance(&q));
}
