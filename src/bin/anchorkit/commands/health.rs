//! `anchorkit health` — show live attestor health read from contract storage.

use crate::exit_code;
use crate::output::{print_ascii_table, validate_output_format};
use crate::settings::{contract_id_or_placeholder, resolve_network};
use crate::soroban_rpc::{get_health_rpc, health_config};

pub fn run_health(attestor: Option<&str>, watch: bool, interval: u64, output: &str, network_flag: Option<&str>) {
    validate_output_format(output);

    // Issue #1493: respect the configured network and fail clearly when the
    // contract / RPC endpoint are not configured instead of printing samples.
    let network = resolve_network(network_flag);
    if let Err(e) = health_config(&network) {
        eprintln!("❌ Health check not configured: {}", e);
        std::process::exit(1);
    }

    if watch {
        if interval == 0 {
            eprintln!("❌ Invalid interval '0'. --interval must be greater than 0 seconds");
            std::process::exit(exit_code::USAGE);
        }
        eprintln!("👀 Watch mode enabled (interval: {}s) — press Ctrl+C to stop", interval);
        loop {
            if let Err(e) = render_health(attestor, output, &network) {
                eprintln!("❌ {}", e);
            }
            std::thread::sleep(std::time::Duration::from_secs(interval));
        }
    }

    if let Err(e) = render_health(attestor, output, &network) {
        eprintln!("❌ {}", e);
        std::process::exit(1);
    }
}

fn render_health(attestor: Option<&str>, output: &str, network: &str) -> Result<(), String> {
    // Live health records read from the contract's HealthStatus storage
    // (the data returned by `get_health_status`).
    let records = get_health_rpc(attestor, network)?;

    match output {
        "json" => {
            let json_records: Vec<serde_json::Value> = records
                .iter()
                .map(|r| serde_json::json!({
                    "anchor":               r.anchor,
                    "latency_ms":           r.latency_ms,
                    "failure_count":        r.failure_count,
                    "availability_percent": r.availability_percent,
                }))
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_records).unwrap());
        }
        "table" => {
            println!("🏥 Health status ({})\n", network);
            let headers = &["Anchor", "Latency (ms)", "Failures", "Availability (%)"];
            let rows: Vec<Vec<String>> = records
                .iter()
                .map(|r| vec![
                    r.anchor.clone(),
                    r.latency_ms.to_string(),
                    r.failure_count.to_string(),
                    r.availability_percent.to_string(),
                ])
                .collect();
            print_ascii_table(headers, &rows);
            if records.is_empty() {
                println!("  No health status recorded for any attestor");
            }
            if let Some(addr) = attestor {
                println!("\n💡 Health score:");
                println!(
                    "   stellar contract invoke --id {} --source <ACCOUNT> \
                     --network {} -- get_anchor_health_score --anchor {}",
                    contract_id_or_placeholder(), network, addr
                );
            }
        }
        _ => {
            // "text" (default)
            if let Some(addr) = attestor {
                println!("🏥 Health for attestor: {} ({})", addr, network);
            } else {
                println!("🏥 Health for all attestors ({}):", network);
            }
            if records.is_empty() {
                println!("  No health status recorded for any attestor");
            }
            for r in &records {
                println!(
                    "  anchor={} latency={}ms failures={} availability={}%",
                    r.anchor, r.latency_ms, r.failure_count, r.availability_percent
                );
            }
            if let Some(addr) = attestor {
                println!("\n📋 Health score:");
                println!("  stellar contract invoke \\");
                println!("    --id {} \\", contract_id_or_placeholder());
                println!("    --source <ACCOUNT> \\");
                println!("    --network {} \\", network);
                println!("    -- \\");
                println!("    get_anchor_health_score \\");
                println!("    --anchor {}", addr);
            }
        }
    }
    Ok(())
}
