use anyhow::Result;
use clap::Subcommand;
use serde_json::json;

use crate::client::Sep10Client;
use crate::output::{OutputFormat, print_json, print_table, print_text};

#[derive(Debug, Subcommand)]
pub enum AttestorsCommand {
    /// List all attestors
    List {
        /// Number of attestors to skip
        #[arg(long, default_value_t = 0)]
        offset: u32,
        /// Maximum number of attestors to return
        #[arg(long)]
        limit: Option<u32>,
    },
}

pub async fn handle(command: AttestorsCommand, client: &Sep10Client, format: OutputFormat) -> Result<()> {
    match command {
        AttestorsCommand::List { offset, limit } => list(client, offset, limit, format).await,
    }
}

async fn list(client: &Sep10Client, offset: u32, limit: Option<u32>, format: OutputFormat) -> Result<()> {
    let attestors = client.get_all_attestors(offset, limit).await?;

    match format {
        OutputFormat::Json => {
            let value = json!(attestors
                .iter()
                .map(|a| json!({
                    "name": a.name,
                    "endpoint": a.endpoint,
                    "services": a.services,
                }))
                .collect::<Vec<_>>());
            print_json(&value)?;
        }
        OutputFormat::Table => {
            let rows: Vec<Vec<String>> = attestors
                .iter()
                .map(|a| {
                    vec![
                        a.name.clone(),
                        a.endpoint.clone(),
                        a.services.join(", "),
                    ]
                })
                .collect();
            print_table(&["Name", "Endpoint", "Services"], &rows);
        }
        OutputFormat::Text => {
            for a in &attestors {
                print_text(&format!(
                    "{} - {} [{}]",
                    a.name,
                    a.endpoint,
                    a.services.join(", ")
                ));
            }
        }
    }

    Ok(())
}
