#![no_std]
extern crate alloc;

mod deterministic_hash;
mod domain_validator;
mod errors;
mod sep10_jwt;
mod rate_limiter;
mod response_validator;
mod retry;
mod transaction_state_tracker;
pub mod storage;
pub mod sep6;
pub mod contract;

pub use domain_validator::validate_anchor_domain;
pub use errors::{AnchorKitError, ErrorCode};

/// Backward-compatible alias. Prefer [`AnchorKitError`] for new code.
pub use errors::Error;
pub use rate_limiter::{RateLimiter, RateLimitConfig, RateLimitState};
pub use response_validator::{
    validate_anchor_info_response, validate_deposit_response, validate_quote_response,
    validate_withdraw_response, AnchorInfoResponse, DepositResponse as ValidatorDepositResponse,
    QuoteResponse, WithdrawResponse,
};
pub use retry::{retry_with_backoff, is_retryable, RetryConfig};
pub use deterministic_hash::{compute_payload_hash, verify_payload_hash};

#[cfg(test)]
mod transaction_state_tracker_tests;
pub use sep6::{
    fetch_transaction_status, initiate_deposit, initiate_withdrawal, DepositResponse,
    RawDepositResponse, RawTransactionResponse, RawWithdrawalResponse, TransactionKind,
    TransactionStatus, TransactionStatusResponse, WithdrawalResponse,
};
pub use contract::{AnchorKitContract, EndpointUpdated, get_admin, get_endpoint, set_endpoint, get_attestation_count};

#[cfg(test)]
mod request_id_tests;

#[cfg(test)]
mod tracing_span_tests;

#[cfg(test)]
mod metadata_cache_tests;

#[cfg(test)]
mod streaming_flow_tests;

#[cfg(test)]
mod webhook_middleware_tests;

#[cfg(test)]
mod session_tests;

#[cfg(test)]
mod anchor_info_discovery_tests;

#[cfg(test)]
mod sep10_test_util;

#[cfg(test)]
mod sep10_contract_tests;

#[cfg(test)]
mod routing_tests;

#[cfg(test)]
mod deterministic_hash_snapshot_tests {
    // Snapshot tests live inside deterministic_hash module itself.
    // This module exists to satisfy the test_snapshots/deterministic_hash_tests path.
}

#[cfg(test)]
mod capability_detection_tests;

#[cfg(test)]
mod attestor_endpoint_tests;

#[cfg(test)]
mod attestation_pagination_tests;

#[cfg(test)]
mod is_initialized_tests;

#[cfg(test)]
mod get_attestation_tests;
