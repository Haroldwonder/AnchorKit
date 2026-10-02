#![cfg(feature = "std")]

use clap::{Parser, Subcommand};
use std::process::Command;
use std::time::Instant;
use regex::Regex;

mod soroban_rpc;
use soroban_rpc::*;

const MIN_RUST_MAJOR: u32 = 1;
const MIN_RUST_MINOR: u32 = 74;

const VALID_NETWORKS: &[&str] = &["testnet", "mainnet", "futurenet"];

fn validate_network_arg(s: &str) -> Result<String, String> {
    if VALID_NETWORKS.contains(&s) {
        Ok(s.to_string())
    } else {
        Err(format!(
            "invalid network '{}'; expected one of: {}",
            s,
            VALID_NETWORKS.join(", ")
        ))
    }
}

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
        #[arg(long, default_value = "testnet", value_parser = validate_network_arg)]
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
        /// Output format: json, text, or table
        #[arg(long, default_value = "text")]
        output: String,
    },
    /// Submit attestation
    Attest {
        /// Subject address
        #[arg(long)]
        subject: String,
        /// Payload hash
        #[arg(long)]
        payload_hash: String,
        /// Session ID for tracking
        #[arg(long)]
        session: Option<String>,
    },
    /// Query attestation by ID
    Query {
        /// Transaction ID to query
        #[arg(long)]
        transaction_id: String,
        /// Output format: json, text, or table
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
        /// Output format: json, text, or table
        #[arg(long, default_value = "text")]
        output: String,
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
    /// Manage AnchorKit configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Fetch and display audit log entries
    #[command(name = "audit")]
    Audit {
        #[command(subcommand)]
        action: AuditAction,
    },
    /// Manage interaction sessions
    #[command(name = "session")]
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Validate configuration files using validate_config.py
    Validate {
        /// Path to a JSON config file or directory (defaults to configs/)
        #[arg(default_value = "configs")]
        path: String,
    },
    /// Interactively create a new validated AnchorKit config file
    Init,
}

