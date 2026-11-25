use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Anchorkit command-line interface.
#[derive(Parser, Debug)]
#[command(name = "anchorkit", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Secret key passed directly as a flag (insecure: visible in shell history and process lists).
    #[arg(long, global = true, env = "ANCHORKIT_SECRET_KEY", hide_env_values = true)]
    pub secret_key: Option<String>,

    /// Read the secret key from a file instead of passing it as a flag.
    #[arg(long, global = true, value_name = "PATH")]
    pub secret_file: Option<PathBuf>,

    /// Read the secret key from the OS keyring (service "anchorkit", user "default").
    #[arg(long, global = true)]
    pub secret_keyring: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize a new anchorkit project.
    Init,
    /// Run the anchorkit daemon.
    Run,
}

impl Cli {
    /// Resolve the secret key from the configured source, preferring secure sources.
    ///
    /// Order of precedence: `--secret-file`, OS keyring, then the `--secret-key`
    /// flag / `ANCHORKIT_SECRET_KEY` environment variable.
    pub fn resolve_secret_key(&self) -> Result<Option<String>, SecretKeyError> {
        if let Some(path) = &self.secret_file {
            let contents = std::fs::read_to_string(path).map_err(|e| SecretKeyError::File {
                path: path.clone(),
                source: e,
            })?;
            return Ok(Some(contents.trim().to_string()));
        }

        if self.secret_keyring {
            return read_keyring_secret().map(Some);
        }

        if self.secret_key.is_some() {
            eprintln!(
                "warning: passing a secret key via --secret-key or ANCHORKIT_SECRET_KEY is insecure; \
                 prefer --secret-file or --secret-keyring"
            );
        }

        Ok(self.secret_key.clone())
    }
}

/// Errors that can occur while resolving the secret key.
#[derive(Debug)]
pub enum SecretKeyError {
    File {
        path: PathBuf,
        source: std::io::Error,
    },
    Keyring(String),
}

impl std::fmt::Display for SecretKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretKeyError::File { path, source } => {
                write!(f, "failed to read secret file {}: {source}", path.display())
            }
            SecretKeyError::Keyring(msg) => write!(f, "failed to read secret from keyring: {msg}"),
        }
    }
}

impl std::error::Error for SecretKeyError {}

/// Read the secret key from the OS keyring.
fn read_keyring_secret() -> Result<String, SecretKeyError> {
    let entry = keyring::Entry::new("anchorkit", "default")
        .map_err(|e| SecretKeyError::Keyring(e.to_string()))?;
    entry
        .get_password()
        .map_err(|e| SecretKeyError::Keyring(e.to_string()))
}
