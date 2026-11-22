#![cfg(feature = "std")]

//! AnchorKit CLI entry point.
//!
//! This file only defines the argument parser and dispatches to the
//! per-command modules under `commands/`. Shared output helpers live in
//! `output.rs`, CLI-wide settings (network, contract ID) in `settings.rs`,
//! input validators in `validation.rs` and the Soroban RPC client in
//! `soroban_rpc.rs`.

use clap::{Parser, Subcommand};

mod commands;
mod output;
mod settings;
mod soroban_rpc;
mod validation;

use commands::*;
use settings::validate_network_arg;
use validation::validate_contract_id;

#[derive(Parser)]
#[command(name = "anchorkit", about = "AnchorKit CLI for Soroban anchor management")]
struct Cli {
    /// Deployed AnchorKit contract ID (starts with C, 56 chars).
    /// Takes precedence over the ANCHORKIT_CONTRACT_ID / SOROBAN_CONTRACT_ID
    /// env vars and the `deployment.contract_id` config file key.
    #[arg(long, global = true, value_name = "CONTRACT_ID", value_parser = validate_contract_id)]
    contract_id: Option<String>,
    /// Config file (JSON or TOML) to read `deployment.contract_id` from.
    /// Defaults to the ANCHORKIT_CONFIG env var.
    #[arg(long, global = true, value_name = "PATH")]
    config: Option<String>,
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
        /// Target network (testnet, mainnet, futurenet).
        /// Defaults to the ANCHORKIT_NETWORK env var, then "testnet".
        #[arg(long, value_parser = validate_network_arg)]
        network: Option<String>,
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
        /// Target network (testnet, mainnet, futurenet).
        /// Defaults to the ANCHORKIT_NETWORK env var, then "testnet".
        #[arg(long, value_parser = validate_network_arg)]
        network: Option<String>,
    },
    /// Query attestation by ID
    Query {
        /// Attestation ID to query (the numeric ID returned by submit_attestation)
        #[arg(long)]
        transaction_id: String,
        /// Output format: json, text, or table
        #[arg(long, default_value = "text")]
        output: String,
        /// Target network (testnet, mainnet, futurenet).
        /// Defaults to the ANCHORKIT_NETWORK env var, then "testnet".
        #[arg(long, value_parser = validate_network_arg)]
        network: Option<String>,
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
    settings::init(cli.contract_id, cli.config);

    match cli.command {
        Commands::Build { release } => build::run_build(release),
        Commands::Deploy { network } => deploy::run_deploy(&network),
        Commands::Init { admin, network } => init::run_init(&admin, network.as_deref()),
        Commands::Doctor => doctor::run_doctor(),
        Commands::Validate { path } => validate::run_validate(&path),
        Commands::Register { address, services, endpoint, output } => {
            register::run_register(&address, services.as_deref(), endpoint.as_deref(), &output)
        }
        Commands::Attest { subject, payload_hash, session, network } => {
            attest::run_attest(&subject, &payload_hash, session.as_deref(), network.as_deref())
        }
        Commands::Query { transaction_id, output, network } => {
            query::run_query(&transaction_id, &output, network.as_deref())
        }
        Commands::Health { attestor, watch, interval, output } => {
            health::run_health(attestor.as_deref(), watch, interval, &output)
        }
        Commands::Test { pattern, verbose } => test::run_test(pattern.as_deref(), verbose),
        Commands::ExportAudit { format, output } => export_audit::run_export_audit(&format, &output),
        Commands::Config { command } => match command {
            ConfigCommands::Validate { path } => config::run_config_validate(&path),
            ConfigCommands::Init => config::run_config_init(),
        },
        Commands::Audit { action } => match action {
            AuditAction::Get { log_id } => audit::run_audit_get(log_id),
            AuditAction::List { session, from, to, format, pretty } => {
                audit::run_audit_list(session, from, to, &format, pretty)
            }
        },
        Commands::Session { action } => match action {
            SessionAction::Create { initiator } => session::run_session_create(initiator),
            SessionAction::Get { session_id } => session::run_session_get(session_id),
            SessionAction::List { format, pretty, limit } => {
                session::run_session_list(&format, pretty, limit)
            }
        },
    }
}
