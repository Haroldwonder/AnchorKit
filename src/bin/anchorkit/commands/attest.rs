//! `anchorkit attest` — validate and print the steps to submit an attestation.

use crate::settings::{contract_id_or_placeholder, resolve_network, CONTRACT_ID_PLACEHOLDER};
use crate::validation::is_valid_g_address;
use crate::exit_code;

pub fn run_attest(
    subject: &str,
    payload_hash: &str,
    session: Option<&str>,
    schema_id: Option<&str>,
    network_flag: Option<&str>,
) {
    println!("📝 Submitting attestation...");
    println!("  Subject: {}", subject);
    println!("  Payload Hash: {}", payload_hash);
    if let Some(s) = session {
        println!("  Session: {}", s);
    }
    if let Some(schema) = schema_id {
        println!("  Schema ID: {}", schema);
    }

    // Resolve effective network: CLI flag → ANCHORKIT_NETWORK env → "testnet".
    // Issue #1397: the generated command must reflect the user's actual network.
    let network = resolve_network(network_flag);

    // Validate inputs
    if !is_valid_g_address(subject) {
        eprintln!("❌ Invalid subject address format. Expected Stellar public key starting with 'G' followed by 55 base32 characters (A-Z2-7)");
        std::process::exit(exit_code::USAGE);
    }

    if payload_hash.len() != 64 || !payload_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        eprintln!("❌ Invalid payload hash format (expected 64-character hex string)");
        std::process::exit(exit_code::USAGE);
    }

    if let Some(schema) = schema_id {
        let valid = !schema.is_empty()
            && schema.len() <= 32
            && schema
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !valid {
            eprintln!("❌ Invalid schema ID (expected 1-32 letters, digits, or underscores)");
            std::process::exit(1);
        }
    }


    // Issue #1498: use the configured contract (--contract-id, env or config file).
    let contract_id = contract_id_or_placeholder();

    println!("📋 Attestation steps:");
    println!("  Use stellar CLI to submit:");
    println!("  stellar contract invoke \\");
    println!("    --id {} \\", contract_id);
    println!("    --source <ATTESTOR_ACCOUNT> \\");
    println!("    --network {} \\", network);
    println!("    -- \\");
    match (session, schema_id) {
        (Some(s), Some(schema)) => {
            println!("    submit_session_with_schema \\");
            println!("    --session {} \\", s);
            println!("    --schema-id {} \\", schema);
        }
        (Some(s), None) => {
            println!("    submit_attestation_with_session \\");
            println!("    --session {} \\", s);
        }
        (None, Some(schema)) => {
            println!("    submit_attestation_with_schema \\");
            println!("    --schema-id {} \\", schema);
        }
        (None, None) => {
            println!("    submit_attestation \\");
        }
    }
    println!("    --subject {} \\", subject);
    println!("    --payload-hash {}", payload_hash);
    println!();
    if contract_id == CONTRACT_ID_PLACEHOLDER {
        println!("💡 Note: Replace <CONTRACT_ID> and <ATTESTOR_ACCOUNT> with actual values");
        println!("   (or pass --contract-id / set ANCHORKIT_CONTRACT_ID to fill in the contract)");
    } else {
        println!("💡 Note: Replace <ATTESTOR_ACCOUNT> with the attestor's account");
    }
}
