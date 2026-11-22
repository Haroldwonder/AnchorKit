//! `anchorkit query` — fetch an attestation from the deployed contract.

use crate::output::{print_ascii_table, validate_output_format};
use crate::settings::resolve_network;
use crate::soroban_rpc::{get_attestation_rpc, get_rpc_url_for_network, AttestationData};

pub fn run_query(transaction_id: &str, output: &str, network_flag: Option<&str>) {
    validate_output_format(output);

    // Issue #1397: use the effective network so the right RPC endpoint is used.
    let network = resolve_network(network_flag);

    // The contract's `get_attestation(id: u64)` takes the numeric attestation
    // ID returned by `submit_attestation`.
    let attestation_id: u64 = match transaction_id.trim().parse() {
        Ok(id) => id,
        Err(_) => fail(&format!(
            "Invalid attestation ID '{}': expected the numeric ID returned by submit_attestation",
            transaction_id
        )),
    };

    let rpc_url = get_rpc_url_for_network(&network).unwrap_or_else(|e| fail(&e));

    // Issue #1492: always query the chain; never print placeholder values.
    let attestation = match get_attestation_rpc(attestation_id, &rpc_url) {
        Ok(a) => a,
        Err(e) => fail(&format!("Failed to query attestation {}: {}", attestation_id, e)),
    };

    let status = attestation_status(&attestation);
    let expires_at = attestation
        .expires_at
        .map(|t| t.to_string())
        .unwrap_or_else(|| "never".to_string());
    let record = [
        ("Attestation ID", attestation.id.to_string()),
        ("Status",         status.to_string()),
        ("Subject",        attestation.subject.clone()),
        ("Issuer",         attestation.issuer.clone()),
        ("Payload Hash",   attestation.payload_hash.clone()),
        ("Signature",      attestation.signature.clone()),
        ("Timestamp",      attestation.timestamp.to_string()),
        ("Expires At",     expires_at),
        ("Issuer Revoked", attestation.issuer_revoked.to_string()),
    ];

    match output {
        "json" => {
            let obj = serde_json::json!({
                "id": attestation.id,
                "status": status,
                "subject": attestation.subject,
                "issuer": attestation.issuer,
                "payload_hash": attestation.payload_hash,
                "signature": attestation.signature,
                "timestamp": attestation.timestamp,
                "expires_at": attestation.expires_at,
                "issuer_revoked": attestation.issuer_revoked,
                "network": network,
            });
            println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        }
        "table" => {
            println!("🔍 Attestation {} ({})\n", attestation.id, network);
            let headers = &["Field", "Value"];
            let rows: Vec<Vec<String>> = record
                .iter()
                .map(|(k, v)| vec![k.to_string(), v.clone()])
                .collect();
            print_ascii_table(headers, &rows);
        }
        _ => {
            // "text" (default)
            println!("🔍 Attestation {} ({})\n", attestation.id, network);
            for (k, v) in &record {
                println!("  {:<16} {}", format!("{}:", k), v);
            }
        }
    }
}

/// `revoked` when the issuer's authority was withdrawn, `expired` once
/// `expires_at` has passed, `valid` otherwise.
fn attestation_status(attestation: &AttestationData) -> &'static str {
    if attestation.issuer_revoked {
        return "revoked";
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    match attestation.expires_at {
        Some(expires_at) if expires_at <= now => "expired",
        _ => "valid",
    }
}

fn fail(message: &str) -> ! {
    eprintln!("❌ {}", message);
    std::process::exit(1);
}
