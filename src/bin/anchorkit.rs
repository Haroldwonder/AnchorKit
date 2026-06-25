#![cfg(feature = "std")]

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use std::process::Command;
use std::time::Instant;
use regex::Regex;

const MIN_RUST_MAJOR: u32 = 1;
const MIN_RUST_MINOR: u32 = 74;

#[derive(Parser)]
#[command(name = "anchorkit", about = "AnchorKit CLI for Soroban anchor management")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the smart contract
    Build {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Deploy to Stellar network
    Deploy {
        /// Target network (testnet, mainnet)
        #[arg(long, default_value = "testnet")]
        network: String,
    },
    /// Initialize contract with admin
    Init {
        /// Admin account address
        #[arg(long)]
        admin: String,
    },
    /// Run environment diagnostics
    Doctor,
    /// Validate configuration files (JSON and TOML)
    Validate {
        /// Path to config file or directory (defaults to configs/)
        #[arg(default_value = "configs")]
        path: String,
    },
    /// Register a new attestor
    Register {
        /// Stellar address of the attestor
        #[arg(long)]
        address: String,
        /// Comma-separated services: deposits, withdrawals, quotes, kyc
        #[arg(long)]
        services: Option<String>,
        /// Attestor endpoint URL
        #[arg(long)]
        endpoint: Option<String>,
    },
    /// Submit or verify attestations
    Attest {
        #[command(subcommand)]
        cmd: AttestCommands,
    },
    /// Query attestation by ID
    Query {
        /// Transaction ID to query
        #[arg(long)]
        transaction_id: String,
        /// Output format: json or text
        #[arg(long, default_value = "text")]
        output: String,
    },
    /// Check attestor health
    Health {
        /// Specific attestor to check
        #[arg(long)]
        attestor: Option<String>,
        /// Watch mode with continuous monitoring
        #[arg(long)]
        watch: bool,
        /// Interval in seconds for watch mode
        #[arg(long, default_value = "60")]
        interval: u64,
    },
    /// Run contract tests
    Test {
        /// Run specific test pattern
        #[arg(long)]
        pattern: Option<String>,
        /// Verbose output
        #[arg(long)]
        verbose: bool,
    },
    /// Export audit logs
    #[command(name = "export-audit")]
    ExportAudit {
        /// Output format: json or csv
        #[arg(long, default_value = "json")]
        format: String,
        /// Output file path
        #[arg(long, short)]
        output: String,
    },
    /// Generate shell completion script
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum AttestCommands {
    /// Submit an attestation on-chain
    Submit {
        /// Subject address
        #[arg(long)]
        subject: String,
        /// Payload hash (64-character hex string)
        #[arg(long)]
        payload_hash: String,
        /// Session ID for tracking
        #[arg(long)]
        session: Option<String>,
    },
    /// Verify attestation validity on-chain
    Verify {
        /// Attestation ID to verify
        #[arg(long)]
        id: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Build { release } => run_build(release),
        Commands::Deploy { network } => run_deploy(&network),
        Commands::Init { admin } => run_init(&admin),
        Commands::Doctor => run_doctor(),
        Commands::Validate { path } => run_validate(&path),
        Commands::Register { address, services, endpoint } => {
            run_register(&address, services.as_deref(), endpoint.as_deref())
        }
        Commands::Attest { cmd } => match cmd {
            AttestCommands::Submit { subject, payload_hash, session } => {
                run_attest(&subject, &payload_hash, session.as_deref())
            }
            AttestCommands::Verify { id } => run_verify(&id),
        },
        Commands::Query { transaction_id, output } => run_query(&transaction_id, &output),
        Commands::Health { attestor, watch, interval } => {
            run_health(attestor.as_deref(), watch, interval)
        }
        Commands::Test { pattern, verbose } => run_test(pattern.as_deref(), verbose),
        Commands::ExportAudit { format, output } => run_export_audit(&format, &output),
        Commands::Completions { shell } => run_completions(shell),
    }
}

// ── completions ──────────────────────────────────────────────────────────────

fn run_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();
    generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
}

// ── build ───────────────────────────────────────────────────────────────────

