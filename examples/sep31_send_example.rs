//! SEP-31 Cross-Border Payments — sending-anchor example
//!
//! Demonstrates the full client flow a sending anchor runs when initiating a
//! cross-border payment via SEP-31:
//!
//! 1. Query `GET /info` to discover which assets the receiver anchor accepts
//!    and which SEP-12 KYC fields it requires.
//! 2. Validate the payment amount against the anchor's limits.
//! 3. Call `POST /transactions` to initiate the payment.
//! 4. Poll `GET /transactions/:id` until the transaction reaches a terminal
//!    status.
//! 5. Register a callback URL for push notifications.
//!
//! The example uses only AnchorKit's normalization layer — no real HTTP calls
//! are made.  Callers make HTTP requests themselves (e.g. with `reqwest`) and
//! feed the raw response bodies into the `Raw*` input structs shown here.
//!
//! # SEP-10 authentication
//!
//! Every real request to a SEP-31 endpoint must include an
//! `Authorization: Bearer <jwt>` header obtained via SEP-10.  That step is
//! omitted here because it requires a live Stellar network connection.

use anchorkit::sep31::{
    get_sep31_info, get_sep31_transaction, handle_sep31_http_status,
    initiate_sep31_transaction, set_sep31_callback, update_sep31_transaction,
    RawSep31Asset, RawSep31CallbackRequest, RawSep31Field, RawSep31InfoResponse,
    RawSep31TransactionRequest, RawSep31TransactionResponse,
    RawSep31TransactionStatusResponse, RawSep31UpdateRequest, Sep31TransactionStatus,
};
use anchorkit::errors::AnchorKitError;

const RECEIVER_STELLAR_ACCOUNT: &str =
    "GAAACAQDAQCQMBYIBEFAWDANBYHRAEISCMKBKFQXDAMRUGY4DUPB7JZX";

fn sep() {
    println!("\n{}", "─".repeat(60));
}

// ── Step 1: GET /info ─────────────────────────────────────────────────────────

fn demo_get_info() {
    sep();
    println!(" Step 1 · GET /info");
    sep();

    let raw = RawSep31InfoResponse {
        assets: vec![
            RawSep31Asset {
                asset_code: "USDC".to_string(),
                sep12_sender_types: vec!["sep31-sender".to_string()],
                sep12_receiver_types: vec!["sep31-receiver".to_string()],
                min_amount: Some(1_000_000),
                max_amount: Some(100_000_000_000),
                fee_fixed: Some(500_000),
                fee_percent: Some(30),
                fields: vec![
                    RawSep31Field {
                        name: "bank_account_number".to_string(),
                        description: "Receiver's bank account number".to_string(),
                        optional: false,
                        choices: vec![],
                    },
                    RawSep31Field {
                        name: "bank_routing_number".to_string(),
                        description: "US ABA routing number (9 digits)".to_string(),
                        optional: false,
                        choices: vec![],
                    },
                    RawSep31Field {
                        name: "bank_account_type".to_string(),
                        description: "Type of bank account".to_string(),
                        optional: true,
                        choices: vec!["checking".to_string(), "savings".to_string()],
                    },
                ],
            },
            RawSep31Asset {
                asset_code: "EURC".to_string(),
                sep12_sender_types: vec!["sep31-sender".to_string()],
                sep12_receiver_types: vec!["sep31-receiver-eu".to_string()],
                min_amount: Some(5_000_000),
                max_amount: Some(50_000_000_000),
                fee_fixed: Some(1_000_000),
                fee_percent: Some(25),
                fields: vec![RawSep31Field {
                    name: "iban".to_string(),
                    description: "Receiver IBAN".to_string(),
                    optional: false,
                    choices: vec![],
                }],
            },
        ],
    };

    match get_sep31_info(raw) {
        Ok(info) => {
            println!("Receiver anchor supports {} asset(s):", info.assets.len());
            for asset in &info.assets {
                println!(
                    "  {} — min: {:?}, max: {:?}, fee_percent: {:?} bps",
                    asset.asset_code, asset.min_amount, asset.max_amount, asset.fee_percent
                );
                for f in &asset.fields {
                    let req = if f.optional { "optional" } else { "required" };
                    println!("    field `{}` ({req}): {}", f.name, f.description);
                }
            }
        }
        Err(e) => println!("  ✗ GET /info failed: {e}"),
    }
}

