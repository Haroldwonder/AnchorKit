use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cli", about = "State-changing contract CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Register an entry with the contract
    Register {
        /// Identifier to register
        id: String,
        /// Simulate the transaction without submitting it
        #[arg(long)]
        dry_run: bool,
    },
    /// Attest to an existing entry
    Attest {
        /// Identifier to attest
        id: String,
        /// Simulate the transaction without submitting it
        #[arg(long)]
        dry_run: bool,
    },
    /// Revoke an existing entry
    Revoke {
        /// Identifier to revoke
        id: String,
        /// Simulate the transaction without submitting it
        #[arg(long)]
        dry_run: bool,
    },
    /// Pause the contract
    Pause {
        /// Simulate the transaction without submitting it
        #[arg(long)]
        dry_run: bool,
    },
}

/// Result of simulating or submitting a state-changing transaction.
pub struct TxOutcome {
    pub result: String,
    pub fee: u64,
}

/// Simulate a state-changing transaction and report the expected result and fee.
pub fn simulate(action: &str, id: Option<&str>) -> TxOutcome {
    let result = match id {
        Some(id) => format!("{action} would succeed for '{id}' (not submitted)"),
        None => format!("{action} would succeed (not submitted)"),
    };
    TxOutcome {
        result,
        fee: estimate_fee(action),
    }
}

/// Estimate the fee for a given action.
fn estimate_fee(action: &str) -> u64 {
    match action {
        "register" => 100,
        "attest" => 50,
        "revoke" => 50,
        "pause" => 25,
        _ => 0,
    }
}

/// Print the dry-run outcome for a command.
fn print_dry_run(outcome: &TxOutcome) {
    println!("[dry-run] {}", outcome.result);
    println!("[dry-run] expected fee: {}", outcome.fee);
}

impl Command {
    /// Execute the command. When `dry_run` is set, simulate instead of submitting.
    pub fn run(&self) {
        match self {
            Command::Register { id, dry_run } => {
                if *dry_run {
                    print_dry_run(&simulate("register", Some(id)));
                } else {
                    submit("register", Some(id));
                }
            }
            Command::Attest { id, dry_run } => {
                if *dry_run {
                    print_dry_run(&simulate("attest", Some(id)));
                } else {
                    submit("attest", Some(id));
                }
            }
            Command::Revoke { id, dry_run } => {
                if *dry_run {
                    print_dry_run(&simulate("revoke", Some(id)));
                } else {
                    submit("revoke", Some(id));
                }
            }
            Command::Pause { dry_run } => {
                if *dry_run {
                    print_dry_run(&simulate("pause", None));
                } else {
                    submit("pause", None);
                }
            }
        }
    }
}

/// Submit a state-changing transaction to the contract.
fn submit(action: &str, id: Option<&str>) {
    match id {
        Some(id) => println!("submitted {action} for '{id}'"),
        None => println!("submitted {action}"),
    }
}