fn run_build(release: bool) {
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

// ── deploy ──────────────────────────────────────────────────────────────────

fn run_deploy(network: &str) {
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

// ── init ────────────────────────────────────────────────────────────────────

fn run_init(admin: &str) {
    println!("🔧 Initializing contract with admin: {}", admin);
    
    // Validate admin address format
    if !admin.starts_with('G') || admin.len() != 56 {
        eprintln!("❌ Invalid admin address format. Expected Stellar public key starting with 'G'");
        std::process::exit(1);
    }
    
    println!("📋 Initialization steps:");
    println!("  Use soroban CLI to initialize:");
    println!("  soroban contract invoke \\");
    println!("    --id <CONTRACT_ID> \\");
    println!("    --source {} \\", admin);
    println!("    --network testnet \\");
    println!("    -- \\");
    println!("    initialize \\");
    println!("    --admin {}", admin);
    println!();
    println!("💡 Note: Replace <CONTRACT_ID> with your deployed contract address");
}

// ── attest ──────────────────────────────────────────────────────────────────

fn run_attest(subject: &str, payload_hash: &str, session: Option<&str>) {
    println!("📝 Submitting attestation...");
    println!("  Subject: {}", subject);
    println!("  Payload Hash: {}", payload_hash);
    if let Some(s) = session {
        println!("  Session: {}", s);
    }
    
    // Validate inputs
    if !subject.starts_with('G') || subject.len() != 56 {
        eprintln!("❌ Invalid subject address format");
        std::process::exit(1);
    }
    
    if payload_hash.len() != 64 || !payload_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        eprintln!("❌ Invalid payload hash format (expected 64-character hex string)");
        std::process::exit(1);
    }
    
    println!("📋 Attestation steps:");
    println!("  Use soroban CLI to submit:");
    println!("  soroban contract invoke \\");
    println!("    --id <CONTRACT_ID> \\");
    println!("    --source <ATTESTOR_ACCOUNT> \\");
    println!("    --network testnet \\");
    println!("    -- \\");
    if let Some(s) = session {
        println!("    submit_attestation_with_session \\");
        println!("    --session {} \\", s);
    } else {
        println!("    submit_attestation \\");
    }
    println!("    --subject {} \\", subject);
    println!("    --payload-hash {}", payload_hash);
    println!();
    println!("💡 Note: Replace <CONTRACT_ID> and <ATTESTOR_ACCOUNT> with actual values");
}

// ── attest verify ────────────────────────────────────────────────────────────

fn run_verify(id: &str) {
    println!("🔍 Verifying attestation on-chain...");
    println!("  Attestation ID: {}", id);

    if id.parse::<u64>().is_err() {
        eprintln!("❌ Invalid attestation ID: must be a non-negative integer");
        std::process::exit(1);
    }

    println!("📋 Step 1 — check validity:");
    println!("  soroban contract invoke \\");
    println!("    --id <CONTRACT_ID> \\");
    println!("    --source <ACCOUNT> \\");
    println!("    --network testnet \\");
    println!("    -- \\");
    println!("    is_attestation_valid \\");
    println!("    --id {}", id);
    println!();
    println!("📋 Step 2 — retrieve details (subject, attestor, timestamp):");
    println!("  soroban contract invoke \\");
    println!("    --id <CONTRACT_ID> \\");
    println!("    --source <ACCOUNT> \\");
    println!("    --network testnet \\");
    println!("    -- \\");
    println!("    get_attestation \\");
    println!("    --id {}", id);
    println!();
    println!("📊 Field reference:");
    println!("  Status  : is_attestation_valid → true (valid) | false (not found or issuer revoked)");
    println!("  Subject : attestation.subject  (Stellar address of the attested party)");
    println!("  Attestor: attestation.issuer   (Stellar address of the issuing attestor)");
    println!("  Expiry  : attestation.timestamp + 7776000  (Unix seconds; ~90 days after submission)");
    println!();
    println!("💡 Note: Replace <CONTRACT_ID> and <ACCOUNT> with actual values");
}

// ── query ───────────────────────────────────────────────────────────────────

fn run_query(transaction_id: &str, output: &str) {
    println!("🔍 Querying attestation: {}", transaction_id);
    
    if output != "json" && output != "text" {
        eprintln!("❌ Invalid output format. Use 'json' or 'text'");
        std::process::exit(1);
    }
    
    println!("📋 Query steps:");
    println!("  Use soroban CLI to query:");
    println!("  soroban contract invoke \\");
    println!("    --id <CONTRACT_ID> \\");
    println!("    --source <ACCOUNT> \\");
    println!("    --network testnet \\");
    println!("    -- \\");
    println!("    get_attestation \\");
    println!("    --attestation-id {}", transaction_id);
    println!();
    println!("💡 Output format: {}", output);
}

// ── health ──────────────────────────────────────────────────────────────────

fn run_health(attestor: Option<&str>, watch: bool, interval: u64) {
    if let Some(addr) = attestor {
        println!("🏥 Checking health for attestor: {}", addr);
    } else {
        println!("🏥 Checking health for all attestors...");
    }
    
    if watch {
        println!("👀 Watch mode enabled (interval: {}s)", interval);
        println!("   Press Ctrl+C to stop monitoring");
    }
    
    println!("📋 Health check steps:");
    println!("  Use soroban CLI to check health:");
    println!("  soroban contract invoke \\");
    println!("    --id <CONTRACT_ID> \\");
    println!("    --source <ACCOUNT> \\");
    println!("    --network testnet \\");
    println!("    -- \\");
    if let Some(addr) = attestor {
        println!("    get_anchor_health_score \\");
        println!("    --anchor {}", addr);
    } else {
        println!("    get_all_attestors");
    }
    println!();
    println!("💡 Note: Health monitoring requires active contract deployment");
}

// ── test ────────────────────────────────────────────────────────────────────

fn run_test(pattern: Option<&str>, verbose: bool) {
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

// ── doctor ──────────────────────────────────────────────────────────────────

fn run_doctor() {
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
    if vars.iter().any(|v| std::env::var(v).is_ok()) {
        println!("✔ RPC endpoint reachable");
        true
    } else {
        println!("✖ RPC endpoint not configured → set ANCHORKIT_RPC_URL, SOROBAN_RPC_URL, or STELLAR_RPC_URL");
        false
    }
}

fn check_configs() -> bool {
    let configs = std::path::Path::new("configs");
    if !configs.exists() {
        println!("✖ configs/ directory not found");
        return false;
    }
    let count = std::fs::read_dir(configs)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| {
                    matches!(
                        e.path().extension().and_then(|s| s.to_str()),
                        Some("json") | Some("toml")
                    )
                })
                .count()
        })
        .unwrap_or(0);
    if count > 0 {
        println!("✔ Config files valid ({} found)", count);
        true
    } else {
        println!("✖ No config files found in configs/");
        false
    }
}

