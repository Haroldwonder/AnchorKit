//! `anchorkit config validate|init` — run validate_config.py or interactively
//! create a validated config file.

use std::process::Command;

use regex::Regex;

use super::validate::{validate_endpoint_url, validate_file};
use crate::validation::{validate_contract_id, validate_stellar_address};

pub fn run_config_validate(path: &str) {
    // Resolve the script path relative to the binary's location so it works
    // regardless of where the user invokes the CLI from.
    let script_path = {
        let mut p = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        p.push("validate_config.py");
        if !p.exists() {
            // Fall back to the repo root (development layout: target/…/anchorkit)
            p = std::env::current_exe()
                .ok()
                .and_then(|e| {
                    // Walk up until we find validate_config.py (handles
                    // target/debug/ and target/release/ layouts)
                    let mut dir = e.parent()?.to_path_buf();
                    for _ in 0..5 {
                        let candidate = dir.join("validate_config.py");
                        if candidate.exists() {
                            return Some(candidate);
                        }
                        dir = dir.parent()?.to_path_buf();
                    }
                    None
                })
                .unwrap_or_else(|| std::path::PathBuf::from("validate_config.py"));
        }
        p
    };

    if !script_path.exists() {
        eprintln!(
            "❌ validate_config.py not found (looked in: {})",
            script_path.display()
        );
        eprintln!("   Make sure validate_config.py is present in the project root.");
        std::process::exit(1);
    }

    // Require python3
    let python = if Command::new("python3").arg("--version").output().is_ok() {
        "python3"
    } else if Command::new("python").arg("--version").output().is_ok() {
        "python"
    } else {
        eprintln!("❌ Python 3 is required but was not found in PATH.");
        eprintln!("   Install Python 3.7+ from https://www.python.org/downloads/");
        std::process::exit(1)
    };

    println!("🔍 Running config validation on: {}", path);

    // validate_config.py scans configs/ by default (no CLI args).
    // When the caller passes a custom path we set ANCHORKIT_CONFIG_PATH so
    // the script can honour it without breaking backward-compat.
    let mut cmd = Command::new(python);
    cmd.arg(script_path.as_os_str());

    // Pass the path as an env var; validate_config.py picks it up when set.
    // This keeps the Python script's own __main__ block intact.
    cmd.env("ANCHORKIT_CONFIG_PATH", path);

    match cmd.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stdout.is_empty() {
                print!("{}", stdout);
            }
            if !stderr.is_empty() {
                eprint!("{}", stderr);
            }
            if !output.status.success() {
                std::process::exit(output.status.code().unwrap_or(1));
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to execute validate_config.py: {}", e);
            std::process::exit(1);
        }
    }
}

/// Reads a line from stdin, trimming whitespace. Returns `None` on EOF or error.
fn read_input(prompt: &str) -> Option<String> {
    use std::io::Write;
    print!("{}", prompt);
    std::io::stdout().flush().ok()?;
    let mut buf = String::new();
    match std::io::stdin().read_line(&mut buf) {
        Ok(0) => None, // EOF
        Ok(_) => Some(buf.trim().to_string()),
        Err(_) => None,
    }
}

/// Like `read_input` but re-prompts until the value passes `validate`.
/// `validate` returns `Ok(T)` on success or `Err(hint)` with a human-readable
/// reason on failure.
fn prompt_until_valid<T, F>(prompt: &str, validate: F) -> T
where
    F: Fn(&str) -> Result<T, String>,
{
    loop {
        match read_input(prompt) {
            None => {
                eprintln!("\n❌ Unexpected end of input. Aborting.");
                std::process::exit(1);
            }
            Some(raw) => match validate(&raw) {
                Ok(v) => return v,
                Err(hint) => eprintln!("  ✖ {}", hint),
            },
        }
    }
}

