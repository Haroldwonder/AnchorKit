//! `anchorkit health` — show attestor health.

use crate::output::{print_ascii_table, validate_output_format};
use crate::settings::contract_id_or_placeholder;

pub fn run_health(attestor: Option<&str>, watch: bool, interval: u64, output: &str) {
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
    // Issue #1498: use the configured contract (--contract-id, env or config file).
    let contract_id = contract_id_or_placeholder();

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
                    "   soroban contract invoke --id {} --source <ACCOUNT> \
                     --network testnet -- get_anchor_health_score --anchor {}",
                    contract_id, addr
                );
            } else {
                println!(
                    "   soroban contract invoke --id {} --source <ACCOUNT> \
                     --network testnet -- get_all_attestors",
                    contract_id
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
            println!("    --id {} \\", contract_id);
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