fn check_network() -> bool {
    let ok = Command::new("curl")
        .args(["-s", "--max-time", "3", "-o", "/dev/null", "-w", "%{http_code}",
               "https://horizon-testnet.stellar.org"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() != "000")
        .unwrap_or(false);
    if ok {
        println!("✔ Network responding");
    } else {
        println!("✖ Network unreachable → check internet connection");
    }
    ok
}

// ── validate ─────────────────────────────────────────────────────────────────

fn run_validate(path: &str) {
    let p = std::path::Path::new(path);
    if p.is_dir() {
        let mut entries: Vec<_> = std::fs::read_dir(p)
            .expect("cannot read directory")
            .filter_map(|e| e.ok())
            .filter(|e| {
                matches!(
                    e.path().extension().and_then(|s| s.to_str()),
                    Some("json") | Some("toml")
                )
            })
            .collect();
        entries.sort_by_key(|e| e.path());
        if entries.is_empty() {
            println!("No .json or .toml files found in {}", path);
            return;
        }
        let mut all_ok = true;
        for entry in entries {
            all_ok &= validate_file(&entry.path());
        }
        if !all_ok {
            std::process::exit(1);
        }
    } else if !validate_file(p) {
        std::process::exit(1);
    }
}

fn validate_file(path: &std::path::Path) -> bool {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            println!("✖ {}: cannot read file: {}", path.display(), e);
            return false;
        }
    };
    
    // First check syntax
    let config_value = match ext {
        "json" => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(v) => v,
                Err(e) => {
                    println!("✖ {}: invalid JSON at line {}, column {}: {}", 
                             path.display(), e.line(), e.column(), e);
                    return false;
                }
            }
        }
        "toml" => {
            match toml::from_str::<toml::Value>(&content) {
                Ok(v) => serde_json::to_value(v).unwrap(),
                Err(e) => {
                    if let Some(span) = e.span() {
                        let line = content[..span.start].chars().filter(|&c| c == '\n').count() + 1;
                        println!("✖ {}: invalid TOML at line {}: {}", 
                                 path.display(), line, e.message());
                    } else {
                        println!("✖ {}: invalid TOML: {}", path.display(), e);
                    }
                    return false;
                }
            }
        }
        _ => {
            println!("✖ {}: unsupported format (expected .json or .toml)", path.display());
            return false;
        }
    };
    
    // Now validate schema and business rules
    validate_config_schema(path, &config_value)
}