#[derive(Subcommand)]
enum AuditAction {
    /// Fetch and display a single audit log entry by ID
    Get {
        /// Audit log entry ID
        #[arg(value_name = "LOG_ID")]
        log_id: u64,
    },
    /// List audit log entries for a session
    List {
        /// Session ID to filter by
        #[arg(long)]
        session: u64,
        /// Start from this log ID (defaults to 0)
        #[arg(long)]
        from: Option<u64>,
        /// End at this log ID (defaults to latest)
        #[arg(long)]
        to: Option<u64>,
        /// Output format: text (default), json, or csv
        #[arg(long, default_value = "text")]
        format: String,
        /// Pretty-print JSON (only for json format)
        #[arg(long)]
        pretty: bool,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    /// Create a new interaction session
    Create {
        /// Stellar address of the session initiator (optional, uses STELLAR_SECRET_KEY if not provided)
        #[arg(long)]
        initiator: Option<String>,
    },
    /// Retrieve session details by ID
    Get {
        /// Session ID to retrieve
        #[arg(value_name = "SESSION_ID")]
        session_id: u64,
    },
    /// List all active sessions
    List {
        /// Output format: text (default), json, or csv
        #[arg(long, default_value = "text")]
        format: String,
        /// Pretty-print JSON (only for json format)
        #[arg(long)]
        pretty: bool,
        /// Maximum number of sessions to retrieve (defaults to 100)
        #[arg(long)]
        limit: Option<u64>,
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
        Commands::Register { address, services, endpoint, output } => {
            run_register(&address, services.as_deref(), endpoint.as_deref(), &output)
        }
        Commands::Attest { subject, payload_hash, session } => {
            run_attest(&subject, &payload_hash, session.as_deref())
        }
        Commands::Query { transaction_id, output } => run_query(&transaction_id, &output),
        Commands::Health { attestor, watch, interval, output } => {
            run_health(attestor.as_deref(), watch, interval, &output)
        }
        Commands::Test { pattern, verbose } => run_test(pattern.as_deref(), verbose),
        Commands::ExportAudit { format, output } => run_export_audit(&format, &output),
        Commands::Config { command } => match command {
            ConfigCommands::Validate { path } => run_config_validate(&path),
            ConfigCommands::Init => run_config_init(),
        },
        Commands::Audit { action } => match action {
            AuditAction::Get { log_id } => run_audit_get(log_id),
            AuditAction::List { session, from, to, format, pretty } => {
                run_audit_list(session, from, to, &format, pretty)
            }
        },
        Commands::Session { action } => match action {
            SessionAction::Create { initiator } => run_session_create(initiator),
            SessionAction::Get { session_id } => run_session_get(session_id),
            SessionAction::List { format, pretty, limit } => {
                run_session_list(&format, pretty, limit)
            }
        },
    }
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
    
    // Validate admin address format (prefix, length, and base32 strkey charset)
    if !is_valid_g_address(admin) {
        eprintln!("❌ Invalid admin address format. Expected Stellar public key starting with 'G' followed by 55 base32 characters (A-Z2-7)");
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
    if !is_valid_g_address(subject) {
        eprintln!("❌ Invalid subject address format. Expected Stellar public key starting with 'G' followed by 55 base32 characters (A-Z2-7)");
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

// ── ascii table renderer ─────────────────────────────────────────────────────

/// Renders a simple ASCII table from headers and rows.
///
/// Each inner `Vec<String>` is one row; the number of elements must match
/// `headers.len()`.  Column widths are computed automatically.
fn print_ascii_table(headers: &[&str], rows: &[Vec<String>]) {
    // Compute column widths: max of header width and all cell widths.
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let separator = {
        let parts: Vec<String> = widths.iter().map(|w| "-".repeat(w + 2)).collect();
        format!("+{}+", parts.join("+"))
    };

    // Header row
    println!("{}", separator);
    let header_row: Vec<String> = headers
        .iter()
        .zip(widths.iter())
        .map(|(h, w)| format!(" {:w$} ", h, w = w))
        .collect();
    println!("|{}|", header_row.join("|"));
    println!("{}", separator);

    // Data rows
    for row in rows {
        let cells: Vec<String> = row
            .iter()
            .zip(widths.iter())
            .map(|(c, w)| format!(" {:w$} ", c, w = w))
            .collect();
        println!("|{}|", cells.join("|"));
    }
    println!("{}", separator);
}

// ── output format validation ──────────────────────────────────────────────────

/// Returns `true` if the format string is one of the three accepted values.
/// Prints an error and exits on invalid input.
fn validate_output_format(output: &str) {
    if !matches!(output, "json" | "text" | "table") {
        eprintln!("❌ Invalid output format '{}'. Use 'json', 'text', or 'table'", output);
        std::process::exit(1);
    }
}

// ── query ───────────────────────────────────────────────────────────────────

fn run_query(transaction_id: &str, output: &str) {
    validate_output_format(output);

    // Simulated attestation record (in a real implementation this would come
    // from an on-chain query via the soroban RPC).
    let record = [
        ("Transaction ID", transaction_id),
        ("Status",         "pending_external"),
        ("Subject",        "G…<subject>"),
        ("Issuer",         "G…<issuer>"),
        ("Payload Hash",   "abc123…"),
        ("Timestamp",      "2024-01-15T10:30:00Z"),
    ];

    match output {
        "json" => {
            let obj = serde_json::json!({
                "transaction_id": transaction_id,
                "status": "pending_external",
                "subject": "G…<subject>",
                "issuer": "G…<issuer>",
                "payload_hash": "abc123…",
                "timestamp": "2024-01-15T10:30:00Z",
                "note": "Replace placeholders with real soroban RPC query results"
            });
            println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        }
        "table" => {
            println!("🔍 Attestation query: {}\n", transaction_id);
            let headers = &["Field", "Value"];
            let rows: Vec<Vec<String>> = record
                .iter()
                .map(|(k, v)| vec![k.to_string(), v.to_string()])
                .collect();
            print_ascii_table(headers, &rows);
            println!("\n💡 Fetch live data:");
            println!(
                "   soroban contract invoke --id <CONTRACT_ID> --source <ACCOUNT> \
                 --network testnet -- get_attestation --attestation-id {}",
                transaction_id
            );
        }
        _ => {
            // "text" (default)
            println!("🔍 Querying attestation: {}\n", transaction_id);
            for (k, v) in &record {
                println!("  {:<16} {}", format!("{}:", k), v);
            }
            println!("\n📋 Fetch live data with soroban CLI:");
            println!("  soroban contract invoke \\");
            println!("    --id <CONTRACT_ID> \\");
            println!("    --source <ACCOUNT> \\");
            println!("    --network testnet \\");
            println!("    -- \\");
            println!("    get_attestation \\");
            println!("    --attestation-id {}", transaction_id);
        }
    }
}

// ── health ──────────────────────────────────────────────────────────────────

fn run_health(attestor: Option<&str>, watch: bool, interval: u64, output: &str) {
    validate_output_format(output);

    if watch {
        if interval == 0 {
            eprintln!("❌ Invalid interval '0'. --interval must be greater than 0 seconds");
            std::process::exit(1);
        }
        eprintln!("👀 Watch mode enabled (interval: {}s) — press Ctrl+C to stop", interval);
        loop {
            render_health(attestor, output);
            std::thread::sleep(std::time::Duration::from_secs(interval));
        }
    }

    render_health(attestor, output);
}

fn render_health(attestor: Option<&str>, output: &str) {
    // Simulated health records.  In a real implementation these come from
    // on-chain queries; the shape maps 1-to-1 with HealthStatus in types.rs.
    let records: Vec<[&str; 4]> = if let Some(addr) = attestor {
        vec![[addr, "18", "0", "99"]]
    } else {
        vec![
            ["G…attestor-1", "18",  "0", "99"],
            ["G…attestor-2", "42",  "1", "97"],
            ["G…attestor-3", "120", "3", "92"],
        ]
    };

    match output {
        "json" => {
            let json_records: Vec<serde_json::Value> = records
                .iter()
                .map(|r| serde_json::json!({
                    "anchor":               r[0],
                    "latency_ms":           r[1].parse::<u64>().unwrap_or(0),
                    "failure_count":        r[2].parse::<u32>().unwrap_or(0),
                    "availability_percent": r[3].parse::<u32>().unwrap_or(0),
                }))
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_records).unwrap());
        }
        "table" => {
            println!("🏥 Health status\n");
            let headers = &["Anchor", "Latency (ms)", "Failures", "Availability (%)"];
            let rows: Vec<Vec<String>> = records
                .iter()
                .map(|r| r.iter().map(|c| c.to_string()).collect())
                .collect();
            print_ascii_table(headers, &rows);
            println!("\n💡 Fetch live data:");
            if let Some(addr) = attestor {
                println!(
                    "   soroban contract invoke --id <CONTRACT_ID> --source <ACCOUNT> \
                     --network testnet -- get_anchor_health_score --anchor {}",
                    addr
                );
            } else {
                println!(
                    "   soroban contract invoke --id <CONTRACT_ID> --source <ACCOUNT> \
                     --network testnet -- get_all_attestors"
                );
            }
        }
        _ => {
            // "text" (default)
            if let Some(addr) = attestor {
                println!("🏥 Health for attestor: {}", addr);
            } else {
                println!("🏥 Health for all attestors:");
            }
            for r in &records {
                println!(
                    "  anchor={} latency={}ms failures={} availability={}%",
                    r[0], r[1], r[2], r[3]
                );
            }
            println!("\n📋 Fetch live data:");
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
            println!("\n💡 Note: Health monitoring requires active contract deployment");
        }
    }
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

// ── validate ─────────────────────────────────────────────────────────────────

fn run_validate(path: &str) {
    let p = std::path::Path::new(path);
    if p.is_dir() {
        let read_dir = match std::fs::read_dir(p) {
            Ok(rd) => rd,
            Err(e) => {
                eprintln!("error: cannot read directory '{}': {}", path, e);
                std::process::exit(1);
            }
        };
        let mut entries: Vec<_> = read_dir
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

    // Validate deployment section (required as of config init v2)
    if !obj.contains_key("deployment") {
        errors.push("field 'deployment' is missing — run `anchorkit config init` to generate a valid config".to_string());
    } else if let Some(deployment) = obj.get("deployment") {
        validate_deployment_section(deployment, &mut errors, &mut warnings);
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
            let addr_regex = Regex::new(r"^G[A-Z2-7]{55}$").unwrap();
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

fn validate_deployment_section(deployment: &serde_json::Value, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    let obj = match deployment.as_object() {
        Some(o) => o,
        None => {
            errors.push("field 'deployment' must be an object".to_string());
            return;
        }
    };

    // admin_key — required, must look like a Stellar public key
    match obj.get("admin_key").and_then(|v| v.as_str()) {
        Some(k) if k.starts_with('G') && k.len() == 56 => {}
        Some(k) => errors.push(format!(
            "field 'deployment.admin_key' must be a 56-char Stellar public key starting with 'G', got '{}'", k
        )),
        None => errors.push("field 'deployment.admin_key' is required".to_string()),
    }

    // rpc_endpoint — required, must be HTTPS
    match obj.get("rpc_endpoint").and_then(|v| v.as_str()) {
        Some(url) => {
            if let Some(err) = validate_endpoint_url(url) {
                errors.push(format!("field 'deployment.rpc_endpoint': {}", err));
            }
        }
        None => errors.push("field 'deployment.rpc_endpoint' is required".to_string()),
    }

    // network — required
    if !obj.contains_key("network") || obj.get("network").and_then(|v| v.as_str()).is_none() {
        errors.push("field 'deployment.network' is required".to_string());
    }

    // contract_id — optional, but if present must look like a Soroban contract address
    if let Some(id) = obj.get("contract_id").and_then(|v| v.as_str()) {
        if !(id.starts_with('C') && id.len() == 56) {
            warnings.push(format!(
                "field 'deployment.contract_id' should be a 56-char Soroban contract address starting with 'C', got '{}'", id
            ));
        }
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

// ── config ────────────────────────────────────────────────────────────────────

fn run_config_validate(path: &str) {
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

// ── register ─────────────────────────────────────────────────────────────────

/// The complete set of valid service names for anchorkit register --services.
const VALID_SERVICES: &[&str] = &["deposits", "withdrawals", "quotes", "kyc"];

fn run_register(address: &str, services: Option<&str>, endpoint: Option<&str>, output: &str) {
    validate_output_format(output);

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

    // Validate address format (prefix, length, and base32 strkey charset)
    if !is_valid_g_address(address) {
        eprintln!("error: invalid attestor address format. Expected Stellar public key starting with 'G' followed by 55 base32 characters (A-Z2-7)");
        std::process::exit(1);
    }

    let services_str = services.unwrap_or("(none)");
    let endpoint_str = endpoint.unwrap_or("(none)");
    let status = "pending on-chain confirmation";

    match output {
        "json" => {
            let obj = serde_json::json!({
                "address":  address,
                "services": services_str,
                "endpoint": endpoint_str,
                "status":   status,
            });
            println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        }
        "table" => {
            println!("📝 Attestor registration\n");
            let headers = &["Field", "Value"];
            let rows = vec![
                vec!["Address".to_string(),  address.to_string()],
                vec!["Services".to_string(), services_str.to_string()],
                vec!["Endpoint".to_string(), endpoint_str.to_string()],
                vec!["Status".to_string(),   status.to_string()],
            ];
            print_ascii_table(headers, &rows);
            println!("\n💡 Execute with soroban CLI:");
            println!(
                "   soroban contract invoke --id <CONTRACT_ID> --source <ADMIN_ACCOUNT> \
                 --network testnet -- register_attestor --attestor {}",
                address
            );
        }
        _ => {
            // "text" (default) — preserve original behaviour
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
    }
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

    if total == 0 {
        eprintln!("error: no audit log entries found — verify ANCHORKIT_CONTRACT_ID and ANCHORKIT_RPC_URL are set correctly");
        std::process::exit(1);
    }

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
    let rpc_url = std::env::var("ANCHORKIT_RPC_URL")
        .or_else(|_| std::env::var("SOROBAN_RPC_URL"))
        .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string());
    let contract_id = std::env::var("ANCHORKIT_CONTRACT_ID").unwrap_or_default();

    if contract_id.is_empty() {
        eprintln!("warning: ANCHORKIT_CONTRACT_ID not set; cannot fetch audit entries from chain");
        return vec![];
    }

    let offset = page * page_size;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getEvents",
        "params": {
            "startLedger": 0,
            "filters": [{"type": "contract", "contractIds": [contract_id], "topics": [["audit_log"]]}],
            "pagination": {"limit": page_size, "offset": offset}
        }
    });

    let client = reqwest::blocking::Client::new();
    let resp = match client.post(&rpc_url).json(&body).send() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("warning: RPC request failed: {e}");
            return vec![];
        }
    };

    let json: serde_json::Value = match resp.json() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("warning: failed to parse RPC response: {e}");
            return vec![];
        }
    };

    let events = match json["result"]["events"].as_array() {
        Some(arr) => arr.clone(),
        None => return vec![],
    };

    events
        .iter()
        .enumerate()
        .filter_map(|(i, ev)| {
            Some(AuditEntry {
                id: offset + i as u64,
                operation: ev["topic"].as_str().unwrap_or("unknown").to_string(),
                actor: ev["value"]["address"].as_str().unwrap_or("unknown").to_string(),
                timestamp: ev["ledgerClosedAt"]
                    .as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0),
                result: ev["value"]["result"].as_str().unwrap_or("ok").to_string(),
            })
        })
        .collect()
}

// ── audit get ───────────────────────────────────────────────────────────────

fn run_audit_get(log_id: u64) {
    println!("◈ Fetching audit log entry {}", log_id);
    println!();

    match fetch_audit_log_entry(log_id) {
        Some(entry) => {
            print_audit_entry(&entry);
            println!();
            println!("✔ Entry retrieved successfully");
        }
        None => {
            eprintln!("✖ Audit log entry {} not found", log_id);
            std::process::exit(1);
        }
    }
}

// ── audit list ──────────────────────────────────────────────────────────────

fn run_audit_list(session: u64, from: Option<u64>, to: Option<u64>, format: &str, pretty: bool) {
    println!("◈ Fetching audit logs for session {}", session);
    
    // Validate format
    if !["text", "json", "csv"].contains(&format) {
        eprintln!("error: unsupported format '{}'. Use 'text', 'json', or 'csv'", format);
        std::process::exit(1);
    }

    let entries = fetch_audit_logs_by_session(session, from, to);
    
    if entries.is_empty() {
        println!("\n✗ No audit log entries found for session {}", session);
        return;
    }

    println!();
    
    match format {
        "text" => {
            println!("┌─ Audit Log Entries (Session {}) ──────────────────────────────", session);
            for (idx, entry) in entries.iter().enumerate() {
                if idx > 0 {
                    println!("├───────────────────────────────────────────────────────────");
                }
                print_audit_entry_compact(entry);
            }
            println!("└───────────────────────────────────────────────────────────────");
            println!();
            println!("✔ Retrieved {} audit log entr{}", 
                     entries.len(), 
                     if entries.len() == 1 { "y" } else { "ies" });
        }
        "json" => {
            let json_output = if pretty {
                serde_json::to_string_pretty(&entries)
                    .unwrap_or_else(|_| "[]".to_string())
            } else {
                serde_json::to_string(&entries)
                    .unwrap_or_else(|_| "[]".to_string())
            };
            println!("{}", json_output);
        }
        "csv" => {
            println!("log_id,session_id,actor,operation_type,operation_index,timestamp,status,result");
            for entry in &entries {
                println!(
                    "{},{},{},{},{},{},{},\"{}\"",
                    entry.log_id,
                    entry.session_id,
                    entry.actor,
                    entry.operation_type,
                    entry.operation_index,
                    entry.timestamp,
                    entry.status,
                    entry.result.replace("\"", "\\\"")
                );
            }
        }
        _ => unreachable!(),
    }
}

// ── Data structures for audit logging ─────────────────────────────────────────

#[derive(serde::Serialize, Clone)]
struct AuditLogEntry {
    log_id: u64,
    session_id: u64,
    actor: String,
    operation_type: String,
    operation_index: u64,
    timestamp: u64,
    status: String,
    result: String,
}

// ── Pretty-printing helpers ──────────────────────────────────────────────────

fn print_audit_entry(entry: &AuditLogEntry) {
    println!("  Log ID:          {}", entry.log_id);
    println!("  Session ID:      {}", entry.session_id);
    println!("  Actor:           {}", entry.actor);
    println!("  Operation:       {} (index: {})", entry.operation_type, entry.operation_index);
    println!("  Timestamp:       {} ({})", 
             entry.timestamp, 
             format_timestamp(entry.timestamp));
    println!("  Status:          {}", entry.status);
    println!("  Result:          {}", entry.result);
}

fn print_audit_entry_compact(entry: &AuditLogEntry) {
    let actor_truncated: String = entry.actor.chars().take(24).collect();
    let result_truncated: String = entry.result.chars().take(50).collect();
    println!("  │ Log ID:     {} │ Op: {} │ Status: {}", 
             entry.log_id,
             entry.operation_type,
             entry.status);
    println!("  │ Session:    {} │ Actor: {}", 
             entry.session_id,
             actor_truncated);
    println!("  │ Timestamp:  {} (op_index: {})", 
             format_timestamp(entry.timestamp),
             entry.operation_index);
    println!("  │ Result:     {}", 
             result_truncated);
}

fn format_timestamp(ts: u64) -> String {
    // ts is Unix timestamp in seconds
    let secs_per_day = 86400;
    let secs_per_hour = 3600;
    let secs_per_minute = 60;
    
    let days = ts / secs_per_day;
    let hours = (ts % secs_per_day) / secs_per_hour;
    let minutes = (ts % secs_per_hour) / secs_per_minute;
    let seconds = ts % secs_per_minute;
    
    format!("{}d {:02}h {:02}m {:02}s", days, hours, minutes, seconds)
}

// ── On-chain data fetching ───────────────────────────────────────────────────

/// Fetch a single audit log entry from on-chain storage.
/// In production, this would query the Soroban contract via RPC.
/// For now, this is a placeholder that would connect to the on-chain contract.
fn fetch_audit_log_entry(log_id: u64) -> Option<AuditLogEntry> {
    // TODO: Implement actual on-chain fetching via Soroban RPC
    // This would invoke contract method: get_audit_log(log_id)
    // and parse the returned AuditLog structure.
    //
    // Example RPC call structure:
    //   POST {RPC_URL}
    //   {
    //     "jsonrpc": "2.0",
    //     "id": 1,
    //     "method": "simulateTransaction",
    //     "params": {
    //       "transaction": "{encoded_contract_invoke}",
    //       "resourceLeeway": 15
    //     }
    //   }
    //
    // Implementation strategy:
    // 1. Get RPC URL from ANCHORKIT_RPC_URL or SOROBAN_RPC_URL env var
    // 2. Build contract invocation for get_audit_log(log_id)
    // 3. Sign transaction with wallet from STELLAR_SECRET_KEY or Soroban identity
    // 4. Submit via /simulateTransaction endpoint
    // 5. Parse result and extract AuditLog structure
    // 6. Map to AuditLogEntry with actor address and operation context
    
    let rpc_url = std::env::var("ANCHORKIT_RPC_URL")
        .or_else(|_| std::env::var("SOROBAN_RPC_URL"))
        .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string());
    let contract_id = std::env::var("ANCHORKIT_CONTRACT_ID").unwrap_or_default();

    if contract_id.is_empty() {
        eprintln!("warning: ANCHORKIT_CONTRACT_ID not set; cannot fetch audit entry from chain");
        return None;
    }

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getEvents",
        "params": {
            "startLedger": 0,
            "filters": [{"type": "contract", "contractIds": [contract_id], "topics": [["audit_log"]]}],
            "pagination": {"limit": 1, "offset": log_id}
        }
    });

    let client = reqwest::blocking::Client::new();
    let resp = match client.post(&rpc_url).json(&body).send() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("warning: RPC request failed: {e}");
            return None;
        }
    };

    let json: serde_json::Value = match resp.json() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("warning: failed to parse RPC response: {e}");
            return None;
        }
    };

    let ev = json["result"]["events"].as_array()?.first()?.clone();
    Some(AuditLogEntry {
        log_id,
        operation_type: ev["topic"].as_str().unwrap_or("unknown").to_string(),
        operation_index: ev["value"]["operation_index"].as_u64().unwrap_or(0),
        actor: ev["value"]["address"].as_str().unwrap_or("unknown").to_string(),
        timestamp: ev["ledgerClosedAt"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0),
        status: ev["value"]["status"].as_str().unwrap_or("ok").to_string(),
        result: ev["value"]["result"].as_str().unwrap_or("ok").to_string(),
        session_id: ev["value"]["session_id"].as_u64().unwrap_or(0),
    })
}

