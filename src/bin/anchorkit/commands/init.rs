//! `anchorkit init` — print the steps to initialize the contract with an admin.

use crate::settings::resolve_network;
use crate::validation::is_valid_g_address;
use crate::exit_code;

pub fn run_init(admin: &str, network_flag: Option<&str>) {
    println!("🔧 Initializing contract with admin: {}", admin);

    // Resolve effective network: CLI flag → ANCHORKIT_NETWORK env → "testnet".
    // Issue #1397: the generated command must reflect the user's actual network,
    // not always "testnet".
    let network = resolve_network(network_flag);

    // Validate admin address format (prefix, length, and base32 strkey charset)
    if !is_valid_g_address(admin) {
        eprintln!("❌ Invalid admin address format. Expected Stellar public key starting with 'G' followed by 55 base32 characters (A-Z2-7)");
        std::process::exit(exit_code::USAGE);
    }

    println!("📋 Initialization steps:");
    println!("  Use stellar CLI to initialize:");
    println!("  stellar contract invoke \\");
    println!("    --id <CONTRACT_ID> \\");
    println!("    --source {} \\", admin);
    println!("    --network {} \\", network);
    println!("    -- \\");
    println!("    initialize \\");
    println!("    --admin {}", admin);
    println!();
    println!("💡 Note: Replace <CONTRACT_ID> with your deployed contract address");
}
