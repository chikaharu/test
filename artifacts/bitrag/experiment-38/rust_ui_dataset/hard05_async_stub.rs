// async stub — intentional error: missing async runtime
async fn fetch() -> String { String::from("data") }
fn main() {
    let result = fetch();
    println!("{:?}", result);
}
