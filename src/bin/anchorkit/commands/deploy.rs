//! `anchorkit deploy` — print the steps to deploy the contract.

use super::build::run_build;

pub fn run_deploy(network: &str) {
    println!("🚀 Deploying to {} network...", network);
    
    // Check if contract is built
    let wasm_path = "target/wasm32-unknown-unknown/release/anchorkit.wasm";
    if !std::path::Path::new(wasm_path).exists() {
        println!("⚠️  Contract not found. Building first...");
        run_build(true);
    }
    
    println!("📋 Deployment steps:");
    println!("  1. Use soroban CLI to deploy:");
    println!("     soroban contract deploy \\");
    println!("       --wasm {} \\", wasm_path);
    println!("       --source <YOUR_ACCOUNT> \\");
    println!("       --network {}", network);
    println!();
    println!("  2. Initialize the contract:");
    println!("     anchorkit init --admin <ADMIN_ADDRESS>");
    println!();
    println!("💡 Note: Actual deployment requires soroban CLI and configured account");
}