/// Validate the network choice; returns the canonical `stellar-*` value.
fn validate_network(s: &str) -> Result<String, String> {
    match s.trim().to_lowercase().as_str() {
        "testnet" | "stellar-testnet" | "1" => Ok("stellar-testnet".to_string()),
        "mainnet" | "stellar-mainnet" | "2" => Ok("stellar-mainnet".to_string()),
        "futurenet" | "stellar-futurenet" | "3" => Ok("stellar-futurenet".to_string()),
        _ => Err(format!(
            "Unknown network '{}'. Enter 1/testnet, 2/mainnet, or 3/futurenet",
            s
        )),
    }
}

/// Validate an HTTPS RPC endpoint URL using the same rules as the config
/// validator so there are no surprises later.
fn validate_rpc_url(s: &str) -> Result<String, String> {
    if let Some(err) = validate_endpoint_url(s) {
        Err(err)
    } else {
        Ok(s.to_string())
    }
}

/// Derive a safe output filename from the anchor name the user will supply.
fn config_output_path(anchor_name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("configs/{}.json", anchor_name))
}

pub fn run_config_init() {
    println!("🛠  AnchorKit config init");
    println!("   Answer each prompt to generate a validated config file.");
    println!("   Press Ctrl+C at any time to abort.\n");

    // ── 1. Anchor / contract name ──────────────────────────────────────────
    let anchor_name: String = prompt_until_valid(
        "Anchor name (lowercase, hyphens only, e.g. my-anchor): ",
        |s| {
            let re = Regex::new(r"^[a-z0-9-]+$").unwrap();
            if s.is_empty() {
                return Err("Name must not be empty".to_string());
            }
            if s.len() > 64 {
                return Err(format!("Name is too long ({} chars, max 64)", s.len()));
            }
            if !re.is_match(s) {
                return Err("Name must contain only lowercase letters, digits, and hyphens".to_string());
            }
            Ok(s.to_string())
        },
    );

    // ── 2. Network ─────────────────────────────────────────────────────────
    println!("\nNetwork options:");
    println!("  1  testnet   (stellar-testnet)");
    println!("  2  mainnet   (stellar-mainnet)");
    println!("  3  futurenet (stellar-futurenet)");
    let network: String = prompt_until_valid("Network [1/testnet]: ", |s| {
        // Allow empty → default to testnet
        let input = if s.is_empty() { "testnet" } else { s };
        validate_network(input)
    });

    // ── 3. Admin public key ────────────────────────────────────────────────
    println!();
    let admin_key: String = prompt_until_valid(
        "Admin Stellar public key (starts with G, 56 chars): ",
        |s| validate_stellar_address(s),
    );

    // ── 4. RPC endpoint ────────────────────────────────────────────────────
    let default_rpc = match network.as_str() {
        "stellar-testnet" => "https://soroban-testnet.stellar.org",
        "stellar-mainnet" => "https://horizon.stellar.org/rpc",
        _ => "https://rpc-futurenet.stellar.org",
    };
    println!();
    println!("  Default RPC for {}: {}", network, default_rpc);
    if network == "stellar-mainnet" {
        println!("  ⚠  Note: This is a third-party RPC endpoint, not operated by the Stellar Development Foundation.");
        println!("     It will receive your mainnet traffic including session and attestation data.");
        println!("     Consider using your own RPC node or an SDF-operated endpoint for production use.");
    }
    let rpc_endpoint: String = prompt_until_valid(
        &format!("RPC endpoint URL [{}]: ", default_rpc),
        |s| {
            let url = if s.is_empty() { default_rpc } else { s };
            validate_rpc_url(url)
        },
    );

    // ── 5. Contract ID ─────────────────────────────────────────────────────
    println!();
    println!("  Contract ID is the deployed Soroban contract address (starts with C, 56 chars).");
    println!("  Leave blank to skip — you can add it later after deployment.");
    let contract_id: Option<String> = {
        loop {
            match read_input("Contract ID (optional, press Enter to skip): ") {
                None => {
                    eprintln!("\n❌ Unexpected end of input. Aborting.");
                    std::process::exit(1);
                }
                Some(ref s) if s.is_empty() => break None,
                Some(ref s) => match validate_contract_id(s) {
                    Ok(id) => break Some(id),
                    Err(hint) => eprintln!("  ✖ {}", hint),
                },
            }
        }
    };

    // ── Build the config JSON ──────────────────────────────────────────────
    // The config schema requires contract/attestors/sessions.
    // We embed the admin key and rpc_endpoint as metadata in the contract
    // section (description field) and store the contract_id there too if
    // provided, because the schema's "contract" object only allows the
    // known fields (name, version, description, network).
    // The rpc/admin/contract_id are written as top-level extra fields under
    // "deployment" — stored outside the schema-validated sections so that
    // `anchorkit validate` stays happy while the values are preserved for
    // tooling.
    let config = build_init_config(&anchor_name, &network, &admin_key, &rpc_endpoint, contract_id.as_deref());

    // ── Write to disk ──────────────────────────────────────────────────────
    let out_path = config_output_path(&anchor_name);

    // Warn if file already exists
    if out_path.exists() {
        println!();
        print!(
            "⚠  '{}' already exists. Overwrite? [y/N]: ",
            out_path.display()
        );
        use std::io::Write as _;
        std::io::stdout().flush().ok();
        let mut answer = String::new();
        let _ = std::io::stdin().read_line(&mut answer);
        if !matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Aborted. No changes written.");
            std::process::exit(0);
        }
    }

    // Ensure configs/ directory exists
    if let Err(e) = std::fs::create_dir_all("configs") {
        eprintln!("❌ Failed to create configs/ directory: {}", e);
        std::process::exit(1);
    }

    let json_str = serde_json::to_string_pretty(&config)
        .expect("serialization cannot fail for a known-good Value");

    if let Err(e) = std::fs::write(&out_path, &json_str) {
        eprintln!("❌ Failed to write {}: {}", out_path.display(), e);
        std::process::exit(1);
    }

    // ── Validate the file we just wrote ───────────────────────────────────
    println!();
    println!("✅ Config written to {}", out_path.display());
    println!();
    println!("🔍 Validating generated config…");
    let valid = validate_file(&out_path);

    if valid {
        println!();
        println!("🎉 Your config is ready. Next steps:");
        println!("   1. Review and customise {}", out_path.display());
        println!("      • Add real attestor address(es) under attestors.registry");
        println!("      • Adjust session settings if needed");
        if contract_id.is_none() {
            println!("   2. Deploy the contract and add its ID to the config");
            println!("      • anchorkit deploy --network {}", network.replace("stellar-", ""));
        }
        println!("   3. Run `anchorkit doctor` to verify your environment");
    } else {
        eprintln!();
        eprintln!("⚠  The generated file failed validation — this is a bug in anchorkit.");
        eprintln!("   Please open an issue and include the file contents above.");
        std::process::exit(1);
    }
}

/// Construct the full config `serde_json::Value` from the gathered inputs.
fn build_init_config(
    anchor_name: &str,
    network: &str,
    admin_key: &str,
    rpc_endpoint: &str,
    contract_id: Option<&str>,
) -> serde_json::Value {
    use serde_json::json;

    let mut config = json!({
        "contract": {
            "name": anchor_name,
            "version": "1.0.0",
            "description": format!("AnchorKit config for {}", anchor_name),
            "network": network
        },
        "attestors": {
            "registry": [
                {
                    "name": "primary-attestor",
                    "address": admin_key,
                    "description": "Replace with your real attestor address",
                    "endpoint": "https://attestor.example.com/verify",
                    "role": "attestor",
                    "enabled": true
                }
            ]
        },
        "sessions": {
            "enable_session_tracking": true,
            "session_timeout_seconds": 3600,
            "operations_per_session": 100,
            "audit_log_retention_days": 30
        },
        "deployment": {
            "admin_key": admin_key,
            "rpc_endpoint": rpc_endpoint,
            "network": network
        }
    });

    if let Some(id) = contract_id {
        config["deployment"]["contract_id"] = serde_json::Value::String(id.to_string());
    }

    config
}
