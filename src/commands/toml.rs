use anyhow::{anyhow, Context, Result};
use clap::Parser;
use stellar_anchor_sdk::stellar_toml::StellarToml;

/// Fetch and validate an anchor's stellar.toml.
#[derive(Parser, Debug)]
pub struct TomlCommand {
    /// Anchor domain, e.g. `example.com`.
    pub domain: String,
}

impl TomlCommand {
    pub async fn run(&self) -> Result<()> {
        let domain = self.domain.trim();
        if domain.is_empty() {
            return Err(anyhow!("domain must not be empty"));
        }

        let url = format!("https://{domain}/.well-known/stellar.toml");
        println!("Fetching {url}");

        let toml = StellarToml::from_url(&url)
            .await
            .with_context(|| format!("failed to fetch or parse stellar.toml from {url}"))?;

        println!("\nAnchor: {}", toml.documentation.organization.unwrap_or_default());

        // Signing key
        match toml.signing_key.as_deref() {
            Some(key) if !key.is_empty() => println!("Signing key: {key}"),
            _ => println!("Signing key: MISSING"),
        }

        // Network passphrase
        match toml.network_passphrase.as_deref() {
            Some(passphrase) if !passphrase.is_empty() => {
                println!("Network passphrase: {passphrase}");
                if passphrase != stellar_anchor_sdk::DEFAULT_NETWORK_PASSPHRASE {
                    println!("  WARNING: passphrase does not match the default network");
                }
            }
            _ => println!("Network passphrase: MISSING"),
        }

        // SEP endpoints
        println!("\nSEP endpoints:");
        report_endpoint("SEP-1  (transfer server)", toml.transfer_server.as_deref());
        report_endpoint("SEP-10 (web auth)", toml.web_auth_endpoint.as_deref());
        report_endpoint("SEP-24 (interactive)", toml.transfer_server.as_deref());
        report_endpoint("SEP-31 (direct payments)", toml.direct_payment_server.as_deref());
        report_endpoint("SEP-38 (quotes)", toml.anchor_quote_server.as_deref());

        // Supported assets
        println!("\nSupported assets:");
        if toml.currencies.is_empty() {
            println!("  (none declared)");
        } else {
            for currency in &toml.currencies {
                println!("  {} ({})", currency.code, currency.issuer.as_deref().unwrap_or("-"));
            }
        }

        Ok(())
    }
}

fn report_endpoint(label: &str, value: Option<&str>) {
    match value {
        Some(endpoint) if !endpoint.is_empty() => println!("  {label}: {endpoint}"),
        _ => println!("  {label}: MISSING"),
    }
}
