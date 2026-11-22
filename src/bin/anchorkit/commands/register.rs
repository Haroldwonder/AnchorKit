//! `anchorkit register` — validate and print the steps to register an attestor.

use crate::output::{print_ascii_table, validate_output_format};
use crate::validation::is_valid_g_address;

/// The complete set of valid service names for anchorkit register --services.
const VALID_SERVICES: &[&str] = &["deposits", "withdrawals", "quotes", "kyc"];

pub fn run_register(address: &str, services: Option<&str>, endpoint: Option<&str>, output: &str) {
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
