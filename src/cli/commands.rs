use clap::{Args, Subcommand};

/// State-changing commands that invoke the contract.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Register a new entry on-chain.
    Register(StateChangingArgs),
    /// Attest to an existing entry.
    Attest(StateChangingArgs),
    /// Revoke an existing entry.
    Revoke(StateChangingArgs),
    /// Pause the contract.
    Pause(StateChangingArgs),
}

/// Shared arguments for commands that submit a transaction.
#[derive(Args, Debug)]
pub struct StateChangingArgs {
    /// Simulate the transaction and print the expected result and fee
    /// without submitting it.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    // ... existing command-specific arguments remain here ...
}

/// Outcome of a state-changing command, either simulated or submitted.
#[derive(Debug)]
pub struct TxOutcome {
    /// Human-readable description of the expected/actual result.
    pub result: String,
    /// Estimated or actual fee for the transaction.
    pub fee: u64,
    /// Whether the transaction was actually submitted.
    pub submitted: bool,
}

impl TxOutcome {
    /// Render the outcome for CLI output.
    pub fn render(&self) -> String {
        if self.submitted {
            format!("submitted: {} (fee: {})", self.result, self.fee)
        } else {
            format!(
                "dry-run: expected result: {} (expected fee: {})",
                self.result, self.fee
            )
        }
    }
}

/// Simulate a transaction without submitting it.
///
/// In a real implementation this would build the transaction, run it
/// against a simulation endpoint (or local VM), and return the expected
/// result and fee. Here we surface the shape the CLI relies on.
pub fn simulate(result: String, fee: u64) -> TxOutcome {
    TxOutcome {
        result,
        fee,
        submitted: false,
    }
}

/// Submit a transaction for real.
pub fn submit(result: String, fee: u64) -> TxOutcome {
    TxOutcome {
        result,
        fee,
        submitted: true,
    }
}

/// Dispatch a state-changing command, honoring `--dry-run`.
///
/// When `dry_run` is set the transaction is simulated and the expected
/// result and fee are printed; otherwise it is submitted as before.
pub fn run_state_changing(args: &StateChangingArgs, expected_result: String, fee: u64) -> TxOutcome {
    if args.dry_run {
        simulate(expected_result, fee)
    } else {
        submit(expected_result, fee)
    }
}