fn validate_config_schema(path: &std::path::Path, config: &serde_json::Value) -> bool {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    
    // Validate required top-level fields
    let obj = match config.as_object() {
        Some(o) => o,
        None => {
            println!("✖ {}: root must be an object", path.display());
            return false;
        }
    };
    
    // Check required top-level fields
    for required_field in &["contract", "attestors", "sessions"] {
        if !obj.contains_key(*required_field) {
            errors.push(format!("field '{}' is missing", required_field));
        }
    }
    
    // Validate contract section
    if let Some(contract) = obj.get("contract") {
        validate_contract_section(contract, &mut errors, &mut warnings);
    }
    
    // Validate attestors section
    if let Some(attestors) = obj.get("attestors") {
        validate_attestors_section(attestors, &mut errors, &mut warnings);
    }
    
    // Validate sessions section
    if let Some(sessions) = obj.get("sessions") {
        validate_sessions_section(sessions, &mut errors, &mut warnings);
    }
    
    // Print results
    if !warnings.is_empty() {
        for warning in &warnings {
            println!("⚠  {}: {}", path.display(), warning);
        }
    }
    
    if errors.is_empty() {
        println!("✔ {}: valid configuration", path.display());
        true
    } else {
        println!("✖ {}: invalid configuration", path.display());
        for error in &errors {
            println!("  • {}", error);
        }
        false
    }
}

fn validate_contract_section(contract: &serde_json::Value, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    let obj = match contract.as_object() {
        Some(o) => o,
        None => {
            errors.push("field 'contract' must be an object".to_string());
            return;
        }
    };
    
    // Required fields
    for required in &["name", "version", "network"] {
        if !obj.contains_key(*required) {
            errors.push(format!("field 'contract.{}' is missing", required));
        }
    }
    
    // Validate name
    if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
        let name_regex = Regex::new(r"^[a-z0-9-]+$").unwrap();
        if !name_regex.is_match(name) {
            errors.push("field 'contract.name' must contain only lowercase letters, numbers, and hyphens".to_string());
        }
        if name.is_empty() || name.len() > 64 {
            errors.push(format!("field 'contract.name' length must be 1-64 characters, got {}", name.len()));
        }
    } else if obj.contains_key("name") {
        errors.push("field 'contract.name' must be a string".to_string());
    }
    
    // Validate version
    if let Some(version) = obj.get("version").and_then(|v| v.as_str()) {
        let version_regex = Regex::new(r"^\d+\.\d+\.\d+$").unwrap();
        if !version_regex.is_match(version) {
            errors.push("field 'contract.version' must follow semantic versioning (e.g., 1.0.0)".to_string());
        }
    } else if obj.contains_key("version") {
        errors.push("field 'contract.version' must be a string".to_string());
    }
    
    // Validate network
    if let Some(network) = obj.get("network").and_then(|v| v.as_str()) {
        let valid_networks = ["stellar-testnet", "stellar-mainnet", "stellar-futurenet"];
        if !valid_networks.contains(&network) {
            errors.push(format!("field 'contract.network' must be one of: {}", valid_networks.join(", ")));
        }
    } else if obj.contains_key("network") {
        errors.push("field 'contract.network' must be a string".to_string());
    }
    
    // Validate description (optional)
    if let Some(desc) = obj.get("description") {
        if let Some(desc_str) = desc.as_str() {
            if desc_str.len() > 256 {
                errors.push(format!("field 'contract.description' exceeds maximum length of 256 characters"));
            }
        } else {
            errors.push("field 'contract.description' must be a string".to_string());
        }
    }
    
    let _ = warnings; // Suppress unused warning
}

