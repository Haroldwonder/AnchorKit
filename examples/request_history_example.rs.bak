/// Example demonstrating Request Tracing and Session Management features
///
/// This example shows how to:
/// 1. Track operations using request IDs
/// 2. Use sessions to group related operations
/// 3. View tracing spans for operations
/// 4. Monitor operation flow and audit trail

use soroban_sdk::{testutils::Address as _, Address, Bytes, BytesN, Env, String, Vec};

// Import the contract types
use anchorkit::contract::{AnchorKitContract, AnchorKitContractClient};

fn main() {
    println!("=== AnchorKit Request Tracing & Session Management Example ===\n");

    // Setup environment
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let attestor1 = Address::generate(&env);
    let attestor2 = Address::generate(&env);
    let subject = Address::generate(&env);

    println!("1. Initializing contract...");
    client.initialize(&admin);
    println!("   ✓ Contract initialized\n");

    println!("2. Setting up SEP-10 verification...");
    let sep10_issuer = Address::generate(&env);
    client.set_sep10_jwt_verifying_key(&sep10_issuer, &Bytes::from_slice(&env, &[0u8; 32]));
    println!("   ✓ SEP-10 verification configured\n");

    println!("3. Creating a session for grouped operations...");
    let session_id = client.create_session(&admin);
    println!("   ✓ Session created with ID: {}\n", session_id);

    println!("4. Registering attestors within session...");
    client.register_attestor_with_session(&session_id, &attestor1);
    println!("   ✓ Attestor 1 registered");

    client.register_attestor_with_session(&session_id, &attestor2);
    println!("   ✓ Attestor 2 registered\n");

    println!("5. Submitting attestations with request ID tracking...");
    let timestamp = env.ledger().timestamp();
    let payload_hash1 = BytesN::from_array(&env, &[1u8; 32]);
    let payload_hash2 = BytesN::from_array(&env, &[2u8; 32]);
    let signature = Bytes::new(&env);

    // Generate request ID for first attestation
    let request_id1 = client.generate_request_id();
    println!("   → Request ID 1 generated");

    let attestation_id1 = client.submit_with_request_id(
        &request_id1,
        &attestor1,
        &subject,
        &timestamp,
        &payload_hash1,
        &signature,
    );
    println!("   ✓ Attestation 1 submitted (ID: {})", attestation_id1);

    // Generate request ID for second attestation
    let request_id2 = client.generate_request_id();
    println!("   → Request ID 2 generated");

    let attestation_id2 = client.submit_with_request_id(
        &request_id2,
        &attestor2,
        &subject,
        &timestamp,
        &payload_hash2,
        &signature,
    );
    println!("   ✓ Attestation 2 submitted (ID: {})\n", attestation_id2);

    println!("6. Retrieving tracing spans for operations...");

    // Get tracing span for first request
    let span1 = client.get_tracing_span(&request_id1.id);
    if let Some(span) = span1 {
        println!("\n   === TRACING SPAN 1 ===");
        println!("   Operation: {}", span.operation.to_string());
        println!("   Started at: {}", span.started_at);
        println!("   Completed at: {}", span.completed_at);
        println!("   Status: {}", span.status.to_string());
        println!("   Duration: {} seconds", span.completed_at - span.started_at);
    } else {
        println!("   ⚠ No tracing span found for request 1");
    }

    // Get tracing span for second request
    let span2 = client.get_tracing_span(&request_id2.id);
    if let Some(span) = span2 {
        println!("\n   === TRACING SPAN 2 ===");
        println!("   Operation: {}", span.operation.to_string());
        println!("   Started at: {}", span.started_at);
        println!("   Completed at: {}", span.completed_at);
        println!("   Status: {}", span.status.to_string());
        println!("   Duration: {} seconds", span.completed_at - span.started_at);
    } else {
        println!("   ⚠ No tracing span found for request 2");
    }

    println!("\n7. Retrieving session information...");
    let session = client.get_session(&session_id);
    println!("\n   === SESSION DETAILS ===");
    println!("   Session ID: {}", session.session_id);
    println!("   Initiator: {:?}", session.initiator);
    println!("   Created at: {}", session.created_at);
    println!("   Nonce: {}", session.nonce);
    println!("   Operation count: {}", session.operation_count);

    // Get operation count
    let op_count = client.get_session_operation_count(&session_id);
    println!("   Total operations in session: {}", op_count);

    println!("\n8. Submitting a quote with request tracking...");

    // Configure quote service for attestor1
    let mut services = Vec::new(&env);
    services.push_back(3u32); // SERVICE_QUOTES
    client.configure_services(&attestor1, &services);

    let base_asset = String::from_str(&env, "USD");
    let quote_asset = String::from_str(&env, "USDC");
    let rate = 10000u64;
    let fee_percentage = 100u32;
    let minimum_amount = 100u64;
    let maximum_amount = 10000u64;
    let valid_until = env.ledger().timestamp() + 3600;

    // Generate request ID for quote
    let quote_request_id = client.generate_request_id();

    let quote_id = client.quote_with_request_id(
        &quote_request_id,
        &attestor1,
        &base_asset,
        &quote_asset,
        &rate,
        &fee_percentage,
        &minimum_amount,
        &maximum_amount,
        &valid_until,
    );
    println!("   ✓ Quote submitted with tracking (ID: {})", quote_id);

    // Get tracing span for quote
    let quote_span = client.get_tracing_span(&quote_request_id.id);
    if let Some(span) = quote_span {
        println!("\n   === QUOTE TRACING SPAN ===");
        println!("   Operation: {}", span.operation.to_string());
        println!("   Status: {}", span.status.to_string());
        println!("   Duration: {} seconds", span.completed_at - span.started_at);
    }

    println!("\n9. Final session summary...");
    let final_session = client.get_session(&session_id);
    println!("   Total operations tracked in session: {}", final_session.operation_count);

    println!("\n=== Example Complete ===");
    println!("\nKey Features Demonstrated:");
    println!("✓ Request ID generation for operation tracking");
    println!("✓ Session-based grouping of related operations");
    println!("✓ Tracing span retrieval for operation monitoring");
    println!("✓ Operation flow tracking and audit trail");
    println!("✓ Timestamp and duration tracking");
    println!("✓ Support for multiple operation types (attestations, quotes)");
}