// ── Step 2: POST /transactions ────────────────────────────────────────────────

fn demo_initiate() {
    sep();
    println!(" Step 2 · POST /transactions");
    sep();

    let req = RawSep31TransactionRequest {
        amount: 10_000_000,
        asset_code: "USDC".to_string(),
        asset_issuer: Some(RECEIVER_STELLAR_ACCOUNT.to_string()),
        destination_asset: None,
        quote_id: None,
        sender_id: "d9f4e2a1-78bc-4c3e-a3b1-9f2201d81432".to_string(),
        receiver_id: "b7c3f1d0-e5a9-4d8b-bb0c-123456789abc".to_string(),
        fields: Some(vec![
            ("bank_account_number".to_string(), "00123456789".to_string()),
            ("bank_routing_number".to_string(), "021000021".to_string()),
        ]),
    };

    let raw_resp = RawSep31TransactionResponse {
        id: "TX-SEP31-20241121-0001".to_string(),
        stellar_account_id: RECEIVER_STELLAR_ACCOUNT.to_string(),
        stellar_memo: Some("4209871234".to_string()),
        stellar_memo_type: Some("id".to_string()),
    };

    match initiate_sep31_transaction(req, raw_resp) {
        Ok(tx) => {
            println!("  ✓ Transaction initiated");
            println!("    id:                 {}", tx.id);
            println!("    stellar_account_id: {}", tx.stellar_account_id);
            println!("    stellar_memo:       {:?}", tx.stellar_memo);
            println!("    stellar_memo_type:  {:?}", tx.stellar_memo_type);
            println!();
            println!(
                "  → Send Stellar payment to {} with memo {} (type: id)",
                tx.stellar_account_id,
                tx.stellar_memo.as_deref().unwrap_or("(none)")
            );
        }
        Err(e) => println!("  ✗ POST /transactions failed: {e}"),
    }
}

// ── Step 3: GET /transactions/:id ─────────────────────────────────────────────

fn demo_get_transaction() {
    sep();
    println!(" Step 3 · GET /transactions/:id  (polling)");
    sep();

    let pending = RawSep31TransactionStatusResponse {
        id: "TX-SEP31-20241121-0001".to_string(),
        status: "pending_stellar".to_string(),
        message: Some("Waiting for Stellar payment.".to_string()),
        amount_in: Some(10_000_000),
        amount_out: Some(9_650_000),
        amount_fee: Some(350_000),
        stellar_account_id: RECEIVER_STELLAR_ACCOUNT.to_string(),
        stellar_memo: Some("4209871234".to_string()),
        stellar_memo_type: Some("id".to_string()),
        started_at: Some(1_732_185_600),
        completed_at: None,
        external_transaction_id: None,
    };

    println!("  Poll #1 (HTTP 200):");
    match get_sep31_transaction(200, pending) {
        Ok(tx) => println!(
            "    status={} terminal={} needs_action={}",
            tx.status.as_str(),
            tx.status.is_terminal(),
            tx.status.requires_action()
        ),
        Err(e) => println!("    ✗ {e}"),
    }

    let completed = RawSep31TransactionStatusResponse {
        id: "TX-SEP31-20241121-0001".to_string(),
        status: "completed".to_string(),
        message: Some("Payment delivered.".to_string()),
        amount_in: Some(10_000_000),
        amount_out: Some(9_650_000),
        amount_fee: Some(350_000),
        stellar_account_id: RECEIVER_STELLAR_ACCOUNT.to_string(),
        stellar_memo: Some("4209871234".to_string()),
        stellar_memo_type: Some("id".to_string()),
        started_at: Some(1_732_185_600),
        completed_at: Some(1_732_186_200),
        external_transaction_id: Some("WIRE-20241121-98765".to_string()),
    };

    println!("  Poll #2 (HTTP 200):");
    match get_sep31_transaction(200, completed) {
        Ok(tx) => {
            println!("    status={} terminal={}", tx.status.as_str(), tx.status.is_terminal());
            println!("    external_id={:?}", tx.external_transaction_id);
        }
        Err(e) => println!("    ✗ {e}"),
    }

    let dummy = RawSep31TransactionStatusResponse {
        id: "MISSING".to_string(), status: "".to_string(), message: None,
        amount_in: None, amount_out: None, amount_fee: None,
        stellar_account_id: RECEIVER_STELLAR_ACCOUNT.to_string(),
        stellar_memo: None, stellar_memo_type: None,
        started_at: None, completed_at: None, external_transaction_id: None,
    };
    println!("  Simulated HTTP 404:");
    match get_sep31_transaction(404, dummy) {
        Ok(_)  => println!("    Should not succeed"),
        Err(e) => println!("    ✓ Expected error: {e}"),
    }
}