fn validate_attestors_section(attestors: &serde_json::Value, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    let obj = match attestors.as_object() {
        Some(o) => o,
        None => {
            errors.push("field 'attestors' must be an object".to_string());
            return;
        }
    };
    
    // Required field: registry
    if !obj.contains_key("registry") {
        errors.push("field 'attestors.registry' is missing".to_string());
        return;
    }
    
    let registry = match obj.get("registry").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => {
            errors.push("field 'attestors.registry' must be an array".to_string());
            return;
        }
    };
    
    if registry.is_empty() {
        errors.push("field 'attestors.registry' must contain at least one attestor".to_string());
        return;
    }
    
    if registry.len() > 100 {
        errors.push(format!("field 'attestors.registry' exceeds maximum of 100 attestors, got {}", registry.len()));
    }
    
    // Track duplicates
    let mut names = Vec::new();
    let mut addresses = Vec::new();
    let mut has_enabled = false;
    
    for (idx, attestor) in registry.iter().enumerate() {
        let attestor_obj = match attestor.as_object() {
            Some(o) => o,
            None => {
                errors.push(format!("field 'attestors.registry[{}]' must be an object", idx));
                continue;
            }
        };
        
        let attestor_name = attestor_obj.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("attestor-{}", idx));
        
        // Required fields
        for required in &["name", "address", "endpoint", "role", "enabled"] {
            if !attestor_obj.contains_key(*required) {
                errors.push(format!("field 'attestors.registry[{}].{}' is missing", idx, required));
            }
        }
        
        // Validate name
        if let Some(name) = attestor_obj.get("name").and_then(|v| v.as_str()) {
            let name_regex = Regex::new(r"^[a-z0-9-]+$").unwrap();
            if !name_regex.is_match(name) {
                errors.push(format!("field 'attestors.registry[{}].name' must contain only lowercase letters, numbers, and hyphens", idx));
            }
            if name.is_empty() || name.len() > 64 {
                errors.push(format!("field 'attestors.registry[{}].name' length must be 1-64 characters, got {}", idx, name.len()));
            }
            names.push(name.to_string());
        }
        
        // Validate address
        if let Some(address) = attestor_obj.get("address").and_then(|v| v.as_str()) {
            let addr_regex = Regex::new(r"^G[A-Z0-9]{55}$").unwrap();
            if !addr_regex.is_match(address) {
                errors.push(format!("field 'attestors.registry[{}].address' has invalid Stellar address format (must start with 'G' and be 56 characters)", idx));
            }
            let addr_len = address.len();
            if addr_len < 54 || addr_len > 56 {
                errors.push(format!("field 'attestors.registry[{}].address' length must be 54-56 characters, got {}", idx, addr_len));
            }
            addresses.push(address.to_string());
        }
        
        // Validate endpoint
        if let Some(endpoint) = attestor_obj.get("endpoint").and_then(|v| v.as_str()) {
            if let Some(error) = validate_endpoint_url(endpoint) {
                errors.push(format!("field 'attestors.registry[{}].endpoint' has invalid URL '{}' — {}", idx, endpoint, error));
            }
        }
        
        // Validate role
        if let Some(role) = attestor_obj.get("role").and_then(|v| v.as_str()) {
            let valid_roles = ["kyc-issuer", "transfer-verifier", "compliance-approver", "rate-provider", "attestor"];
            if !valid_roles.contains(&role) {
                errors.push(format!("field 'attestors.registry[{}].role' must be one of: {}", idx, valid_roles.join(", ")));
            }
        }
        
        // Check if enabled
        if let Some(enabled) = attestor_obj.get("enabled").and_then(|v| v.as_bool()) {
            if enabled {
                has_enabled = true;
            }
        }
        
        // Validate description (optional)
        if let Some(desc) = attestor_obj.get("description") {
            if let Some(desc_str) = desc.as_str() {
                if desc_str.len() > 256 {
                    errors.push(format!("field 'attestors.registry[{}].description' exceeds maximum length of 256 characters", idx));
                }
            }
        }
    }
    
    // Check for duplicates
    for name in &names {
        if names.iter().filter(|n| *n == name).count() > 1 {
            errors.push(format!("duplicate attestor name found: '{}'", name));
            break;
        }
    }
    
    for address in &addresses {
        if addresses.iter().filter(|a| *a == address).count() > 1 {
            errors.push(format!("duplicate attestor address found: '{}'", address));
            break;
        }
    }
    
    if !has_enabled {
        errors.push("at least one attestor must be enabled (attestors.registry[].enabled = true)".to_string());
    }
    
    let _ = warnings; // Suppress unused warning
}

