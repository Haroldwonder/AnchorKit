#![no_std]
extern crate alloc;

mod deterministic_hash;
mod domain_validator;
mod errors;
mod events;
mod storage;
mod types;
mod validation;
mod sep6;
mod contract;
mod rate_limiter;
mod response_validator;
mod retry;
mod sep10_jwt;

#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod streaming_flow_tests;

use soroban_sdk::{contract, contractimpl, Address, Bytes, BytesN, Env, String, Vec};

pub use config::{AttestorConfig, ContractConfig, SessionConfig};
pub use errors::Error;
pub use rate_limiter::{RateLimiter, RateLimitConfig, RateLimitState};
pub use response_validator::{
    validate_anchor_info_response, validate_deposit_response, validate_quote_response,
    validate_withdraw_response, AnchorInfoResponse, QuoteResponse,
};
pub use retry::{retry_with_backoff, is_retryable, RetryConfig};
pub use deterministic_hash::{compute_payload_hash, verify_payload_hash};

#[cfg(test)]
mod transaction_state_tracker_tests;
pub use sep6::{
    fetch_transaction_status, initiate_deposit, initiate_withdrawal,
    RawDepositResponse, RawTransactionResponse, RawWithdrawalResponse, TransactionKind,
    TransactionStatusResponse, map_sep6_error_code, poll_kyc_status, submit_sep6_attestation,
    Sep6Attestation,
};
pub use types::{DepositResponse, WithdrawalResponse, TransactionStatus};
pub use contract::{AnchorKitContract, get_admin, get_endpoint, set_endpoint, get_attestation_count};
pub use events::EndpointUpdated;

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
mod anchor_health_score_tests;

#[cfg(test)]
mod compute_payload_hash_tests;

#[cfg(test)]
mod sep6_integration_tests;
