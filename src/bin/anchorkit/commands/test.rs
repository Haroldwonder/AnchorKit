//! `anchorkit test` — run the contract test suite.

use std::process::Command;

pub fn run_test(pattern: Option<&str>, verbose: bool) {
    println!("🧪 Running contract tests...");
    
    let mut cmd = Command::new("cargo");
    cmd.arg("test");
    
    if let Some(p) = pattern {
        cmd.arg(p);
        println!("  Pattern: {}", p);
    }
    
    if verbose {
        cmd.arg("--verbose");
        println!("  Mode: Verbose");
    }
    
    match cmd.output() {
        Ok(output) if output.status.success() => {
            println!("✅ All tests passed");
            if verbose {
                println!("{}", String::from_utf8_lossy(&output.stdout));
            }
        }
        Ok(output) => {
            eprintln!("❌ Some tests failed:");
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("❌ Failed to run tests: {}", e);
            std::process::exit(1);
        }
    }
}