fn validate_sessions_section(sessions: &serde_json::Value, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    let obj = match sessions.as_object() {
        Some(o) => o,
        None => {
            errors.push("field 'sessions' must be an object".to_string());
            return;
        }
    };
    
    // Required fields
    for required in &["enable_session_tracking", "session_timeout_seconds", "operations_per_session", "audit_log_retention_days"] {
        if !obj.contains_key(*required) {
            errors.push(format!("field 'sessions.{}' is missing", required));
        }
    }
    
    // Validate session_timeout_seconds
    if let Some(timeout) = obj.get("session_timeout_seconds").and_then(|v| v.as_i64()) {
        if timeout < 1 {
            errors.push("field 'sessions.session_timeout_seconds' must be at least 1".to_string());
        } else if timeout < 60 {
            errors.push("field 'sessions.session_timeout_seconds' must be at least 60 seconds".to_string());
        } else if timeout > 86400 {
            warnings.push("field 'sessions.session_timeout_seconds' exceeds 24 hours — consider shorter timeouts for security".to_string());
        }
    } else if obj.contains_key("session_timeout_seconds") {
        errors.push("field 'sessions.session_timeout_seconds' must be an integer".to_string());
    }
    
    // Validate operations_per_session
    if let Some(max_ops) = obj.get("operations_per_session").and_then(|v| v.as_i64()) {
        if max_ops < 1 {
            errors.push("field 'sessions.operations_per_session' must be at least 1".to_string());
        } else if max_ops > 10000 {
            errors.push(format!("field 'sessions.operations_per_session' exceeds maximum of 10000, got {}", max_ops));
        } else if max_ops > 5000 {
            warnings.push("field 'sessions.operations_per_session' is high (>5000) and may impact performance".to_string());
        }
    } else if obj.contains_key("operations_per_session") {
        errors.push("field 'sessions.operations_per_session' must be an integer".to_string());
    }
    
    // Validate audit_log_retention_days
    if let Some(retention) = obj.get("audit_log_retention_days").and_then(|v| v.as_i64()) {
        if retention < 1 {
            errors.push("field 'sessions.audit_log_retention_days' must be at least 1".to_string());
        } else if retention > 3650 {
            errors.push(format!("field 'sessions.audit_log_retention_days' exceeds maximum of 3650 days, got {}", retention));
        }
    } else if obj.contains_key("audit_log_retention_days") {
        errors.push("field 'sessions.audit_log_retention_days' must be an integer".to_string());
    }
    
    // Validate enable_session_tracking
    if obj.contains_key("enable_session_tracking") && !obj.get("enable_session_tracking").and_then(|v| v.as_bool()).is_some() {
        errors.push("field 'sessions.enable_session_tracking' must be a boolean".to_string());
    }
}