/// Fetch audit logs filtered by session ID.
/// Returns entries with optional range filtering [from_id, to_id].
fn fetch_audit_logs_by_session(session_id: u64, from: Option<u64>, to: Option<u64>) -> Vec<AuditLogEntry> {
    // TODO: Implement actual on-chain fetching via Soroban RPC
    // This would invoke contract method: get_audit_log_range(from_id, to_id)
    // and filter results where session_id matches.
    //
    // The contract's get_audit_log_range() is capped at 100 entries per call,
    // so pagination may be needed for large result sets.
    //
    // Implementation strategy:
    // 1. Determine range: if from/to not provided, fetch latest 100 entries
    // 2. Loop: call get_audit_log_range(from, min(from+100, to))
    // 3. Filter: keep only entries where entry.session_id == session_id
    // 4. Continue: if result set full, fetch next batch
    // 5. Accumulate until complete or limit reached
    // 6. Map AuditLog results to AuditLogEntry with formatted timestamps
    
    let rpc_url = std::env::var("ANCHORKIT_RPC_URL")
        .or_else(|_| std::env::var("SOROBAN_RPC_URL"))
        .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string());
    let contract_id = std::env::var("ANCHORKIT_CONTRACT_ID").unwrap_or_default();

    if contract_id.is_empty() {
        eprintln!("warning: ANCHORKIT_CONTRACT_ID not set; cannot fetch session audit logs from chain");
        return vec![];
    }

    let batch_size = 100u64;
    let range_start = from.unwrap_or(0);
    let range_end = to.unwrap_or(u64::MAX);
    let mut results = Vec::new();
    let mut offset = range_start;
    let client = reqwest::blocking::Client::new();

    loop {
        let limit = batch_size.min(range_end.saturating_sub(offset) + 1);
        if limit == 0 { break; }

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getEvents",
            "params": {
                "startLedger": 0,
                "filters": [{"type": "contract", "contractIds": [contract_id], "topics": [["audit_log"]]}],
                "pagination": {"limit": limit, "offset": offset}
            }
        });

        let resp = match client.post(&rpc_url).json(&body).send() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warning: RPC request failed: {e}");
                break;
            }
        };

        let json: serde_json::Value = match resp.json() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("warning: failed to parse RPC response: {e}");
                break;
            }
        };

        let events = match json["result"]["events"].as_array() {
            Some(arr) => arr.clone(),
            None => break,
        };

        let count = events.len() as u64;
        for (i, ev) in events.iter().enumerate() {
            let entry_session = ev["value"]["session_id"].as_u64().unwrap_or(u64::MAX);
            if entry_session != session_id {
                continue;
            }
            results.push(AuditLogEntry {
                log_id: offset + i as u64,
                operation_type: ev["topic"].as_str().unwrap_or("unknown").to_string(),
                operation_index: ev["value"]["operation_index"].as_u64().unwrap_or(0),
                actor: ev["value"]["address"].as_str().unwrap_or("unknown").to_string(),
                timestamp: ev["ledgerClosedAt"]
                    .as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0),
                status: ev["value"]["status"].as_str().unwrap_or("ok").to_string(),
                result: ev["value"]["result"].as_str().unwrap_or("ok").to_string(),
                session_id,
            });
        }

        if count < limit { break; }
        offset += count;
        if offset > range_end { break; }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_event_uses_real_onchain_log_id_not_pagination_offset() {
        let event = serde_json::json!({
            "value": {
                "log_id": 42,
                "session_id": 7,
                "operation_index": 3,
                "address": "GTEST",
                "status": "success",
                "result": "ok",
                "operation_type": "attest"
            },
            "topic": "audit_log",
            "ledgerClosedAt": "1710000000"
        });

        let entry = parse_audit_log_entry_from_event(&event, 999, 7);

        assert_eq!(entry.log_id, 42);
        assert_eq!(entry.session_id, 7);
        assert_eq!(entry.operation_index, 3);
        assert_eq!(entry.actor, "GTEST");
    }
}

