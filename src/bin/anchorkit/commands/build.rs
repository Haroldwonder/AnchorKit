//! `anchorkit build` — build the smart contract to WASM.

use std::process::Command;

pub fn run_build(release: bool) {
    println!("🔨 Building AnchorKit smart contract...");
    
    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--target", "wasm32-unknown-unknown"]);
    
    if release {
        cmd.arg("--release");
        println!("  Mode: Release (optimized)");
    } else {
        println!("  Mode: Debug");
    }
    
    match cmd.output() {
        Ok(output) if output.status.success() => {
            println!("✅ Contract built successfully");
            if release {
                println!("  Output: target/wasm32-unknown-unknown/release/anchorkit.wasm");
            } else {
                println!("  Output: target/wasm32-unknown-unknown/debug/anchorkit.wasm");
            }
        }
        Ok(output) => {
            eprintln!("❌ Build failed:");
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("❌ Failed to run cargo build: {}", e);
            std::process::exit(1);
        }
    }
}