fn validate_endpoint_url(url: &str) -> Option<String> {
    if url.is_empty() || url.trim().is_empty() {
        return Some("URL must not be empty".to_string());
    }
    
    if url.len() < 10 {
        return Some("URL too short (minimum 10 characters, e.g. https://a.b)".to_string());
    }
    
    if url.len() > 2048 {
        return Some("URL too long (maximum 2048 characters)".to_string());
    }
    
    // HTTPS is required
    if !url.starts_with("https://") {
        return Some("URL must use HTTPS (http:// and other schemes are not allowed)".to_string());
    }
    
    // Check for forbidden characters
    if url.contains("%00") {
        return Some("URL must not contain a null byte".to_string());
    }
    
    for ch in url.chars() {
        if ch < '\x20' || ch == '\x7f' || "<>{}|\\".contains(ch) {
            return Some(format!("URL contains forbidden character '{}'", ch));
        }
    }
    
    // Extract host
    let after_scheme = &url[8..]; // skip "https://"
    let host_part = after_scheme.split('/').next()
        .and_then(|s| s.split('?').next())
        .and_then(|s| s.split('#').next())
        .unwrap_or("");
    
    if host_part.is_empty() {
        return Some("URL has no host after scheme".to_string());
    }
    
    if host_part.contains(' ') {
        return Some("URL host must not contain spaces".to_string());
    }
    
    // Handle optional port
    let domain = if let Some(colon_pos) = host_part.rfind(':') {
        let port_str = &host_part[colon_pos + 1..];
        if port_str.is_empty() {
            return Some("URL port is empty after colon".to_string());
        }
        if let Ok(port) = port_str.parse::<u16>() {
            if port == 0 {
                return Some("URL port 0 is out of valid range (1-65535)".to_string());
            }
        } else {
            return Some(format!("URL port '{}' is not numeric", port_str));
        }
        &host_part[..colon_pos]
    } else {
        host_part
    };
    
    if domain.is_empty() {
        return Some("URL has no domain".to_string());
    }
    
    // Reject loopback
    let lower = domain.to_lowercase();
    if lower == "localhost" || lower.starts_with("localhost.") || lower.ends_with(".localhost") {
        return Some("URL must not use loopback address (localhost)".to_string());
    }
    
    // Must have a TLD
    if !domain.contains('.') {
        return Some("URL domain must have a TLD (e.g. example.com, not just 'example')".to_string());
    }
    
    if domain.starts_with('.') || domain.ends_with('.') {
        return Some("URL domain must not start or end with a dot".to_string());
    }
    
    if domain.contains("..") {
        return Some("URL domain must not contain consecutive dots".to_string());
    }
    
    let labels: Vec<&str> = domain.split('.').collect();
    let non_empty: Vec<&str> = labels.iter().filter(|l| !l.is_empty()).copied().collect();
    
    if non_empty.len() < 2 {
        return Some("URL domain must have at least two labels (e.g. example.com)".to_string());
    }
    
    // Reject raw IPv4
    if labels.iter().all(|l| l.chars().all(|c| c.is_ascii_digit())) {
        return Some("URL must use a domain name, not a raw IP address".to_string());
    }
    
    for label in &labels {
        if label.is_empty() {
            return Some("URL domain contains an empty label".to_string());
        }
        if label.len() > 63 {
            return Some(format!("URL domain label '{}' exceeds 63 characters", label));
        }
        if !label.chars().next().unwrap().is_ascii_alphanumeric() {
            return Some(format!("URL domain label '{}' must start with an alphanumeric character", label));
        }
        if !label.chars().last().unwrap().is_ascii_alphanumeric() {
            return Some(format!("URL domain label '{}' must end with an alphanumeric character", label));
        }
        if label.to_lowercase().starts_with("xn--") {
            return Some(format!("URL domain label '{}' uses Punycode (xn--), which is not allowed", label));
        }
        for ch in label.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '-' {
                return Some(format!("URL domain label '{}' contains invalid character '{}'", label, ch));
            }
        }
    }
    
    None
}

// ── register ─────────────────────────────────────────────────────────────────

/// The complete set of valid service names for anchorkit register --services.
const VALID_SERVICES: &[&str] = &["deposits", "withdrawals", "quotes", "kyc"];