// ── Session data structures ──────────────────────────────────────────────────

/// CLI representation of a session
#[derive(serde::Serialize, serde::Deserialize)]
struct SessionRecord {
    session_id: u64,
    initiator: String,
    created_at: u64,
    operation_count: u64,
    expires_at: u64,
}

// ── Session management commands ──────────────────────────────────────────────

fn run_session_create(initiator: Option<String>) {
    println!("◈ Creating new session");
    println!();

    match initiator {
        Some(addr) => {
            println!("  Initiator: {}", addr);
        }
        None => {
            println!("  Initiator: using STELLAR_SECRET_KEY");
        }
    }

    match create_session_on_chain(initiator) {
        Ok(session_id) => {
            println!();
            println!("✔ Session created successfully");
            println!("  Session ID: {}", session_id);
            println!();
            println!("Use this session ID for subsequent operations:");
            println!("  anchorkit session get {}", session_id);
        }
        Err(e) => {
            eprintln!("✖ Failed to create session: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_session_get(session_id: u64) {
    println!("◈ Fetching session {}", session_id);
    println!();

    match fetch_session_from_chain(session_id) {
        Ok(Some(session)) => {
            print_session(&session);
            println!();
            println!("✔ Session retrieved successfully");
        }
        Ok(None) => {
            eprintln!("✖ Session {} not found", session_id);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("✖ Failed to fetch session {}: {}", session_id, e);
            std::process::exit(1);
        }
    }
}

fn run_session_list(format: &str, pretty: bool, limit: Option<u64>) {
    let limit = limit.unwrap_or(100);
    println!("◈ Fetching sessions (limit: {})", limit);

    // Validate format
    if !["text", "json", "csv"].contains(&format) {
        eprintln!("error: unsupported format '{}'. Use 'text', 'json', or 'csv'", format);
        std::process::exit(1);
    }

    let sessions = match fetch_sessions_from_chain(limit) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("✖ Failed to fetch sessions: {}", e);
            std::process::exit(1);
        }
    };

    if sessions.is_empty() {
        println!("\n✗ No sessions found");
        return;
    }

    println!();

    match format {
        "text" => {
            println!("┌─ Active Sessions ──────────────────────────────────────────────");
            for (idx, session) in sessions.iter().enumerate() {
                if idx > 0 {
                    println!("├───────────────────────────────────────────────────────────────");
                }
                print_session_compact(&session);
            }
            println!("└───────────────────────────────────────────────────────────────────");
            println!();
            println!("✔ Retrieved {} session{}", 
                     sessions.len(), 
                     if sessions.len() == 1 { "" } else { "s" });
        }
        "json" => {
            let json_output = if pretty {
                serde_json::to_string_pretty(&sessions)
                    .unwrap_or_else(|_| "[]".to_string())
            } else {
                serde_json::to_string(&sessions)
                    .unwrap_or_else(|_| "[]".to_string())
            };
            println!("{}", json_output);
        }
        "csv" => {
            println!("session_id,initiator,created_at,operation_count,expires_at");
            for session in &sessions {
                println!(
                    "{},{},{},{},{}",
                    session.session_id,
                    session.initiator,
                    session.created_at,
                    session.operation_count,
                    session.expires_at,
                );
            }
        }
        _ => unreachable!(),
    }
}

// ── Session helper functions ─────────────────────────────────────────────────

fn print_session(session: &SessionRecord) {
    println!("  Session ID:       {}", session.session_id);
    println!("  Initiator:        {}", session.initiator);
    println!("  Created At:       {} ({})", 
             session.created_at, 
             format_timestamp(session.created_at));
    println!("  Operation Count:  {}", session.operation_count);
    println!("  Expires At:       {} ({})", 
             session.expires_at,
             format_timestamp(session.expires_at));
}

fn print_session_compact(session: &SessionRecord) {
    println!("│ Session: {}  Initiator: {}...", 
             session.session_id,
             &session.initiator.chars().take(16).collect::<String>());
    println!("│ Created: {}  Operations: {}  Expires: {}", 
             session.created_at,
             session.operation_count,
             session.expires_at);
}

fn create_session_on_chain(initiator: Option<String>) -> Result<u64, String> {
    create_session_rpc(initiator)
}

fn fetch_session_from_chain(session_id: u64) -> Result<Option<SessionRecord>, String> {
    match get_session_rpc(session_id) {
        Ok(session_data) => Ok(Some(SessionRecord {
            session_id: session_data.session_id,
            initiator: session_data.initiator,
            created_at: session_data.created_at,
            nonce: session_data.nonce,
            operation_count: session_data.operation_count,
            expires_at: session_data.expires_at,
        })),
        // #914: surface the descriptive RPC error instead of silently returning None
        Err(e) => Err(e),
    }
}

fn fetch_sessions_from_chain(limit: u64) -> Result<Vec<SessionRecord>, String> {
    match list_sessions_rpc(limit) {
        Ok(sessions) => Ok(sessions
            .into_iter()
            .map(|s| SessionRecord {
                session_id: s.session_id,
                initiator: s.initiator,
                created_at: s.created_at,
                nonce: s.nonce,
                operation_count: s.operation_count,
                expires_at: s.expires_at,
            })
            .collect()),
        // #914: surface the descriptive RPC error instead of silently returning vec![]
        Err(e) => Err(e),
    }
}

// ── config init ───────────────────────────────────────────────────────────────

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

/// Returns `true` if `addr` is a syntactically valid Stellar public key:
/// prefix 'G', exactly 56 characters, remaining 55 chars from the RFC4648
/// base32 strkey alphabet (A-Z, 2-7).  Digits 0, 1, 8, 9 and any lowercase
/// letter are not part of that alphabet and will cause this to return `false`.
fn is_valid_g_address(addr: &str) -> bool {
    addr.starts_with('G')
        && addr.len() == 56
        && addr[1..].chars().all(|c| matches!(c, 'A'..='Z' | '2'..='7'))
}

/// Validate a Stellar public key (admin / contract-id format).
fn validate_stellar_address(s: &str) -> Result<String, String> {
    // Stellar public keys are RFC4648 base32 strkeys: prefix 'G' + 55 chars from A-Z2-7
    let re = Regex::new(r"^G[A-Z2-7]{55}$").unwrap();
    if re.is_match(s) {
        Ok(s.to_string())
    } else {
        Err(format!(
            "Invalid Stellar public key — must start with 'G' followed by 55 base32 characters (A-Z, 2-7) (56 chars total), got '{}'",
            s
        ))
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

/// Validate a Soroban contract ID.
/// Contract IDs on Stellar are the same format as Stellar account addresses:
/// a 56-character strkey starting with 'C'.
fn validate_contract_id(s: &str) -> Result<String, String> {
    let re = Regex::new(r"^C[A-Z2-7]{55}$").unwrap();
    if re.is_match(s) {
        Ok(s.to_string())
    } else {
        Err(format!(
            "Invalid contract ID — must start with 'C' and be exactly 56 uppercase base-32 characters, got '{}'",
            s
        ))
    }
}

/// Derive a safe output filename from the anchor name the user will supply.
fn config_output_path(anchor_name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("configs/{}.json", anchor_name))
}

fn run_config_init() {
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
