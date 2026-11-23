//! Integration test for the full SEP-6 deposit lifecycle.
//!
//! This test exercises the complete deposit lifecycle that a real Stellar testnet
//! anchor would expose:
//!
//! 1. **SEP-10 authentication** — generate a keypair, register a verifying key,
//!    and obtain a signed JWT.
//! 2. **Anchor registration** — register the anchor on-chain and configure it
//!    for the Deposits service.
//! 3. **Deposit initiation** — call `sep6::initiate_deposit` with a raw anchor
//!    response (simulating what the anchor's `/deposit` HTTP endpoint returns).
//! 4. **Transaction status polling** — step the transaction through every
//!    meaningful SEP-6 status transition:
//!    `incomplete → pending_external → pending_anchor → pending_user → completed`.
//! 5. **Completion assertions** — verify the final normalized response matches
//!    expected values including fee propagation and status.
//!
//! The test is gated behind the `testnet-integration` feature flag so it is
//! excluded from the default CI matrix.  Enable it with:
//!
//! ```bash
//! cargo test --features testnet-integration
//! ```
//!
//! When connecting to the real Stellar testnet instead of the local Soroban
//! test environment, replace the `make_env()` / `setup()` helpers with a
//! `SorobanClient` pointed at `https://soroban-testnet.stellar.org` and swap
//! `RawDepositResponse` payloads for live HTTP responses from your anchor's
//! SEP-6 endpoint.

#![cfg(feature = "testnet-integration")]

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Bytes, Env, String, Vec,
};

use anchorkit::contract::{AnchorKitContract, AnchorKitContractClient};
use anchorkit::sep6::{
    fetch_transaction_status, get_transaction_status, initiate_deposit, list_transactions,
    RawDepositResponse, RawTransactionListRequest, RawTransactionResponse,
};
use anchorkit::types::{DepositResponse, TransactionStatus};

// ---------------------------------------------------------------------------
// Helpers shared across the lifecycle test stages
// ---------------------------------------------------------------------------

fn make_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn set_ledger(env: &Env, timestamp: u64) {
    env.ledger().set(LedgerInfo {
        timestamp,
        protocol_version: 21,
        sequence_number: 1,
        network_id: Default::default(),
        base_reserve: 0,
        min_persistent_entry_ttl: 4096,
        min_temp_entry_ttl: 16,
        max_entry_ttl: 6_312_000,
    });
}

