//! `anchorkit quote` — dry-run transaction routing and print the selected quote.

use crate::output::{print_ascii_table, validate_output_format};
use crate::settings::resolve_network;
use crate::soroban_rpc::{route_quote_rpc, QuoteCandidate};

const VALID_STRATEGIES: &[&str] = &[
    "LowestFee",
    "FastestSettlement",
    "HighestReputation",
    "Balanced",
    "Weighted",
];

fn validate_strategy_arg(strategy: &str) -> Result<(), String> {
    if VALID_STRATEGIES.contains(&strategy) {
        Ok(())
    } else {
        Err(format!(
            "invalid strategy '{strategy}'; expected one of: {}",
            VALID_STRATEGIES.join(", ")
        ))
    }
}

/// Build the JSON payload for a routing dry run.
fn quote_json_value(
    selected_anchor: &str,
    candidates: &[QuoteCandidate],
    show_all: bool,
    strategy: &str,
) -> serde_json::Value {
    serde_json::json!({
        "strategy": strategy,
        "selected_anchor": selected_anchor,
        "selected_quote": candidates.iter().find(|candidate| candidate.anchor == selected_anchor),
        "candidates": if show_all { candidates } else { &[] },
    })
}

/// Render the result of a routing dry run. Kept separate from RPC I/O so the
/// three output formats can be tested offline.
fn render_quote(
    selected_anchor: &str,
    candidates: &[QuoteCandidate],
    show_all: bool,
    strategy: &str,
    output: &str,
) {
    let selected = candidates
        .iter()
        .find(|candidate| candidate.anchor == selected_anchor);
    match output {
        "json" => {
            let value = quote_json_value(selected_anchor, candidates, show_all, strategy);
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
        }
        "table" => {
            println!("💱 Routing dry run ({strategy})\n");
            println!("Selected anchor: {selected_anchor}\n");
            let visible: Vec<&QuoteCandidate> = if show_all {
                candidates.iter().collect()
            } else {
                selected.into_iter().collect()
            };
            let headers = &[
                "Quote", "Anchor", "Pair", "Rate", "Fee (bps)", "Min", "Max", "Valid until",
            ];
            let rows: Vec<Vec<String>> = visible
                .iter()
                .map(|candidate| {
                    vec![
                        candidate.quote_id.to_string(),
                        candidate.anchor.clone(),
                        format!("{}/{}", candidate.base_asset, candidate.quote_asset),
                        candidate.rate.to_string(),
                        candidate.fee_percentage.to_string(),
                        candidate.minimum_amount.to_string(),
                        candidate.maximum_amount.to_string(),
                        candidate.valid_until.to_string(),
                    ]
                })
                .collect();
            print_ascii_table(headers, &rows);
        }
        _ => {
            println!("💱 Routing dry run ({strategy})");
            println!("  Selected anchor: {selected_anchor}");
            match selected {
                Some(quote) => {
                    println!("  Selected quote:  #{} ({} -> {})", quote.quote_id, quote.base_asset, quote.quote_asset);
                    println!("  Rate:            {}", quote.rate);
                    println!("  Fee:             {} bps", quote.fee_percentage);
                    println!("  Amount range:    {} - {}", quote.minimum_amount, quote.maximum_amount);
                    println!("  Valid until:     {}", quote.valid_until);
                }
                None => println!("  Selected quote:  unavailable"),
            }
            if show_all {
                println!("\n  Candidates:");
                for candidate in candidates {
                    let marker = if candidate.anchor == selected_anchor { "*" } else { " " };
                    println!(
                        "  {marker} #{} {} {} rate={} fee={}bps range={}-{} valid_until={}",
                        candidate.quote_id, candidate.anchor, candidate.base_asset,
                        candidate.rate, candidate.fee_percentage,
                        candidate.minimum_amount, candidate.maximum_amount, candidate.valid_until,
                    );
                }
            }
        }
    }
}

pub fn run_quote(
    base: &str,
    quote_asset: &str,
    amount: u64,
    strategy: &str,
    all_candidates: bool,
    output: &str,
    network_flag: Option<&str>,
) {
    validate_output_format(output);
    if let Err(error) = validate_strategy_arg(strategy) {
        eprintln!("❌ {error}");
        std::process::exit(1);
    }
    if amount == 0 {
        eprintln!("❌ Invalid amount '0'. --amount must be greater than 0");
        std::process::exit(1);
    }
    let _network = resolve_network(network_flag);

    match route_quote_rpc(base, quote_asset, amount, strategy) {
        Ok((selected, candidates)) => {
            render_quote(&selected, &candidates, all_candidates, strategy, output);
        }
        Err(error) => {
            eprintln!("❌ Could not dry-run routing: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quote(anchor: &str, fee: u32) -> QuoteCandidate {
        QuoteCandidate {
            quote_id: 7,
            anchor: anchor.to_string(),
            base_asset: "USDC".to_string(),
            quote_asset: "NGN".to_string(),
            rate: 1500,
            fee_percentage: fee,
            minimum_amount: 1,
            maximum_amount: 1_000_000,
            valid_until: 1_800_000_000,
        }
    }

    #[test]
    fn quote_strategy_is_validated() {
        assert!(validate_strategy_arg("LowestFee").is_ok());
        assert!(validate_strategy_arg("Weighted").is_ok());
        assert!(validate_strategy_arg("Cheapest").is_err());
    }

    #[test]
    fn quote_json_reports_selected_quote_and_optional_candidates() {
        let candidates = vec![quote("GSELECTED", 5), quote("GOTHER", 9)];

        let selected_only = quote_json_value("GSELECTED", &candidates, false, "LowestFee");
        assert_eq!(selected_only["selected_anchor"], "GSELECTED");
        assert_eq!(selected_only["selected_quote"]["fee_percentage"], 5);
        assert_eq!(selected_only["candidates"].as_array().unwrap().len(), 0);

        let with_candidates = quote_json_value("GSELECTED", &candidates, true, "LowestFee");
        assert_eq!(with_candidates["candidates"].as_array().unwrap().len(), 2);
    }
}
