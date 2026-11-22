//! `anchorkit audit get|list` — fetch and display audit log entries.

use base64::Engine;
use soroban_client::{
    address::{Address, AddressTrait},
    contract::{ContractBehavior, Contracts},
    xdr::{Limits, ReadXdr, ScVal},
};

use crate::output::format_timestamp;
use crate::settings;

pub fn run_audit_get(log_id: u64) {
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

pub fn run_audit_list(session: u64, from: Option<u64>, to: Option<u64>, format: &str, pretty: bool) {
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

/// Fetch a single audit log entry from on-chain contract storage.
///
/// Issue #1399 fix: the previous implementation used `getEvents` with
/// `offset: log_id`, treating the log ID as an event offset. Event position
/// has no relationship to the audit log ID — other contract events are
/// interleaved, pruned entries shift positions, and the RPC only retains
/// recent ledgers.
///
/// The fix simulates a `get_audit_log(log_id)` contract call via
/// `simulateTransaction` (no signing needed for reads) and decodes the
/// returned `AuditLog` struct. If the contract returns a `not-found`
/// error or the entry has been pruned, a clear message is shown.
fn fetch_audit_log_entry(log_id: u64) -> Option<AuditLogEntry> {
    let rpc_url = std::env::var("ANCHORKIT_RPC_URL")
        .or_else(|_| std::env::var("SOROBAN_RPC_URL"))
        .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string());
    // Issue #1498: --contract-id flag → env vars → config file.
    let contract_id = match settings::require_contract_id() {
        Ok(id) => id,
        Err(e) => {
            eprintln!("warning: {e}; cannot fetch audit entry from chain");
            return None;
        }
    };

    // ── Issue #1399: simulate get_audit_log(log_id) ─────────────────────────
    //
    // Build a `simulateTransaction` request for a read-only contract call.
    // No signing is needed because simulation of read-only invocations does
    // not require authorization.
    //
    // The contract exposes `get_audit_log(log_id: u64) -> AuditLog` which
    // returns the on-chain AuditLog struct for the given ID, or panics
    // with an error if the ID does not exist or has been pruned.

    // Build the contract invocation XDR for simulateTransaction.
    // We use a raw JSON-RPC call since the soroban_client crate's
    // simulateTransaction API requires a full TransactionBuilder pipeline
    // which is complex for a read-only simulation.
    let contract_addr = match Contracts::new(&contract_id) {
        Ok(c) => c.address().to_sc_address().ok(),
        Err(_) => None,
    };

    let contract_addr = match contract_addr {
        Some(addr) => addr,
        None => {
            eprintln!("warning: invalid contract ID '{contract_id}'");
            return None;
        }
    };

    // Encode the contract invocation: get_audit_log(log_id)
    let invoke_args = serde_json::json!({
        "contract_id": contract_id,
        "function_name": "get_audit_log",
        "args": [log_id.to_string()],
    });

    // Use simulateTransaction to read the audit log without signing.
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "simulateTransaction",
        "params": {
            "transaction": invoke_args,
            "resourceLeeway": 15
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

    // Check for simulation errors — the contract may return an error if
    // the log ID does not exist or has been pruned.
    if let Some(error) = json.get("error") {
        let msg = error["message"].as_str().unwrap_or("unknown error");
        if msg.contains("not found") || msg.contains("pruned") || msg.contains("NotFound") {
            eprintln!("ℹ  Audit log {log_id} not found or has been pruned: {msg}");
            return None;
        }
        eprintln!("warning: contract simulation failed: {msg}");
        return None;
    }

    // Also check the simulation result for a contract error.
    if let Some(err) = json["result"]["error"].as_str() {
        if err.contains("not found") || err.contains("pruned") || err.contains("NotFound") {
            eprintln!("ℹ  Audit log {log_id} not found or has been pruned: {err}");
            return None;
        }
        eprintln!("warning: contract returned an error: {err}");
        return None;
    }

    // Decode the returned AuditLog struct from the simulation result.
    // The result is in `result.results[0].xdr` as a base64-encoded ScVal.
    let results = json["result"]["results"].as_array()?;
    let first_result = results.first()?;
    let xdr_str = first_result["xdr"].as_str()?;

    // Decode the base64 XDR into an ScVal.
    let xdr_bytes = match base64::engine::general_purpose::STANDARD.decode(xdr_str) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("warning: failed to decode base64 XDR: {e}");
            return None;
        }
    };

    let scval: ScVal = match ScVal::from_xdr(&xdr_bytes, Limits::none()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("warning: failed to decode XDR ScVal: {e}");
            return None;
        }
    };

    // Parse the AuditLog struct from the ScVal::Vec.
    // Expected field order: log_id, session_id, actor, operation_type,
    // operation_index, timestamp, status, result.
    let fields = match scval {
        ScVal::Vec(Some(fields)) => fields,
        ScVal::Error(err) => {
            let msg = format!("{err:?}");
            if msg.contains("NotFound") || msg.contains("pruned") {
                eprintln!("ℹ  Audit log {log_id} not found or has been pruned (contract error: {msg})");
            } else {
                eprintln!("warning: contract returned an error: {msg}");
            }
            return None;
        }
        other => {
            eprintln!("warning: expected an AuditLog struct, got {other:?}");
            return None;
        }
    };

    let get_u64 = |idx: usize, name: &str| -> Option<u64> {
        match &fields.get(idx)? {
            ScVal::U64(v) => Some(*v),
            _ => {
                eprintln!("warning: field '{name}' at index {idx} is not U64");
                None
            }
        }
    };

    let get_string = |idx: usize, name: &str| -> Option<String> {
        match &fields.get(idx)? {
            ScVal::String(s) => Some(s.0.to_string()),
            ScVal::Symbol(s) => Some(s.0.to_string()),
            ScVal::Address(addr) => Some(Address::from_sc_address(addr).ok()?.to_string()),
            _ => {
                eprintln!("warning: field '{name}' at index {idx} is not a string");
                None
            }
        }
    };

    let parsed_log_id = get_u64(0, "log_id")?;
    let session_id = get_u64(1, "session_id")?;
    let actor = get_string(2, "actor")?;
    let operation_type = get_string(3, "operation_type")?;
    let operation_index = get_u64(4, "operation_index")?;
    let timestamp = get_u64(5, "timestamp")?;
    let status = get_string(6, "status")?;
    let result = get_string(7, "result")?;

    // Verify the on-chain log_id matches the requested ID.
    if parsed_log_id != log_id {
        eprintln!(
            "warning: contract returned audit log with ID {parsed_log_id} but requested {log_id}"
        );
        return None;
    }

    Some(AuditLogEntry {
        log_id: parsed_log_id,
        session_id,
        actor,
        operation_type,
        operation_index,
        timestamp,
        status,
        result,
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
    // Issue #1498: --contract-id flag → env vars → config file.
    let contract_id = match settings::require_contract_id() {
        Ok(id) => id,
        Err(e) => {
            eprintln!("warning: {e}; cannot fetch session audit logs from chain");
            return vec![];
        }
    };

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
            results.push(parse_audit_log_entry_from_event(ev, offset + i as u64, session_id));
        }

        if count < limit { break; }
        offset += count;
        if offset > range_end { break; }
    }

    results
}

/// Build an `AuditLogEntry` from a `getEvents` event. The log ID comes from
/// the event payload's on-chain `log_id`; `fallback_log_id` (the pagination
/// position) is only used when the payload does not carry one.
fn parse_audit_log_entry_from_event(
    ev: &serde_json::Value,
    fallback_log_id: u64,
    session_id: u64,
) -> AuditLogEntry {
    AuditLogEntry {
        log_id: ev["value"]["log_id"].as_u64().unwrap_or(fallback_log_id),
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
    }
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