/// Builds a minimal signed SEP-10 JWT for `sub` expiring at `exp`.
fn build_sep10_jwt(sk: &SigningKey, sub: &str, exp: u64) -> std::string::String {
    let header = r#"{"alg":"EdDSA","typ":"JWT"}"#;
    let payload = format!(r#"{{"sub":"{}","exp":{}}}"#, sub, exp);
    let h = URL_SAFE_NO_PAD.encode(header);
    let p = URL_SAFE_NO_PAD.encode(&payload);
    let msg = format!("{}.{}", h, p);
    let sig = sk.sign(msg.as_bytes());
    format!("{}.{}", msg, URL_SAFE_NO_PAD.encode(sig.to_bytes()))
}

/// Convenience macro so we can use `format!` inside a `#[no_std]`-compatible
/// crate boundary.  In integration tests `std` is always available, so this
/// just aliases `format!`.
macro_rules! alloc_format {
    ($($t:tt)*) => { format!($($t)*) };
}

/// Register `anchor` on the contract and configure it for the Deposits (1) service.
fn register_anchor(
    env: &Env,
    client: &AnchorKitContractClient,
    anchor: &Address,
    sk: &SigningKey,
) {
    // Publish the SEP-10 verifying key so the contract can validate JWTs.
    let vk_bytes = Bytes::from_slice(env, sk.verifying_key().as_bytes());
    client.set_sep10_jwt_verifying_key(anchor, &vk_bytes);
    client.add_attestation_key(anchor, &vk_bytes);

    // Build a JWT whose `sub` matches the anchor's on-chain address string.
    let sub_soroban = anchor.to_string();
    let mut sub_buf = [0u8; 128];
    let copy_len = (sub_soroban.len() as usize).min(128);
    sub_soroban.copy_into_slice(&mut sub_buf[..copy_len]);
    let sub_str = core::str::from_utf8(&sub_buf[..copy_len]).unwrap_or("");

    let exp = env.ledger().timestamp() + 86_400;
    let jwt_str = build_sep10_jwt(sk, sub_str, exp);
    let jwt = String::from_str(env, jwt_str.as_str());

    client.register_attestor(anchor, &jwt, anchor);

    // Enable the Deposits service for this anchor.
    let mut services = Vec::new(env);
    services.push_back(1u32); // SERVICE_DEPOSITS
    client.configure_services(anchor, &services);
}

// ---------------------------------------------------------------------------
// Simulated anchor HTTP responses for each lifecycle stage
// ---------------------------------------------------------------------------

/// Anchor returns `incomplete` immediately after the deposit is initiated —
/// the user has not yet provided all required KYC fields.
fn raw_deposit_incomplete(tx_id: &str) -> RawDepositResponse {
    RawDepositResponse {
        transaction_id: tx_id.to_string(),
        how: "Send 100 USDC to account GABC…".to_string(),
        extra_info: Some("Complete KYC before sending funds.".to_string()),
        min_amount: Some(10),
        max_amount: Some(50_000),
        fee_fixed: Some(1),
        fee_percent: Some(25), // 0.25 %
        status: Some("incomplete".to_string()),
        depositor_account: None,
    }
}

/// Anchor moves to `pending_external` once the user has submitted KYC and
/// the anchor is waiting for the on-chain Stellar payment.
fn raw_tx_pending_external(tx_id: &str) -> RawTransactionResponse {
    RawTransactionResponse {
        transaction_id: tx_id.to_string(),
        kind: Some("deposit".to_string()),
        status: "pending_external".to_string(),
        amount_in: Some(100),
        amount_out: None,
        amount_fee: None,
        message: Some("Waiting for your Stellar payment.".to_string()),
    }
}

/// Anchor moves to `pending_anchor` after detecting the on-chain payment.
fn raw_tx_pending_anchor(tx_id: &str) -> RawTransactionResponse {
    RawTransactionResponse {
        transaction_id: tx_id.to_string(),
        kind: Some("deposit".to_string()),
        status: "pending_anchor".to_string(),
        amount_in: Some(100),
        amount_out: None,
        amount_fee: Some(1),
        message: Some("Payment received. Processing deposit.".to_string()),
    }
}

/// Anchor moves to `pending_user` requesting additional confirmation.
fn raw_tx_pending_user(tx_id: &str) -> RawTransactionResponse {
    RawTransactionResponse {
        transaction_id: tx_id.to_string(),
        kind: Some("deposit".to_string()),
        status: "pending_user".to_string(),
        amount_in: Some(100),
        amount_out: Some(99),
        amount_fee: Some(1),
        message: Some("Please confirm your deposit details.".to_string()),
    }
}

/// Final state: anchor has completed the deposit.
fn raw_tx_completed(tx_id: &str) -> RawTransactionResponse {
    RawTransactionResponse {
        transaction_id: tx_id.to_string(),
        kind: Some("deposit".to_string()),
        status: "completed".to_string(),
        amount_in: Some(100),
        amount_out: Some(99),
        amount_fee: Some(1),
        message: None,
    }
}

// ---------------------------------------------------------------------------
// Full deposit lifecycle integration test
// ---------------------------------------------------------------------------

/// Runs the complete SEP-6 deposit lifecycle against the local Soroban test
/// environment, simulating what would happen against the Stellar testnet.
///
/// The test covers:
/// - SEP-10 authentication and anchor registration
/// - Deposit initiation with fee propagation
/// - Status polling through every meaningful lifecycle transition
/// - Final `completed` state assertions
#[test]
fn test_full_sep6_deposit_lifecycle() {
    // ── Stage 0: environment setup ────────────────────────────────────────
    let env = make_env();
    set_ledger(&env, 1_700_000_000); // representative testnet timestamp

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &500_u64, &None);

    // ── Stage 1: SEP-10 authentication & anchor registration ─────────────
    let anchor = Address::generate(&env);
    let sk = SigningKey::generate(&mut OsRng);
    register_anchor(&env, &client, &anchor, &sk);

    assert!(
        client.is_attestor(&anchor),
        "anchor must be registered on-chain after SEP-10 auth"
    );
    assert!(
        client.supports_service(&anchor, &1u32),
        "anchor must expose the Deposits service"
    );

    // ── Stage 2: deposit initiation ───────────────────────────────────────
    let tx_id = "sep6-testnet-txn-001";

    let deposit_result = initiate_deposit(raw_deposit_incomplete(tx_id), "USDC");
    assert!(deposit_result.is_ok(), "initiate_deposit should succeed: {:?}", deposit_result);

    let deposit = deposit_result.unwrap();
    assert_eq!(deposit.transaction_id, tx_id);
    assert_eq!(deposit.status, TransactionStatus::Incomplete,
        "initial status must be Incomplete");
    assert_eq!(deposit.fee_fixed, Some(1),
        "fixed fee must be propagated from raw response");
    assert_eq!(deposit.fee_percent, Some(25),
        "percentage fee must be propagated from raw response");
    assert_eq!(deposit.min_amount, Some(10));
    assert_eq!(deposit.max_amount, Some(50_000));

    // ── Stage 3a: status poll — pending_external ─────────────────────────
    let status_result = get_transaction_status(200, raw_tx_pending_external(tx_id));
    assert!(status_result.is_ok());
    let status = status_result.unwrap();
    assert_eq!(status.transaction_id, tx_id);
    assert_eq!(status.status, TransactionStatus::PendingExternal,
        "must transition to PendingExternal after on-chain payment is sent");
    assert_eq!(status.amount_in, Some(100));
    assert!(status.message.is_some(), "anchor should provide guidance message");

    // Simulate a non-200 response (e.g. testnet rate limit) and verify it's
    // handled gracefully before continuing.
    let throttled = get_transaction_status(429, raw_tx_pending_external(tx_id));
    assert!(throttled.is_err(), "429 must be surfaced as RateLimitExceeded error");

    // ── Stage 3b: status poll — pending_anchor ───────────────────────────
    let status_result = get_transaction_status(200, raw_tx_pending_anchor(tx_id));
    assert!(status_result.is_ok());
    let status = status_result.unwrap();
    assert_eq!(status.status, TransactionStatus::PendingAnchor,
        "must transition to PendingAnchor after payment is detected");
    assert_eq!(status.amount_fee, Some(1),
        "fee must be present once the anchor starts processing");

    // ── Stage 3c: status poll — pending_user ─────────────────────────────
    let status_result = get_transaction_status(200, raw_tx_pending_user(tx_id));
    assert!(status_result.is_ok());
    let status = status_result.unwrap();
    assert_eq!(status.status, TransactionStatus::PendingUser);
    assert_eq!(status.amount_out, Some(99),
        "amount_out must be populated before final confirmation");

    // ── Stage 3d: status poll — completed ────────────────────────────────
    let status_result = get_transaction_status(200, raw_tx_completed(tx_id));
    assert!(status_result.is_ok());
    let final_status = status_result.unwrap();
    assert_eq!(final_status.transaction_id, tx_id);
    assert_eq!(final_status.status, TransactionStatus::Completed,
        "lifecycle must end in Completed state");
    assert_eq!(final_status.amount_in, Some(100));
    assert_eq!(final_status.amount_out, Some(99));
    assert_eq!(final_status.amount_fee, Some(1));
    assert!(final_status.message.is_none(),
        "completed transactions typically have no pending-action message");

    // ── Stage 4: transaction history pagination ───────────────────────────
    // Simulate fetching the user's transaction history from the anchor API.
    let account = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWNA";
    let history = vec![
        raw_tx_completed(tx_id),
        // A second historical transaction for pagination testing.
        RawTransactionResponse {
            transaction_id: "sep6-testnet-txn-000".to_string(),
            kind: Some("deposit".to_string()),
            status: "completed".to_string(),
            amount_in: Some(50),
            amount_out: Some(49),
            amount_fee: Some(1),
            message: None,
        },
    ];

    let list_result = list_transactions(
        RawTransactionListRequest {
            account: account.to_string(),
            asset_code: "USDC".to_string(),
            limit: 10,
            cursor: None,
        },
        history.clone(),
    );
    assert!(list_result.is_ok());
    let txns = list_result.unwrap();
    assert_eq!(txns.len(), 2, "history must contain both transactions");
    assert_eq!(txns[0].transaction_id, tx_id);
    assert_eq!(txns[1].transaction_id, "sep6-testnet-txn-000");

    // Cursor-based pagination: fetch only transactions after the most recent one.
    let paginated = list_transactions(
        RawTransactionListRequest {
            account: account.to_string(),
            asset_code: "USDC".to_string(),
            limit: 10,
            cursor: Some(tx_id.to_string()),
        },
        history,
    )
    .unwrap();
    assert_eq!(paginated.len(), 1);
    assert_eq!(paginated[0].transaction_id, "sep6-testnet-txn-000",
        "cursor must skip the most-recent transaction");

    // ── Stage 5: 404 for unknown transaction ─────────────────────────────
    let not_found = get_transaction_status(
        404,
        RawTransactionResponse {
            transaction_id: "sep6-testnet-txn-MISSING".to_string(),
            kind: None,
            status: "".to_string(),
            amount_in: None,
            amount_out: None,
            amount_fee: None,
            message: None,
        },
    );
    assert!(not_found.is_err(), "unknown transaction must return error");

    println!(
        "SEP-6 deposit lifecycle test completed successfully. \
         Lifecycle: incomplete → pending_external → pending_anchor → pending_user → completed"
    );
}

