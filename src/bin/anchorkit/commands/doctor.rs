//! `anchorkit doctor` — environment diagnostics.

use std::process::Command;
use std::time::Instant;

use super::validate::validate_file;

const MIN_RUST_MAJOR: u32 = 1;
const MIN_RUST_MINOR: u32 = 74;

pub fn run_doctor() {
    println!("🔍 Running AnchorKit diagnostics...\n");
    let start = Instant::now();
    let mut all_ok = true;

    all_ok &= check_rust_version();
    all_ok &= check_wasm_target();
    all_ok &= check_wallet();
    all_ok &= check_rpc();
    all_ok &= check_configs();
    all_ok &= check_network();

    println!("\n⏱  Completed in {:.2}s\n", start.elapsed().as_secs_f64());

    if all_ok {
        println!("✅ All checks passed! Your environment is ready.");
        std::process::exit(0);
    } else {
        println!("⚠️  Some checks failed. Please address the issues above.");
        std::process::exit(1);
    }
}

fn check_rust_version() -> bool {
    match Command::new("rustc").arg("--version").output() {
        Err(_) => {
            println!("✖ Rust toolchain not found → install from https://rustup.rs");
            false
        }
        Ok(out) => {
            let version_str = String::from_utf8_lossy(&out.stdout);
            if let Some((major, minor)) = parse_rustc_version(&version_str) {
                if major > MIN_RUST_MAJOR || (major == MIN_RUST_MAJOR && minor >= MIN_RUST_MINOR) {
                    println!("✔ Rust {}.{} detected (meets minimum {}.{}+)", major, minor, MIN_RUST_MAJOR, MIN_RUST_MINOR);
                    true
                } else {
                    println!(
                        "✖ Rust {}.{} detected but {}.{}+ is required (edition 2021)\n  \
                         → Run: rustup update stable",
                        major, minor, MIN_RUST_MAJOR, MIN_RUST_MINOR
                    );
                    false
                }
            } else {
                println!("✖ Could not parse rustc version: {}", version_str.trim());
                false
            }
        }
    }
}

/// Parse "rustc X.Y.Z ..." → (X, Y)
fn parse_rustc_version(s: &str) -> Option<(u32, u32)> {
    let version_part = s.split_whitespace().nth(1)?;
    let mut parts = version_part.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    Some((major, minor))
}

fn check_wasm_target() -> bool {
    let out = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output();
    match out {
        Ok(o) if String::from_utf8_lossy(&o.stdout).contains("wasm32-unknown-unknown") => {
            println!("✔ WASM target installed");
            true
        }
        _ => {
            println!("✖ WASM target missing → run: rustup target add wasm32-unknown-unknown");
            false
        }
    }
}

fn check_wallet() -> bool {
    let vars = ["STELLAR_SECRET_KEY", "SOROBAN_SECRET_KEY", "ANCHORKIT_SECRET_KEY"];
    if vars.iter().any(|v| std::env::var(v).is_ok()) {
        println!("✔ Wallet configured");
        return true;
    }
    let identity_dir = std::env::var("HOME").ok().map(|h| h + "/.config/soroban/identity");
    if let Some(dir) = identity_dir {
        if std::path::Path::new(&dir).exists() {
            println!("✔ Wallet configured (soroban identity)");
            return true;
        }
    }
    println!("✖ Wallet not configured → set STELLAR_SECRET_KEY or configure soroban identity");
    false
}

fn check_rpc() -> bool {
    let vars = ["ANCHORKIT_RPC_URL", "SOROBAN_RPC_URL", "STELLAR_RPC_URL"];
    let url = vars.iter().find_map(|v| std::env::var(v).ok());

    let url = match url {
        Some(u) => u,
        None => {
            println!("✖ RPC endpoint not configured → set ANCHORKIT_RPC_URL, SOROBAN_RPC_URL, or STELLAR_RPC_URL");
            return false;
        }
    };

    let result = Command::new("curl")
        .args(["-s", "--max-time", "5", "-o", "/dev/null", "-w", "%{http_code}", &url])
        .output();

    match result {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("✖ curl not found → install curl to verify RPC reachability");
            false
        }
        Err(e) => {
            println!("✖ RPC probe failed: {}", e);
            false
        }
        Ok(o) if String::from_utf8_lossy(&o.stdout).trim() == "000" => {
            println!("✖ RPC endpoint unreachable → check that {} is accessible", url);
            false
        }
        Ok(_) => {
            println!("✔ RPC endpoint reachable ({})", url);
            true
        }
    }
}

fn check_configs() -> bool {
    let configs = std::path::Path::new("configs");
    if !configs.exists() {
        println!("✖ configs/ directory not found");
        return false;
    }
    let entries = match std::fs::read_dir(configs) {
        Ok(entries) => entries,
        Err(e) => {
            println!("✖ Cannot read configs/ directory: {}", e);
            return false;
        }
    };
    let mut config_files = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                println!("✖ Failed to read an entry in configs/: {}", e);
                return false;
            }
        };
        let path = entry.path();
        if path.is_file()
            && matches!(
                path.extension().and_then(|s| s.to_str()),
                Some("json") | Some("toml")
            )
        {
            config_files.push(path);
        }
    }
    config_files.sort();
    if config_files.is_empty() {
        println!("✖ No config files found in configs/");
        return false;
    }

    let count = config_files.len();
    let mut all_valid = true;
    for path in &config_files {
        all_valid &= validate_file(path);
    }
    if all_valid {
        println!("✔ Config files valid ({} found)", count);
    } else {
        println!("✖ Config validation failed ({} files checked)", count);
    }
    all_valid
}

fn check_network() -> bool {
    let url = std::env::var("ANCHORKIT_HEALTH_URL")
        .unwrap_or_else(|_| "https://horizon-testnet.stellar.org".to_string());

    let result = Command::new("curl")
        .args(["-s", "--max-time", "3", "-o", "/dev/null", "-w", "%{http_code}", &url])
        .output();

    match result {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("✖ curl not found → install curl to enable network checks");
            false
        }
        Err(e) => {
            println!("✖ failed to run curl: {}", e);
            false
        }
        Ok(o) if String::from_utf8_lossy(&o.stdout).trim() == "000" => {
            println!("✖ Network unreachable → check internet connection (probe: {})", url);
            false
        }
        Ok(_) => {
            println!("✔ Network responding (probe: {})", url);
            true
        }
    }
}
