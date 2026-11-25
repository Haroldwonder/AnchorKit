use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use keyring::Entry;

const SECRET_KEY_ENV: &str = "ANCHORKIT_SECRET_KEY";
const KEYRING_SERVICE: &str = "anchorkit";
const KEYRING_USER: &str = "secret-key";

/// Command-line configuration for Anchorkit.
#[derive(Debug, Parser)]
#[command(name = "anchorkit", version, about)]
pub struct Config {
    /// Secret key passed directly as a flag (insecure: visible in shell history and process lists).
    #[arg(long, value_name = "KEY")]
    pub secret_key: Option<String>,

    /// Read the secret key from a file instead of a flag.
    #[arg(long, value_name = "PATH")]
    pub secret_file: Option<PathBuf>,

    /// Read the secret key from the OS keyring instead of a flag.
    #[arg(long)]
    pub secret_keyring: bool,
}

impl Config {
    /// Resolve the secret key from the most secure available source.
    ///
    /// Precedence: `--secret-file`, OS keyring, `ANCHORKIT_SECRET_KEY`, then the
    /// `--secret-key` flag (which emits a warning because it is insecure).
    pub fn resolve_secret_key(&self) -> Result<String> {
        if let Some(path) = &self.secret_file {
            let contents = fs::read_to_string(path)
                .with_context(|| format!("failed to read secret file {}", path.display()))?;
            let key = contents.trim();
            if key.is_empty() {
                return Err(anyhow!("secret file {} is empty", path.display()));
            }
            return Ok(key.to_string());
        }

        if self.secret_keyring {
            return read_keyring_secret();
        }

        if let Ok(key) = env::var(SECRET_KEY_ENV) {
            let key = key.trim();
            if !key.is_empty() {
                return Ok(key.to_string());
            }
        }

        if let Some(key) = &self.secret_key {
            eprintln!(
                "warning: passing the secret key via --secret-key exposes it in shell history and process lists; \
                 prefer {SECRET_KEY_ENV}, --secret-file, or --secret-keyring"
            );
            return Ok(key.clone());
        }

        Err(anyhow!(
            "no secret key provided; set {SECRET_KEY_ENV}, use --secret-file, --secret-keyring, or --secret-key"
        ))
    }
}

fn read_keyring_secret() -> Result<String> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .context("failed to open OS keyring entry")?;
    let key = entry
        .get_password()
        .context("failed to read secret key from OS keyring")?;
    let key = key.trim();
    if key.is_empty() {
        return Err(anyhow!("secret key in OS keyring is empty"));
    }
    Ok(key.to_string())
}
