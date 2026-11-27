#![cfg(test)]

//! Tests for quote cancellation (#1508), `get_latest_quote` (#1509),
//! `list_quotes` (#1510) and the asset-pair routing index (#1511).

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger, LedgerInfo},
    Address, Env, String, Symbol, Vec,
};

use crate::contract::{AnchorKitContract, AnchorKitContractClient};
use crate::errors::ErrorCode;
use crate::sep10_test_util::register_attestor_with_sep10;
use crate::types::{RoutingOptions, RoutingRequest};

fn set_ledger(env: &Env, timestamp: u64) {
    env.ledger().set(LedgerInfo {
        timestamp,
        protocol_version: 21,
        sequence_number: 0,
        network_id: Default::default(),
        base_reserve: 0,
        min_persistent_entry_ttl: 4096,
        min_temp_entry_ttl: 16,
        max_entry_ttl: 6312000,
    });
}

fn setup(env: &Env) -> AnchorKitContractClient<'_> {
    env.mock_all_auths();
    set_ledger(env, 1_000_000);
    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin, &100_u64, &None, &None);
    client
}

fn register_anchor(env: &Env, client: &AnchorKitContractClient) -> Address {
    let anchor = Address::generate(env);
    let signing_key = SigningKey::generate(&mut OsRng);
    register_attestor_with_sep10(env, client, &anchor, &anchor, &signing_key);
    let mut services = Vec::new(env);
    services.push_back(1u32);
    services.push_back(3u32);
    client.configure_services(&anchor, &services);
    client.set_anchor_metadata(&anchor, &5000u32, &300u64, &7500u32, &9900u32, &1_000_000u64, &None);
    anchor
}

fn quote(env: &Env, client: &AnchorKitContractClient, anchor: &Address, base: &str, q: &str, fee: u32, until: u64) -> u64 {
    client.submit_quote(
        anchor,
        &String::from_str(env, base),
        &String::from_str(env, q),
        &10000u64,
        &fee,
        &100u64,
        &100000u64,
        &until,
    )
}

fn options(env: &Env, base: &str, q: &str) -> RoutingOptions {
    let mut strategy = Vec::new(env);
    strategy.push_back(Symbol::new(env, "LowestFee"));
    RoutingOptions {
        request: RoutingRequest {
            base_asset: String::from_str(env, base),
            quote_asset: String::from_str(env, q),
            amount: 5000,
            operation_type: 1,
        },
        strategy,
        min_reputation: 0,
        max_anchors: 10,
        require_kyc: false,
        jurisdiction: None,
        fallback_chain: Vec::new(env),
    }
}

fn err(code: ErrorCode) -> Result<soroban_sdk::Error, soroban_sdk::InvokeError> {
    Ok(soroban_sdk::Error::from_contract_error(code as u32))
}

// --- #1508 ---------------------------------------------------------------

#[test]
fn cancelled_quote_is_rejected_by_receive_quote() {
    let env = Env::default();
    let client = setup(&env);
    let anchor = register_anchor(&env, &client);
    let id = quote(&env, &client, &anchor, "USD", "USDC", 10, 1_003_600);

    client.cancel_quote(&anchor, &id);

    let receiver = Address::generate(&env);
    let res = client.try_receive_quote(&receiver, &anchor, &id);
    assert_eq!(res.err().unwrap(), err(ErrorCode::QuoteCancelled));
}

#[test]
fn cancel_quote_emits_event() {
    let env = Env::default();
    let client = setup(&env);
    let anchor = register_anchor(&env, &client);
    let id = quote(&env, &client, &anchor, "USD", "USDC", 10, 1_003_600);
    let before = env.events().all().len();
    client.cancel_quote(&anchor, &id);
    assert_eq!(env.events().all().len(), before + 1);
}

#[test]
fn cancel_twice_or_unknown_fails() {
    let env = Env::default();
    let client = setup(&env);
    let anchor = register_anchor(&env, &client);
    let id = quote(&env, &client, &anchor, "USD", "USDC", 10, 1_003_600);
    client.cancel_quote(&anchor, &id);
    assert_eq!(client.try_cancel_quote(&anchor, &id).err().unwrap(), err(ErrorCode::QuoteCancelled));
    assert_eq!(client.try_cancel_quote(&anchor, &999).err().unwrap(), err(ErrorCode::QuoteNotFound));
}

#[test]
fn cancelled_quote_is_skipped_by_routing() {
    let env = Env::default();
    let client = setup(&env);
    let cheap = register_anchor(&env, &client);
    let pricey = register_anchor(&env, &client);
    let cheap_id = quote(&env, &client, &cheap, "USD", "USDC", 5, 1_003_600);
    quote(&env, &client, &pricey, "USD", "USDC", 50, 1_003_600);

    assert_eq!(client.route_transaction(&options(&env, "USD", "USDC")).anchor, cheap);
    client.cancel_quote(&cheap, &cheap_id);
    assert_eq!(client.route_transaction(&options(&env, "USD", "USDC")).anchor, pricey);

    client.cancel_quote(&pricey, &2);
    let res = client.try_route_transaction(&options(&env, "USD", "USDC"));
    assert_eq!(res.err().unwrap(), err(ErrorCode::NoQuotesAvailable));
}

