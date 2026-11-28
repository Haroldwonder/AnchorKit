//! CLI-wide settings: target network and deployed contract ID.
//!
//! # Contract ID precedence
//!
//! The contract ID used by `query`, `health`, `attest`, `session` and `audit`
//! (and `export-audit`) is resolved in this order — the first one set wins:
//!
//! 1. the global `--contract-id <CONTRACT_ID>` flag
//! 2. the `ANCHORKIT_CONTRACT_ID` env var
//! 3. the `SOROBAN_CONTRACT_ID` env var
//! 4. the `deployment.contract_id` key of the config file given by the global
//!    `--config <PATH>` flag or, if that is absent, the `ANCHORKIT_CONFIG`
//!    env var (JSON or TOML, e.g. the file written by `anchorkit config init`)

use std::sync::OnceLock;

use crate::validation::validate_contract_id;

pub const VALID_NETWORKS: &[&str] = &["testnet", "mainnet", "futurenet"];

pub const CONTRACT_ID_ENV: &str = "ANCHORKIT_CONTRACT_ID";
pub const CONTRACT_ID_ENV_ALT: &str = "SOROBAN_CONTRACT_ID";
pub const CONFIG_ENV: &str = "ANCHORKIT_CONFIG";

/// Shown wherever a command needs a contract ID but none is configured.
pub const CONTRACT_ID_NOT_CONFIGURED: &str = "no contract ID configured — pass --contract-id <CONTRACT_ID>, \
     set ANCHORKIT_CONTRACT_ID (or SOROBAN_CONTRACT_ID), or add `deployment.contract_id` \
     to the config file given by --config / ANCHORKIT_CONFIG";

/// Placeholder printed in generated soroban CLI commands when no contract ID
/// is configured.
pub const CONTRACT_ID_PLACEHOLDER: &str = "<CONTRACT_ID>";

static CONTRACT_ID_FLAG: OnceLock<Option<String>> = OnceLock::new();
static CONFIG_FLAG: OnceLock<Option<String>> = OnceLock::new();

/// Record the global `--contract-id` and `--config` flags. Called once from
/// `main` right after argument parsing.
pub fn init(contract_id: Option<String>, config: Option<String>) {
    let _ = CONTRACT_ID_FLAG.set(contract_id);
    let _ = CONFIG_FLAG.set(config);
}

// ── network ──────────────────────────────────────────────────────────────────

pub fn validate_network_arg(s: &str) -> Result<String, String> {
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

/// Resolve the effective network from an optional CLI flag, the
/// `ANCHORKIT_NETWORK` env var, or the "testnet" default (in that order).
pub fn resolve_network(flag: Option<&str>) -> String {
    if let Some(n) = flag {
        return n.to_string();
    }
    std::env::var("ANCHORKIT_NETWORK")
        .or_else(|_| std::env::var("SOROBAN_NETWORK"))
        .unwrap_or_else(|_| "testnet".to_string())
}

// ── contract ID ──────────────────────────────────────────────────────────────

/// Resolve the contract ID using the precedence documented at the top of this
/// module. Returns `Ok(None)` when no source provides one, and `Err` when a
/// source is set but invalid (bad format, unreadable config file, …).
pub fn contract_id() -> Result<Option<String>, String> {
    if let Some(Some(id)) = CONTRACT_ID_FLAG.get() {
        return Ok(Some(id.clone()));
    }

    for var in [CONTRACT_ID_ENV, CONTRACT_ID_ENV_ALT] {
        if let Ok(value) = std::env::var(var) {
            let value = value.trim();
            if !value.is_empty() {
                return validate_contract_id(value)
                    .map(Some)
                    .map_err(|e| format!("{var}: {e}"));
            }
        }
    }

    match config_path() {
        Some(path) => contract_id_from_config(&path),
        None => Ok(None),
    }
}

/// Like [`contract_id`], but treats a missing contract ID as an error.
pub fn require_contract_id() -> Result<String, String> {
    contract_id()?.ok_or_else(|| CONTRACT_ID_NOT_CONFIGURED.to_string())
}

/// Contract ID to show in generated soroban CLI commands: the configured one,
/// or `<CONTRACT_ID>` when none is set. Exits with an error when a source is
/// set but invalid, rather than silently falling back to the placeholder.
pub fn contract_id_or_placeholder() -> String {
    match contract_id() {
        Ok(Some(id)) => id,
        Ok(None) => CONTRACT_ID_PLACEHOLDER.to_string(),
        Err(e) => {
            eprintln!("❌ {}", e);
            std::process::exit(1);
        }
    }
}

fn config_path() -> Option<String> {
    if let Some(Some(path)) = CONFIG_FLAG.get() {
        return Some(path.clone());
    }
    std::env::var(CONFIG_ENV).ok().filter(|p| !p.trim().is_empty())
}

/// Read `deployment.contract_id` from a JSON or TOML config file.
fn contract_id_from_config(path: &str) -> Result<Option<String>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read config file '{}': {}", path, e))?;

    let config: serde_json::Value = if path.ends_with(".toml") {
        let value: toml::Value = toml::from_str(&content)
            .map_err(|e| format!("invalid TOML in config file '{}': {}", path, e))?;
        serde_json::to_value(value)
            .map_err(|e| format!("cannot read config file '{}': {}", path, e))?
    } else {
        serde_json::from_str(&content)
            .map_err(|e| format!("invalid JSON in config file '{}': {}", path, e))?
    };

    match config.get("deployment").and_then(|d| d.get("contract_id")) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(id)) if id.trim().is_empty() => Ok(None),
        Some(serde_json::Value::String(id)) => validate_contract_id(id.trim())
            .map(Some)
            .map_err(|e| format!("{}: deployment.contract_id: {}", path, e)),
        Some(_) => Err(format!(
            "{}: deployment.contract_id must be a string",
            path
        )),
    }
}
