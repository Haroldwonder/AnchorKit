#![cfg(feature = "proptest-tests")]
#![cfg(test)]

//! Property-based tests for `select_routing_quote` (issue #1534).
//!
//! Generates random anchors, quotes and routing options and asserts the
//! routing invariants:
//!
//!   - the selected quote is never expired, inactive, below `min_reputation`,
//!     or outside the amount range
//!   - `LowestFee` picks a quote with fee <= every other eligible quote
//!   - dry run and real run select the same anchor
//!
//! Gated behind the `proptest-tests` feature so normal CI stays fast.

use ed25519_dalek::SigningKey;
use proptest::prelude::*;
use rand::rngs::OsRng;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, String, Vec, symbol_short,
};

use crate::contract::{AnchorKitContract, AnchorKitContractClient};
use crate::sep10_test_util::register_attestor_with_sep10;
use crate::types::{RoutingOptions, RoutingRequest, SERVICE_QUOTES};

fn make_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

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

fn setup_client(env: &Env) -> AnchorKitContractClient {
    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin, &100_u64, &None, &None);
    client
}

#[derive(Clone, Debug)]
struct AnchorCfg {
    active: bool,
    reputation: u32,
    min_amount: u64,
    max_amount: u64,
    fee: u32,
    valid_offset: i64,
}

fn anchor_cfg() -> impl Strategy<Value = AnchorCfg> {
    (
        any::<bool>(),
        0u32..10_000u32,
        0u64..100_000u64,
        0u64..100_000u64,
        0u32..10_000u32,
        -600i64..600i64,
    )
        .prop_map(
            |(active, reputation, min_amount, max_amount, fee, valid_offset)| AnchorCfg {
                active,
                reputation,
                min_amount,
                max_amount: max_amount.max(min_amount),
                fee,
                valid_offset,
            },
        )
}

fn strategy() -> impl Strategy<Value = (Vec<AnchorCfg>, u32, u64)> {
    (
        prop::collection::vec(anchor_cfg(), 1..6),
        0u32..10_000u32,
        0u64..100_000u64,
    )
}

fn eligible(cfg: &AnchorCfg, now: u64, min_reputation: u32, amount: u64) -> bool {
    let valid_until = (now as i64 + cfg.valid_offset).max(0) as u64;
    cfg.active
        && cfg.reputation >= min_reputation
        && valid_until > now
        && amount >= cfg.min_amount
        && amount <= cfg.max_amount
}

fn register_and_quote(
    env: &Env,
    client: &AnchorKitContractClient,
    anchor: &Address,
    cfg: &AnchorCfg,
    now: u64,
) {
    let signing_key = SigningKey::generate(&mut OsRng);
    register_attestor_with_sep10(env, client, anchor, anchor, &signing_key);
    let mut services = Vec::new(env);
    services.push_back(SERVICE_QUOTES);
    client.configure_services(anchor, &services);
    client.set_anchor_metadata(
        anchor,
        &cfg.reputation,
        &300u64,
        &10_000u32,
        &5_000u32,
        &0u64,
        &None,
    );
    client.update_health_status(anchor, &100u64, &0u32, &100u32);
    let valid_until = (now as i64 + cfg.valid_offset).max(0) as u64;
    client.submit_quote(
        anchor,
        &String::from_str(env, "USD"),
        &String::from_str(env, "USDC"),
        &1u64,
        &cfg.fee,
        &cfg.min_amount,
        &cfg.max_amount,
        &valid_until,
    );
}

#[proptest::test]
fn routing_selection_invariants(
    input in strategy().prop_filter("at least one eligible anchor", |(cfgs, min_reputation, amount)| {
        let now = 2_000_000_000u64;
        cfgs.iter().any(|c| eligible(c, now, *min_reputation, *amount))
    }),
) {
    let (cfgs, min_reputation, amount) = input;
    let now = 2_000_000_000u64;
    let env = make_env();
    set_ledger(&env, now);
    let client = setup_client(&env);

    let mut anchors = Vec::new(&env);
    for cfg in &cfgs {
        let anchor = Address::generate(&env);
        anchors.push_back(anchor);
        register_and_quote(&env, &client, &anchor, cfg, now);
    }

    let strategy_sym = soroban_sdk::Symbol::new(&env, "LowestFee");
    let mut strategy = Vec::new(&env);
    strategy.push_back(strategy_sym);
    let options = RoutingOptions {
        request: RoutingRequest {
            base_asset: String::from_str(&env, "USD"),
            quote_asset: String::from_str(&env, "USDC"),
            amount,
            operation_type: 1,
        },
        strategy,
        min_reputation,
        max_anchors: 0,
        require_kyc: false,
        jurisdiction: None,
        fallback_chain: Vec::new(&env),
    };

    // Real run.
    let quote = client.route_transaction(&options);
    let selected_idx = anchors
        .iter()
        .position(|a| a == &quote.anchor)
        .expect("selected anchor must be registered");
    let selected_cfg = &cfgs[selected_idx];

    // Invariant 1: never expired, inactive, below min_reputation, or outside amount range.
    prop_assert!(
        eligible(selected_cfg, now, min_reputation, amount),
        "selected anchor ineligible: {:?} vs min_reputation={} amount={}",
        selected_cfg,
        min_reputation,
        amount
    );

    // Invariant 2: LowestFee fee <= every other eligible quote.
    let eligible_fees: Vec<u32> = anchors
        .iter()
        .zip(cfgs.iter())
        .filter(|(_, cfg)| eligible(cfg, now, min_reputation, amount))
        .map(|(_, cfg)| cfg.fee)
        .collect();
    prop_assert!(
        eligible_fees.iter().all(|f| selected_cfg.fee <= *f),
        "LowestFee selected fee {} not <= all eligible fees {:?}",
        selected_cfg.fee,
        eligible_fees
    );

    // Invariant 3: dry run and real run select the same anchor.
    let dry_run_anchor = client.route_transaction_dry_run(&options);
    prop_assert_eq!(dry_run_anchor, quote.anchor);
}