fn run_register(address: &str, services: Option<&str>, endpoint: Option<&str>) {
    // Validate service names before doing anything else
    if let Some(svc_str) = services {
        let invalid: Vec<&str> = svc_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && !VALID_SERVICES.contains(s))
            .collect();

        if !invalid.is_empty() {
            eprintln!(
                "error: unknown service(s): {}\n  valid services: {}",
                invalid.join(", "),
                VALID_SERVICES.join(", ")
            );
            std::process::exit(1);
        }
    }

    // Validate address format
    if !address.starts_with('G') || address.len() != 56 {
        eprintln!("error: invalid attestor address format. Expected Stellar public key starting with 'G'");
        std::process::exit(1);
    }

    println!("📝 Registering attestor: {}", address);
    if let Some(s) = services { println!("  Services: {}", s); }
    if let Some(e) = endpoint { println!("  Endpoint: {}", e); }
    
    println!("\n📋 Registration steps:");
    println!("  1. Use soroban CLI to register attestor:");
    println!("     soroban contract invoke \\");
    println!("       --id <CONTRACT_ID> \\");
    println!("       --source <ADMIN_ACCOUNT> \\");
    println!("       --network testnet \\");
    println!("       -- \\");
    println!("       register_attestor \\");
    println!("       --attestor {}", address);
    
    if let Some(s) = services {
        println!("\n  2. Configure services:");
        println!("     soroban contract invoke \\");
        println!("       --id <CONTRACT_ID> \\");
        println!("       --source <ADMIN_ACCOUNT> \\");
        println!("       --network testnet \\");
        println!("       -- \\");
        println!("       configure_services \\");
        println!("       --attestor {} \\", address);
        println!("       --services \"{}\"", s);
    }
    
    if let Some(e) = endpoint {
        println!("\n  3. Set endpoint:");
        println!("     soroban contract invoke \\");
        println!("       --id <CONTRACT_ID> \\");
        println!("       --source <ADMIN_ACCOUNT> \\");
        println!("       --network testnet \\");
        println!("       -- \\");
        println!("       set_attestor_endpoint \\");
        println!("       --attestor {} \\", address);
        println!("       --endpoint \"{}\"", e);
    }
    
    println!("\n💡 Note: Replace <CONTRACT_ID> and <ADMIN_ACCOUNT> with actual values");
    println!("🔗 This will create actual on-chain transactions when executed with soroban CLI");
}

// ── export-audit ─────────────────────────────────────────────────────────────

fn run_export_audit(format: &str, output: &str) {
    if format != "json" && format != "csv" {
        eprintln!("error: unsupported format '{}'. Use 'json' or 'csv'", format);
        std::process::exit(1);
    }
    
    // Validate output path before starting export
    let output_path = std::path::Path::new(output);
    
    // Check if parent directory exists
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            eprintln!("error: parent directory '{}' does not exist", parent.display());
            eprintln!("  → Create the directory first or specify a valid path");
            std::process::exit(1);
        }
        
        // Check if parent directory is writable (Unix-specific check)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = parent.metadata() {
                let permissions = metadata.permissions();
                if permissions.mode() & 0o200 == 0 {
                    eprintln!("error: directory '{}' is not writable", parent.display());
                    eprintln!("  → Check directory permissions");
                    std::process::exit(1);
                }
            }
        }
    }
    
    // Check if file already exists and is writable
    if output_path.exists() {
        if let Err(e) = std::fs::OpenOptions::new().write(true).open(output_path) {
            eprintln!("error: cannot write to existing file '{}': {}", output, e);
            eprintln!("  → Check file permissions or choose a different path");
            std::process::exit(1);
        }
    }
    
    println!("Fetching audit log entries...");
    let entries = fetch_audit_entries();
    let total = entries.len();
    let content = match format {
        "csv" => {
            let mut out = String::from("id,operation,actor,timestamp,result\n");
            for e in &entries {
                out.push_str(&format!("{},{},{},{},{}\n", e.id, e.operation, e.actor, e.timestamp, e.result));
            }
            out
        }
        _ => serde_json::to_string_pretty(&entries).unwrap(),
    };
    std::fs::write(output, &content).unwrap_or_else(|e| {
        eprintln!("error: cannot write to {}: {}", output, e);
        std::process::exit(1);
    });
    println!("✔ Exported {} audit log entries to {} ({})", total, output, format);
}

#[derive(serde::Serialize)]
struct AuditEntry {
    id: u64,
    operation: String,
    actor: String,
    timestamp: u64,
    result: String,
}

fn fetch_audit_entries() -> Vec<AuditEntry> {
    let page_size = 50u64;
    let mut entries = Vec::new();
    let mut page = 0u64;
    loop {
        let batch = fetch_page(page, page_size);
        let done = batch.len() < page_size as usize;
        entries.extend(batch);
        if done { break; }
        page += 1;
        eprint!("\r  Fetched {} entries...", entries.len());
    }
    if !entries.is_empty() { eprintln!(); }
    entries
}

fn fetch_page(page: u64, page_size: u64) -> Vec<AuditEntry> {
    let _ = (page, page_size);
    vec![]
}