// ---------------------------------------------------------------------------
// Additional invariant: deposit with valid Stellar depositor account accepted
// ---------------------------------------------------------------------------

#[test]
fn test_sep6_deposit_with_depositor_account_accepted() {
    let valid_account = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWNA";
    let raw = RawDepositResponse {
        transaction_id: "txn-with-account".to_string(),
        how: "Send USDC to the address below.".to_string(),
        extra_info: None,
        min_amount: Some(5),
        max_amount: Some(1_000),
        fee_fixed: None,
        fee_percent: None,
        status: Some("pending_external".to_string()),
        depositor_account: Some(valid_account.to_string()),
    };

    let result = initiate_deposit(raw, "USDC");
    assert!(result.is_ok(), "deposit with valid Stellar address must be accepted");
    let deposit = result.unwrap();
    assert_eq!(deposit.status, TransactionStatus::PendingExternal);
}

// ---------------------------------------------------------------------------
// Negative: deposit with invalid Stellar address must be rejected
// ---------------------------------------------------------------------------

#[test]
fn test_sep6_deposit_invalid_depositor_account_rejected() {
    let raw = RawDepositResponse {
        transaction_id: "txn-bad-account".to_string(),
        how: "Send to the address.".to_string(),
        extra_info: None,
        min_amount: None,
        max_amount: None,
        fee_fixed: None,
        fee_percent: None,
        status: None,
        depositor_account: Some("not-a-stellar-address".to_string()),
    };

    let result = initiate_deposit(raw, "USDC");
    assert!(result.is_err(), "invalid Stellar address must cause initiate_deposit to fail");
}
