//! Integration tests for SEP-6 deposit happy path
//!
//! These tests cover the full workflow of a typical SEP-6 deposit:
//! 1. Initiate a deposit with an anchor
//! 2. Poll for KYC status updates
//! 3. Confirm transaction completion

extern crate alloc;

use crate::sep6::*;
use crate::errors::{Error, ErrorCode};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[test]
fn test_sep6_deposit_happy_path_basic_flow() {
    // Step 1: Initiate a deposit
    let deposit = RawDepositResponse {
        transaction_id: "deposit-12345".to_string(),
        how: "Send USDC to bank account 1234567890".to_string(),
        extra_info: Some("Reference: INV-001".to_string()),
        min_amount: Some(100),
        max_amount: Some(100_000),
        fee_fixed: Some(2),
        fee_percent: Some(25),
        status: Some("pending_external".to_string()),
        depositor_account: Some("GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWNA".to_string()),
    };

    let deposit_response = initiate_deposit(deposit, "USDC").expect("Deposit should succeed");
    assert_eq!(deposit_response.transaction_id, "deposit-12345");
    assert_eq!(deposit_response.how, "Send USDC to bank account 1234567890");
    assert_eq!(deposit_response.status, TransactionStatus::PendingExternal);
    assert_eq!(deposit_response.min_amount, Some(100));
    assert_eq!(deposit_response.max_amount, Some(100_000));
    assert_eq!(deposit_response.fee_fixed, Some(2));
    assert_eq!(deposit_response.fee_percent, Some(25));

    // Step 2: Fetch initial transaction status (simulating polling)
    let txn_status = RawTransactionResponse {
        transaction_id: "deposit-12345".to_string(),
        kind: Some("deposit".to_string()),
        status: "pending_external".to_string(),
        amount_in: None,
        amount_out: None,
        amount_fee: None,
        message: Some("Waiting for customer bank transfer".to_string()),
    };

    let status = fetch_transaction_status(txn_status).expect("Status fetch should succeed");
    assert_eq!(status.transaction_id, "deposit-12345");
    assert_eq!(status.kind, TransactionKind::Deposit);
    assert_eq!(status.status, TransactionStatus::PendingExternal);
    assert_eq!(
        status.message,
        Some("Waiting for customer bank transfer".to_string())
    );

    // Step 3: Simulate status updates during polling
    let txn_anchor_received = RawTransactionResponse {
        transaction_id: "deposit-12345".to_string(),
        kind: Some("deposit".to_string()),
        status: "pending_anchor".to_string(),
        amount_in: Some(1000),
        amount_out: None,
        amount_fee: None,
        message: Some("Anchor received bank transfer, processing".to_string()),
    };

    let status = fetch_transaction_status(txn_anchor_received).expect("Status fetch should succeed");
    assert_eq!(status.status, TransactionStatus::PendingAnchor);
    assert_eq!(status.amount_in, Some(1000));

    // Step 4: Final completion status
    let txn_completed = RawTransactionResponse {
        transaction_id: "deposit-12345".to_string(),
        kind: Some("deposit".to_string()),
        status: "completed".to_string(),
        amount_in: Some(1000),
        amount_out: Some(998),
        amount_fee: Some(2),
        message: Some("Deposit completed successfully".to_string()),
    };

    let final_status = fetch_transaction_status(txn_completed).expect("Status fetch should succeed");
    assert_eq!(final_status.status, TransactionStatus::Completed);
    assert_eq!(final_status.amount_in, Some(1000));
    assert_eq!(final_status.amount_out, Some(998));
    assert_eq!(final_status.amount_fee, Some(2));
}

