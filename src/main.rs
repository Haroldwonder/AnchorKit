fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        if args[1] == "--version" || args[1] == "-v" {
            // Version from Cargo.toml
            println!("AnchorKit v0.1.0");
            return;
        }
    }
    println!("AnchorKit builds successfully. Use cargo test/cargo build for contract checks.");
}