// --- #1509 ---------------------------------------------------------------

#[test]
fn get_latest_quote_returns_most_recent() {
    let env = Env::default();
    let client = setup(&env);
    let anchor = register_anchor(&env, &client);
    assert!(client.get_latest_quote(&anchor).is_none());
    quote(&env, &client, &anchor, "USD", "USDC", 10, 1_003_600);
    let id2 = quote(&env, &client, &anchor, "EUR", "USDC", 20, 1_003_600);
    let latest = client.get_latest_quote(&anchor).unwrap();
    assert_eq!(latest.quote_id, id2);
    assert_eq!(latest.fee_percentage, 20);
}

// --- #1510 ---------------------------------------------------------------

#[test]
fn list_quotes_paginates_and_prunes() {
    let env = Env::default();
    let client = setup(&env);
    let anchor = register_anchor(&env, &client);
    let a = quote(&env, &client, &anchor, "USD", "USDC", 1, 1_003_600);
    let b = quote(&env, &client, &anchor, "USD", "USDC", 2, 1_000_500);
    let c = quote(&env, &client, &anchor, "USD", "USDC", 3, 1_003_600);

    let all = client.list_quotes(&anchor, &0, &10);
    assert_eq!(all.len(), 3);
    let page = client.list_quotes(&anchor, &1, &1);
    assert_eq!(page.len(), 1);
    assert_eq!(page.get(0).unwrap().quote_id, b);

    client.cancel_quote(&anchor, &a);
    set_ledger(&env, 1_001_000); // `b` expires
    let live = client.list_quotes(&anchor, &0, &10);
    assert_eq!(live.len(), 1);
    assert_eq!(live.get(0).unwrap().quote_id, c);
}

#[test]
fn list_quotes_respects_max_page_size() {
    let env = Env::default();
    let client = setup(&env);
    let anchor = register_anchor(&env, &client);
    for i in 0..5u32 {
        quote(&env, &client, &anchor, "USD", "USDC", i, 1_003_600);
    }
    client.set_max_page_size(&2);
    assert_eq!(client.list_quotes(&anchor, &0, &100).len(), 2);
    assert_eq!(client.list_quotes(&anchor, &0, &0).len(), 0);
}

// --- #1511 ---------------------------------------------------------------

#[test]
fn pair_index_tracks_submit_cancel_and_expiry() {
    let env = Env::default();
    let client = setup(&env);
    let a1 = register_anchor(&env, &client);
    let a2 = register_anchor(&env, &client);
    let a3 = register_anchor(&env, &client);
    let usd = String::from_str(&env, "USD");
    let usdc = String::from_str(&env, "USDC");
    let eur = String::from_str(&env, "EUR");

    quote(&env, &client, &a1, "USD", "USDC", 10, 1_003_600);
    quote(&env, &client, &a2, "USD", "USDC", 20, 1_000_500);
    quote(&env, &client, &a3, "USD", "USDC", 30, 1_003_600);
    assert_eq!(client.get_pair_anchors(&usd, &usdc).len(), 3);

    // a1 moves to a different pair (quote id 4).
    quote(&env, &client, &a1, "EUR", "USDC", 10, 1_003_600);
    assert_eq!(client.get_pair_anchors(&usd, &usdc).len(), 2);
    assert_eq!(client.get_pair_anchors(&eur, &usdc).len(), 1);

    // Routing only sees anchors quoting the requested pair.
    assert_eq!(client.route_transaction(&options(&env, "USD", "USDC")).anchor, a2);
    assert_eq!(client.route_transaction(&options(&env, "EUR", "USDC")).anchor, a1);

    // Expiry prunes a2 from the index on the next routing pass.
    set_ledger(&env, 1_001_000);
    assert_eq!(client.route_transaction(&options(&env, "USD", "USDC")).anchor, a3);
    let remaining = client.get_pair_anchors(&usd, &usdc);
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining.get(0).unwrap(), a3);

    // Cancelling the latest quote removes the anchor from its pair.
    client.cancel_quote(&a1, &4);
    assert_eq!(client.get_pair_anchors(&eur, &usdc).len(), 0);
}

#[test]
fn removed_anchor_is_not_routed_via_pair_index() {
    let env = Env::default();
    let client = setup(&env);
    let a1 = register_anchor(&env, &client);
    quote(&env, &client, &a1, "USD", "USDC", 10, 1_003_600);
    client.remove_anchor(&a1);
    assert!(client.try_route_transaction(&options(&env, "USD", "USDC")).is_err());
}
