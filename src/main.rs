use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "contract-cli", version, about = "Contract CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Register a new entry
    Register {
        /// Identifier to register
        id: String,
        /// Simulate the transaction without submitting
        #[arg(long)]
        dry_run: bool,
    },
    /// Attest to an existing entry
    Attest {
        /// Identifier to attest
        id: String,
        /// Simulate the transaction without submitting
        #[arg(long)]
        dry_run: bool,
    },
    /// Revoke an existing entry
    Revoke {
        /// Identifier to revoke
        id: String,
        /// Simulate the transaction without submitting
        #[arg(long)]
        dry_run: bool,
    },
    /// Pause the contract
    Pause {
        /// Simulate the transaction without submitting
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Register { id, dry_run } => {
            run_state_change("register", Some(&id), dry_run);
        }
        Commands::Attest { id, dry_run } => {
            run_state_change("attest", Some(&id), dry_run);
        }
        Commands::Revoke { id, dry_run } => {
            run_state_change("revoke", Some(&id), dry_run);
        }
        Commands::Pause { dry_run } => {
            run_state_change("pause", None, dry_run);
        }
    }
}

/// Simulate or submit a state-changing contract call.
///
/// When `dry_run` is set, the transaction is simulated and the expected
/// result and fee are printed without submitting anything on-chain.
fn run_state_change(action: &str, id: Option<&str>, dry_run: bool) {
    let (expected_result, expected_fee) = simulate(action, id);

    if dry_run {
        println!("[dry-run] action: {action}");
        if let Some(id) = id {
            println!("[dry-run] id: {id}");
        }
        println!("[dry-run] expected result: {expected_result}");
        println!("[dry-run] expected fee: {expected_fee}");
        println!("[dry-run] transaction not submitted");
        return;
    }

    submit(action, id);
}

/// Placeholder simulation returning the expected result and fee.
fn simulate(action: &str, id: Option<&str>) -> (String, String) {
    let result = match id {
        Some(id) => format!("{action} would succeed for '{id}'"),
        None => format!("{action} would succeed"),
    };
    (result, "0.001".to_string())
}

/// Placeholder submission path for the actual transaction.
fn submit(action: &str, id: Option<&str>) {
    match id {
        Some(id) => println!("submitted {action} for '{id}'"),
        None => println!("submitted {action}"),
    }
}
