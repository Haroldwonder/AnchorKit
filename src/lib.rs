#![no_std]
extern crate alloc;

mod contract;
pub mod deterministic_hash;
pub mod domain_validator;
mod errors;
mod events;
mod rate_limiter;
mod response_validator;
mod retry;
pub mod sdk_config;
pub mod sep10_jwt;
pub mod sep6;
mod storage;
mod transaction_state_tracker;
pub mod transport;
mod types;

pub use errors::{AnchorKitError, ErrorCode};

pub use deterministic_hash::{compute_payload_hash, verify_payload_hash};
pub use domain_validator::validate_anchor_domain;
pub use errors::Error;
pub use rate_limiter::{RateLimitConfig, RateLimitState, RateLimitStatus, RateLimiter};
pub use response_validator::{
    validate_anchor_info_response, validate_deposit_response, validate_quote_response,
    validate_withdraw_response, AnchorInfoResponse, QuoteResponse,
};
pub use retry::{is_retryable, retry_with_backoff, RetryConfig, RetryErrorCode};

pub use contract::{
    get_admin, get_attestation_count, get_endpoint, set_endpoint, AnchorKitContract,
};
pub use events::EndpointUpdated;
pub use sep6::{
    deposit_exchange, fetch_transaction_status, get_fee_estimate, get_transactions,
    initiate_deposit, initiate_withdrawal, validate_amount, withdraw_exchange, AnchorFeeData,
    AssetLimits, DepositExchangeResponse, FeeEstimate, FeeOperation, RawDepositExchangeRequest,
    RawDepositResponse, RawTransactionResponse, RawWithdrawExchangeResponse, RawWithdrawalResponse,
    TransactionFilters, TransactionKind, TransactionStatusResponse,
};
pub use types::{AnchorMetadata, RoutingOptions, RoutingRequest};
pub use types::{DepositResponse, TransactionStatus, WithdrawalResponse};

// Generated contract client, re-exported for the criterion benchmark suite
// (`benches/`, issue #738) which drives the contract through the Soroban test
// harness as an external consumer of this crate.
pub use contract::AnchorKitContractClient;

#[cfg(test)]
mod transaction_state_tracker_tests;

#[cfg(test)]
mod request_id_tests;

#[cfg(test)]
mod tracing_span_tests;

#[cfg(test)]
mod metadata_cache_tests;

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

// Snapshot path note: anchor_info_discovery_tests uses an inner module of the
// same name, so Soroban writes snapshots to
//   test_snapshots/anchor_info_discovery_tests/anchor_info_discovery_tests/
// The previously existing test_snapshots/anchor_info_discovery/tests/ directory
// was generated under an older module layout and has been removed.

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

#[cfg(test)]
mod replay_window_tests;

#[cfg(test)]
mod attestor_cap_batch_tests;

#[cfg(test)]
mod anchor_health_score_tests;

#[cfg(test)]
mod compute_payload_hash_tests;

#[cfg(test)]
mod new_features_tests;

#[cfg(test)]
mod audit_log_offset_tests;

#[cfg(test)]
mod contract_tests;

#[cfg(test)]
mod payload_hash_vectors_tests;

#[cfg(test)]
mod session_expiry_error_tests;

#[cfg(test)]
mod pause_tests;

#[cfg(test)]
mod attestation_validation_tests;

#[cfg(test)]
mod attestor_event_tests;

#[cfg(test)]
mod sdk_config_tests;

#[cfg(test)]
mod streaming_flow_tests;