#[test]
fn test_sep6_deposit_with_kyc_requirement() {
    // Deposit flow where customer info is needed
    let deposit = RawDepositResponse {
        transaction_id: "deposit-kyc-456".to_string(),
        how: "Send USDC via wire transfer".to_string(),
        extra_info: None,
        min_amount: Some(500),
        max_amount: Some(50_000),
        fee_fixed: Some(5),
        fee_percent: Some(10),
        status: Some("incomplete".to_string()),
        depositor_account: None,
    };

    let deposit_response = initiate_deposit(deposit, "USDC").expect("Deposit should succeed");
    assert_eq!(deposit_response.status, TransactionStatus::Incomplete);

    // Simulate polling where KYC is needed
    let kyc_needed_status = RawTransactionResponse {
        transaction_id: "deposit-kyc-456".to_string(),
        kind: Some("deposit".to_string()),
        status: "pending_user".to_string(),
        amount_in: None,
        amount_out: None,
        amount_fee: None,
        message: Some("Please complete KYC verification".to_string()),
    };

    let status = fetch_transaction_status(kyc_needed_status).expect("Status fetch should succeed");
    assert_eq!(status.status, TransactionStatus::PendingUser);

    // After KYC is completed, status progresses
    let kyc_completed_status = RawTransactionResponse {
        transaction_id: "deposit-kyc-456".to_string(),
        kind: Some("deposit".to_string()),
        status: "pending_external".to_string(),
        amount_in: None,
        amount_out: None,
        amount_fee: None,
        message: Some("KYC verified, waiting for transfer".to_string()),
    };

    let status = fetch_transaction_status(kyc_completed_status).expect("Status fetch should succeed");
    assert_eq!(status.status, TransactionStatus::PendingExternal);
}

#[test]
fn test_sep6_deposit_with_error_status() {
    // Deposit that fails
    let deposit = RawDepositResponse {
        transaction_id: "deposit-error-789".to_string(),
        how: "Send via rejected method".to_string(),
        extra_info: None,
        min_amount: Some(100),
        max_amount: Some(10_000),
        fee_fixed: None,
        fee_percent: None,
        status: Some("error".to_string()),
        depositor_account: None,
    };

    let deposit_response = initiate_deposit(deposit, "USDC").expect("Deposit should succeed");
    assert_eq!(deposit_response.status, TransactionStatus::Error);

    // Error status confirmed
    let error_status = RawTransactionResponse {
        transaction_id: "deposit-error-789".to_string(),
        kind: Some("deposit".to_string()),
        status: "error".to_string(),
        amount_in: None,
        amount_out: None,
        amount_fee: None,
        message: Some("Deposit method not supported by anchor".to_string()),
    };

    let status = fetch_transaction_status(error_status).expect("Status fetch should succeed");
    assert_eq!(status.status, TransactionStatus::Error);
    assert_eq!(
        status.message,
        Some("Deposit method not supported by anchor".to_string())
    );
}

#[test]
fn test_sep6_deposit_list_and_pagination() {
    // Simulate fetching a list of deposits for an account
    const ACCOUNT: &str = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWNA";

    let req = RawTransactionListRequest {
        account: ACCOUNT.to_string(),
        asset_code: "USDC".to_string(),
        limit: 10,
        cursor: None,
    };

    let raw_items = alloc::vec![
        RawTransactionResponse {
            transaction_id: "dep-1".to_string(),
            kind: Some("deposit".to_string()),
            status: "completed".to_string(),
            amount_in: Some(100),
            amount_out: Some(99),
            amount_fee: Some(1),
            message: None,
        },
        RawTransactionResponse {
            transaction_id: "dep-2".to_string(),
            kind: Some("deposit".to_string()),
            status: "pending".to_string(),
            amount_in: None,
            amount_out: None,
            amount_fee: None,
            message: None,
        },
        RawTransactionResponse {
            transaction_id: "dep-3".to_string(),
            kind: Some("deposit".to_string()),
            status: "completed".to_string(),
            amount_in: Some(500),
            amount_out: Some(498),
            amount_fee: Some(2),
            message: None,
        },
    ];

    let result = list_transactions(req, raw_items).expect("List should succeed");
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].transaction_id, "dep-1");
    assert_eq!(result[0].status, TransactionStatus::Completed);
    assert_eq!(result[1].transaction_id, "dep-2");
    assert_eq!(result[2].transaction_id, "dep-3");
}

