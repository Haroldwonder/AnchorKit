//! SEP-31 sending-anchor client example.
//!
//! Walks through a complete receiving-anchor interaction without making any
//! network requests: a [`Sep31Client`] backed by a [`MockSep31Transport`]
//! discovers info, creates a transaction, polls it, patches requested fields,
//! registers a callback URL, and verifies a status callback.
//!
//! Run with: `cargo run --example sep31_sending_anchor`

use anchorkit::{
    callback_signature_header, MockSep31Transport, RawSep31CreationResponse, RawSep31InfoResponse,
    RawSep31Transaction, Sep31AssetInfo, Sep31Auth, Sep31Callback, Sep31Client,
    Sep31CustomerReference, Sep31Request, Sep31Response, Sep31Status, Sep31TransactionRequest,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use ed25519_dalek::{Signer, SigningKey};

const JWT: &str = "header.payload.signature";
const ACCOUNT: &str = "GAAACAQDAQCQMBYIBEFAWDANBYHRAEISCMKBKFQXDAMRUGY4DUPB7JZX";
const CALLBACK_URL: &str = "https://sending.example.com/sep31/callback";

fn main() {
    println!("=== SEP-31 Sending Anchor Example ===\n");

    let auth = Sep31Auth::new(JWT).expect("valid SEP-10 JWT");
    let asset =
        Sep31AssetInfo::new("USDC", &["SEPA", "SWIFT"]).with_limits(Some("0.1"), Some("1000"));

    let mut transport = MockSep31Transport::new();
    transport.add_response(
        Sep31Request::info(JWT, Some("en")),
        Sep31Response::Info(
            RawSep31InfoResponse::new(vec![asset])
                .with_customer_types(&["sep31_sender"], &["sep31_receiver"]),
        ),
    );

    // 1. Register both customers through SEP-12 before creating the payment.
    //    The IDs returned by SEP-12 `PUT /customer` are referenced here.
    let request = Sep31TransactionRequest::new("100", "USDC", "SWIFT")
        .with_asset_issuer(ACCOUNT)
        .with_destination_asset("iso4217:BRL")
        .with_sender(Sep31CustomerReference::with_type(
            "sender-customer-id",
            "sep31_sender",
        ))
        .with_receiver(Sep31CustomerReference::new("receiver-customer-id"));

    transport.add_response(
        Sep31Request::create(&request, JWT),
        Sep31Response::Created(RawSep31CreationResponse::new("tx-123").with_stellar(
            ACCOUNT,
            "987654321",
            "id",
        )),
    );
    transport.add_response(
        Sep31Request::get("tx-123", JWT),
        Sep31Response::Transaction(Box::new(
            RawSep31Transaction::new("tx-123", Sep31Status::PendingSender)
                .with_amounts("100", "95", "5")
                .with_stellar(ACCOUNT, "987654321", "id"),
        )),
    );
    let fields = [("receiver_bank_account", "123456789")];
    transport.add_response(
        Sep31Request::patch("tx-123", &fields, JWT),
        Sep31Response::Transaction(Box::new(RawSep31Transaction::new(
            "tx-123",
            Sep31Status::PendingReceiver,
        ))),
    );
    transport.add_response(
        Sep31Request::register_callback("tx-123", CALLBACK_URL, JWT),
        Sep31Response::callback_registered(),
    );

    let mut client = Sep31Client::new(transport, auth);

    // 2. Discover what the receiving anchor supports.
    let info = client.get_info(Some("en")).expect("info response");
    println!(
        "Supported assets: {:?}; SEP-12 sender types: {:?}",
        info.receive
            .iter()
            .map(|asset| asset.asset_code.as_str())
            .collect::<Vec<_>>(),
        info.sender_customer_types
    );

    // 3. Create the transaction and read the Stellar payment instructions.
    let created = client
        .create_transaction(&request)
        .expect("creation response");
    println!(
        "Created {} — ready to pay: {} (memo {:?})",
        created.id,
        created.is_ready_to_pay(),
        created.stellar_memo
    );

    // 4. Poll until it is ready, then patch any requested fields.
    let transaction = client.get_transaction(&created.id).expect("transaction");
    println!(
        "Status: {} — terminal: {}",
        transaction.status.as_str(),
        transaction.status.is_terminal()
    );

    let updated = client
        .patch_transaction(&created.id, &fields)
        .expect("patch response");
    println!("Status after PATCH: {}", updated.status.as_str());

    // 5. Ask the receiving anchor to call us back on status changes.
    client
        .register_callback(&created.id, CALLBACK_URL)
        .expect("callback registration");

    // 6. Handle and verify a callback POST from the receiving anchor.
    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
    let timestamp = 1_700_000_000u64;
    let body = RawSep31Transaction::new("tx-123", Sep31Status::Completed);
    let body_json = r#"{"transaction":{"id":"tx-123","status":"completed"}}"#;
    let payload = format!("{timestamp}.{CALLBACK_URL}.{body_json}");
    let signature = signing_key.sign(payload.as_bytes());
    let header = format!("t={timestamp}, s={}", STANDARD.encode(signature.to_bytes()));

    let header = callback_signature_header(Some(&header), None).expect("signature header");
    let callback = Sep31Callback::parse(header, CALLBACK_URL, body, timestamp + 5, 60)
        .expect("valid callback");
    callback
        .verify(body_json, signing_key.verifying_key().as_bytes())
        .expect("valid callback signature");
    println!(
        "Verified callback for {} with status {}",
        callback.transaction.id,
        callback.transaction.status.as_str()
    );

    println!("\n=== Example Complete ===");
}
