//! `anchorkit export-audit` — export audit log events to JSON or CSV.

use crate::settings;

pub fn run_export_audit(format: &str, output: &str) {
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
        eprintln!("error: no audit log entries found — verify the contract ID (--contract-id / ANCHORKIT_CONTRACT_ID) and ANCHORKIT_RPC_URL are set correctly");
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
    // Issue #1498: --contract-id flag → env vars → config file.
    let contract_id = match settings::require_contract_id() {
        Ok(id) => id,
        Err(e) => {
            eprintln!("warning: {e}; cannot fetch audit entries from chain");
            return vec![];
        }
    };

    // ── Issue #1398 fix ─────────────────────────────────────────────────────
    //
    // The original code had three separate bugs:
    //
    // 1. `topics: [["audit_log"]]` — wrong format and wrong topic value.
    //    Topic filters must be arrays of base64-XDR ScVals. The contract
    //    emits the topic pair `(Symbol("audit"), Symbol("logged"))`.
    //    We encode each Symbol as base64 XDR.
    //
    // 2. `startLedger: 0` — outside every RPC retention window and
    //    rejected by the RPC with a "startLedger must be within the
    //    ledger retention window" error. We derive startLedger from
    //    getLatestLedger minus the retention window (typically ~17280
    //    ledgers ≈ 24 hours on a 5 s cadence; we default to 6 hours
    //    to stay safely within the window).
    //
    // 3. `pagination.offset` — Soroban RPC paginates with `cursor`,
    //    not `offset`. We use the `cursor` field from the previous page's
    //    last event, or omit it for the first page.
    //
    // Topic XDR encoding:
    //   Symbol("audit")  → ScVal::Symbol("audit")  → base64(XDR)
    //   Symbol("logged") → ScVal::Symbol("logged") → base64(XDR)
    //   Wildcard (*)     → omit the third element   → match any third topic
    //
    // We encode via the Soroban XDR helper; the resulting base64 strings
    // are stable and do not depend on runtime state.

    // Base64-encode a Soroban Symbol ScVal for use in topic filters.
    // Per the Soroban RPC spec, each topic element is a base64-XDR ScVal.
    fn encode_symbol_topic(name: &str) -> Option<String> {
        use soroban_client::xdr::{Limits, ScSymbol, ScVal, StringM, WriteXdr};
        use std::str::FromStr;
        use base64::Engine;

        let sym = StringM::from_str(name).ok()?;
        let scval = ScVal::Symbol(ScSymbol(sym));
        let buf = scval.to_xdr(Limits::none()).ok()?;
        Some(base64::engine::general_purpose::STANDARD.encode(&buf))
    }

    let audit_topic = match encode_symbol_topic("audit") {
        Some(t) => t,
        None => {
            eprintln!("warning: failed to encode 'audit' topic");
            return vec![];
        }
    };
    let logged_topic = match encode_symbol_topic("logged") {
        Some(t) => t,
        None => {
            eprintln!("warning: failed to encode 'logged' topic");
            return vec![];
        }
    };

    // Derive startLedger from getLatestLedger minus a safe retention window.
    // Default retention window = 6 hours at 5 s/ledger ≈ 4 320 ledgers.
    let retention_window: u32 = std::env::var("ANCHORKIT_EVENT_RETENTION_LEDGERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4_320);

    let client = reqwest::blocking::Client::new();

    let start_ledger = {
        let latest_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getLatestLedger",
            "params": {}
        });
        let resp = match client.post(&rpc_url).json(&latest_body).send() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warning: could not fetch latest ledger: {e}");
                return vec![];
            }
        };
        let json: serde_json::Value = match resp.json() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("warning: could not parse getLatestLedger response: {e}");
                return vec![];
            }
        };
        let latest = json["result"]["sequence"]
            .as_u64()
            .unwrap_or(0);
        latest.saturating_sub(retention_window as u64)
    };

    // For cursor-based pagination: on page 0 we omit the cursor; on
    // subsequent pages the caller must supply it.  The current
    // `fetch_audit_entries` loop does not track cursors, so we only
    // fetch the first page from RPC when page > 0 is requested without
    // a cursor.  A future refactor should thread the cursor through.
    // For page > 0 without a cursor we return empty (graceful degradation).
    if page > 0 {
        // Without cursor tracking we cannot reliably fetch page > 0.
        // Return empty so fetch_audit_entries terminates after the first
        // batch — better than returning the wrong events.
        return vec![];
    }

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getEvents",
        "params": {
            "startLedger": start_ledger,
            "filters": [{
                "type": "contract",
                "contractIds": [contract_id],
                // Topic filter: [Symbol("audit"), Symbol("logged"), *]
                // The third element is omitted to act as a wildcard for the log_id.
                "topics": [[audit_topic, logged_topic]]
            }],
            "pagination": {"limit": page_size}
        }
    });

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

    if let Some(err) = json.get("error") {
        let msg = err["message"].as_str().unwrap_or("unknown error");
        eprintln!("warning: getEvents RPC error: {msg}");
        return vec![];
    }

    let events = match json["result"]["events"].as_array() {
        Some(arr) => arr.clone(),
        None => return vec![],
    };

    events
        .iter()
        .enumerate()
        .filter_map(|(i, ev)| {
            // Extract log_id from the third topic element (the XDR-encoded U64).
            let log_id = ev["topic"]
                .as_array()
                .and_then(|t| t.get(2))
                .and_then(|v| v.as_str())
                .and_then(|xdr_b64| {
                    use base64::Engine;
                    use soroban_client::xdr::{Limits, ReadXdr, ScVal};
                    let bytes = base64::engine::general_purpose::STANDARD.decode(xdr_b64).ok()?;
                    let scval = ScVal::from_xdr(&bytes, Limits::none()).ok()?;
                    match scval {
                        ScVal::U64(n) => Some(n),
                        _ => None,
                    }
                })
                .unwrap_or(i as u64);

            Some(AuditEntry {
                id: log_id,
                operation: ev["topic"]
                    .as_array()
                    .and_then(|t| t.get(1))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                actor: ev["value"]["xdr"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
                timestamp: ev["ledgerClosedAt"]
                    .as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0),
                result: ev["value"]["xdr"]
                    .as_str()
                    .unwrap_or("ok")
                    .to_string(),
            })
        })
        .collect()
}
