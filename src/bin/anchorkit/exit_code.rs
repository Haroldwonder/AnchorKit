//! Process exit codes (#1609 / issue #1496), kept in sync with the README.

pub use codes::*;

/// Process exit codes used by the CLI so scripts can tell failure classes
/// apart. Keep in sync with `EXIT_CODES_HELP` and the README.
pub mod codes {
    /// Generic / unexpected failure (build or test failure, I/O error, ...).
    pub const GENERIC: i32 = 1;
    /// Invalid usage or failed validation (bad flag value, malformed address,
    /// config file that fails validation). Clap also uses 2 for parse errors.
    pub const USAGE: i32 = 2;
    /// Required configuration is missing (contract ID, signing key, config path).
    pub const CONFIG: i32 = 3;
    /// Network or RPC failure (endpoint unreachable, request rejected, timeout).
    pub const NETWORK: i32 = 4;
    /// The contract returned an error or the requested record does not exist.
    pub const CONTRACT: i32 = 5;
}

pub const EXIT_CODES_HELP: &str = "\
Exit codes:
  0  Success
  1  Generic failure (build/test failure, I/O error)
  2  Usage or validation error (invalid arguments, invalid config file)
  3  Configuration missing (e.g. ANCHORKIT_CONTRACT_ID, STELLAR_SECRET_KEY)
  4  Network or RPC failure
  5  Contract error or record not found

Output:
  Emoji and box-drawing decorations are replaced with plain ASCII when
  --plain/--no-emoji is passed, NO_COLOR is set, or the stream is not a TTY.";

/// Classify an error message coming from the Soroban RPC helpers into one of
/// the documented exit codes.
pub fn exit_code_for_rpc_error(msg: &str) -> i32 {
    let m = msg.to_ascii_lowercase();
    if m.contains(" not set")
        || m.contains("invalid secret key")
        || m.contains("invalid contract address")
    {
        codes::CONFIG
    } else if m.contains("invalid stellar account address")
        || m.contains("must match the account")
        || m.contains("exceeds the maximum")
    {
        codes::USAGE
    } else if m.contains("contract returned")
        || m.contains("not found")
        || m.contains("failed on-chain")
        || m.contains("simulation failed")
        || m.contains("unexpected value")
        || m.contains("without a return value")
    {
        codes::CONTRACT
    } else {
        codes::NETWORK
    }
}

/// Returns `true` when no contract ID is configured (flag, env or config file).
pub fn contract_id_missing() -> bool {
    !matches!(crate::settings::contract_id(), Ok(Some(_)))
}