#[test]
fn test_sep6_deposit_with_attestation() {
    // Deposit with attestation mapping
    let deposit = RawDepositResponse {
        transaction_id: "deposit-attest-100".to_string(),
        how: "Transfer to verified account".to_string(),
        extra_info: None,
        min_amount: Some(250),
        max_amount: Some(25_000),
        fee_fixed: Some(3),
        fee_percent: Some(5),
        status: Some("completed".to_string()),
        depositor_account: Some("GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWNA".to_string()),
    };

    let deposit_response = initiate_deposit(deposit, "USDC").expect("Deposit should succeed");
    assert_eq!(deposit_response.transaction_id, "deposit-attest-100");

    // Create attestation mapping for audit trail
    let attestation = submit_sep6_attestation(
        "deposit-attest-100",
        "sha256:abc123def456ghi789jkl",
        Some(1234567890),
    )
    .expect("Attestation submission should succeed");

    assert_eq!(attestation.transaction_id, "deposit-attest-100");
    assert_eq!(
        attestation.payload_hash,
        "sha256:abc123def456ghi789jkl"
    );
    assert_eq!(attestation.created_at, 1234567890);
}

#[test]
fn test_sep6_error_mapping() {
    // Test SEP-6 error code mapping
    let customer_info_error = map_sep6_error_code("customer_info_needed");
    assert_eq!(customer_info_error.code, ErrorCode::ComplianceNotMet);

    let txn_info_error = map_sep6_error_code("transaction_info_needed");
    assert_eq!(txn_info_error.code, ErrorCode::InvalidTransactionIntent);

    let too_large_error = map_sep6_error_code("too_large");
    assert_eq!(too_large_error.code, ErrorCode::ValidationError);

    let too_small_error = map_sep6_error_code("too_small");
    assert_eq!(too_small_error.code, ErrorCode::ValidationError);

    let invalid_params_error = map_sep6_error_code("invalid_request_params");
    assert_eq!(invalid_params_error.code, ErrorCode::ValidationError);

    let pending_kyc_error = map_sep6_error_code("pending_customer_info_update");
    assert_eq!(pending_kyc_error.code, ErrorCode::ComplianceNotMet);

    let unsupported_error = map_sep6_error_code("unsupported_operation");
    assert_eq!(unsupported_error.code, ErrorCode::InvalidServiceType);

    let unknown_error = map_sep6_error_code("unknown_sep6_code");
    assert_eq!(unknown_error.code, ErrorCode::ValidationError);
}

#[test]
fn test_sep6_deposit_validation_errors() {
    // Invalid asset code
    let deposit = RawDepositResponse {
        transaction_id: "tx-123".to_string(),
        how: "Transfer".to_string(),
        extra_info: None,
        min_amount: None,
        max_amount: None,
        fee_fixed: None,
        fee_percent: None,
        status: None,
        depositor_account: None,
    };

    let err = initiate_deposit(deposit.clone(), "").unwrap_err();
    assert_eq!(err.code, ErrorCode::ValidationError);

    let err = initiate_deposit(deposit.clone(), "TOOLONGASSETCODE").unwrap_err();
    assert_eq!(err.code, ErrorCode::ValidationError);

    // Invalid stellar address
    let mut deposit_with_addr = deposit.clone();
    deposit_with_addr.depositor_account = Some("invalid-address".to_string());
    let err = initiate_deposit(deposit_with_addr, "USDC").unwrap_err();
    assert_eq!(err.code, ErrorCode::ValidationError);

    // Valid flow with proper asset code
    let valid_result = initiate_deposit(deposit, "USDC");
    assert!(valid_result.is_ok());
}

#[test]
fn test_sep6_attestation_validation() {
    // Valid attestation
    let attestation = submit_sep6_attestation("txn-123", "hash-abc", Some(12345))
        .expect("Should create attestation");
    assert_eq!(attestation.transaction_id, "txn-123");
    assert_eq!(attestation.payload_hash, "hash-abc");
    assert_eq!(attestation.created_at, 12345);

    // Empty transaction_id should fail
    let err = submit_sep6_attestation("", "hash-abc", None).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidTransactionIntent);

    // Empty payload_hash should fail
    let err = submit_sep6_attestation("txn-123", "", None).unwrap_err();
    assert_eq!(err.code, ErrorCode::ValidationError);

    // Timestamp defaults to 0 if not provided
    let attestation = submit_sep6_attestation("txn-456", "hash-def", None)
        .expect("Should create attestation");
    assert_eq!(attestation.created_at, 0);
}