// ── Step 4: PUT /transactions/:id ─────────────────────────────────────────────

fn demo_update() {
    sep();
    println!(" Step 4 · PUT /transactions/:id  (fix wrong routing number)");
    sep();

    let req = RawSep31UpdateRequest {
        transaction_id: "TX-SEP31-20241121-0001".to_string(),
        fields: vec![("bank_routing_number".to_string(), "021000089".to_string())],
    };
    match update_sep31_transaction(req) {
        Ok(r)  => println!("  ✓ {} field(s) validated: {:?}", r.fields.len(), r.fields),
        Err(e) => println!("  ✗ {e}"),
    }
}

// ── Step 5: PUT /transactions/:id/callback ────────────────────────────────────

fn demo_callback() {
    sep();
    println!(" Step 5 · PUT /transactions/:id/callback");
    sep();

    let valid = RawSep31CallbackRequest {
        transaction_id: "TX-SEP31-20241121-0001".to_string(),
        callback_url: "https://my-sending-anchor.example.com/sep31/callback".to_string(),
    };
    match set_sep31_callback(valid) {
        Ok(r)  => println!("  ✓ Callback registered: {}", r.callback_url),
        Err(e) => println!("  ✗ {e}"),
    }

    let insecure = RawSep31CallbackRequest {
        transaction_id: "TX-SEP31-20241121-0001".to_string(),
        callback_url: "http://insecure.example.com/cb".to_string(),
    };
    match set_sep31_callback(insecure) {
        Ok(_)  => println!("  ✗ Should have been rejected"),
        Err(e) => println!("  ✓ HTTP callback correctly rejected: {e}"),
    }
}

// ── HTTP status helper demo ───────────────────────────────────────────────────

fn demo_http_dispatcher() {
    sep();
    println!(" handle_sep31_http_status — all code paths");
    sep();
    for &code in &[200u32, 403, 404, 408, 429, 500, 503, 302] {
        let res = handle_sep31_http_status(code, || Ok::<_, AnchorKitError>("ok"));
        match res {
            Ok(v)  => println!("  HTTP {code} → Ok({v})"),
            Err(e) => println!("  HTTP {code} → Err({e})"),
        }
    }
}

// ── Status enum reference ─────────────────────────────────────────────────────

fn demo_status_table() {
    sep();
    println!(" Sep31TransactionStatus reference");
    sep();
    let statuses = [
        "pending_sender", "pending_stellar",
        "pending_customer_info_update", "pending_transaction_info_update",
        "pending_receiver", "pending_external",
        "completed", "refunded", "expired", "error", "future_unknown",
    ];
    println!("  {:<40} {:>8}  {:>12}", "status", "terminal", "needs_action");
    println!("  {}", "─".repeat(64));
    for s in statuses {
        let st = Sep31TransactionStatus::from_str(s);
        println!(
            "  {:<40} {:>8}  {:>12}",
            st.as_str(), st.is_terminal(), st.requires_action()
        );
    }
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║   AnchorKit · SEP-31 Cross-Border Payments Example       ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    demo_get_info();
    demo_initiate();
    demo_get_transaction();
    demo_update();
    demo_callback();
    demo_http_dispatcher();
    demo_status_table();

    println!("\n✓ Done.");
}
