use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "anchorkit", version, about = "AnchorKit CLI")]
struct Cli {
    /// Secret key passed directly as a flag (insecure; prefer env, --secret-file, or keyring)
    #[arg(long, global = true, env = "ANCHORKIT_SECRET_KEY", hide_env_values = true)]
    secret_key: Option<String>,

    /// Read the secret key from a file
    #[arg(long, global = true, value_name = "PATH")]
    secret_file: Option<PathBuf>,

    /// Read the secret key from the OS keyring
    #[arg(long, global = true)]
    secret_keyring: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the resolved secret key source (does not print the secret)
    Status,
}

/// Resolve the secret key from the available sources, in priority order:
/// --secret-file, --secret-keyring, then the flag/env value.
fn resolve_secret(cli: &Cli) -> Result<String> {
    if let Some(path) = &cli.secret_file {
        let mut contents = String::new();
        fs::File::open(path)
            .with_context(|| format!("failed to open secret file {}", path.display()))?
            .read_to_string(&mut contents)
            .with_context(|| format!("failed to read secret file {}", path.display()))?;
        let trimmed = contents.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("secret file {} is empty", path.display()));
        }
        return Ok(trimmed.to_string());
    }

    if cli.secret_keyring {
        let entry = keyring::Entry::new("anchorkit", "secret_key")
            .context("failed to open OS keyring entry")?;
        return entry
            .get_password()
            .context("failed to read secret key from OS keyring");
    }

    if let Some(secret) = &cli.secret_key {
        if std::env::var_os("ANCHORKIT_SECRET_KEY").is_some() {
            return Ok(secret.clone());
        }
        eprintln!(
            "warning: passing a secret key as a command-line flag exposes it in shell history and process lists; prefer ANCHORKIT_SECRET_KEY, --secret-file, or --secret-keyring"
        );
        return Ok(secret.clone());
    }

    Err(anyhow!(
        "no secret key provided; set ANCHORKIT_SECRET_KEY, use --secret-file, or use --secret-keyring"
    ))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Command::Status => {
            let secret = resolve_secret(&cli)?;
            println!("secret key resolved ({} bytes)", secret.len());
        }
    }

    Ok(())
}
