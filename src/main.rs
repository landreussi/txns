fn main() {
    let arg = std::env::args()
        .last()
        .and_then(|arg| std::fs::read_to_string(arg).ok())
        .expect("only one valid transaction file should be passed");
    println!("{arg}");
}
