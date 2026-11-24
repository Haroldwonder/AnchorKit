#![no_std]
#![warn(missing_docs)]

extern crate alloc;

/// Soroban contract implementing the AnchorKit attestation ledger.
///
/// Provides attestation recording, attestor registration/revocation,
/// SEP-10 JWT verification, rate limiting, replay-window protection and
/// admin/pause controls.
mod contract;
/// Error types for AnchorKit.
///
/// All errors are represented as [`AnchorKitError`], a unified base error type
/// carrying a [`code`](AnchorKitError::code), [`message`](AnchorKitError::message),
/// and optional [`context`](AnchorKitError::context).
///
/// The [`ErrorCode`] enum enumerates every distinct error kind. Use the
/// provided constructor helpers (e.g. [`AnchorKitError::already_initialized`])
/// to build errors without touching raw codes.
///
/// ## no-std / WASM builds
///
/// When compiled without the `std` feature (e.g. for Soroban WASM), heap
/// allocation via `alloc::string::String` is unavailable on the hot path.
/// In that case `AnchorKitError` stores only an `ErrorCode` discriminant and
/// a static message, and falls back to a code-only representation.
mod errors;
/// Contract events emitted by `AnchorKitContract`.
///
/// Events are published through `env.events().publish(...)` and mirror every
/// state mutation so off-chain indexers can react to attestation, session,
/// rate-limit and admin changes.
mod events;
/// Per-attestor rate limiting for attestation submissions.
///
/// Implements sliding-window counters stored in contract persistent storage
/// so the same window is observed across transactions.
mod rate_limiter;
/// Response schema validation for AnchorKit API responses.
///
/// Validates that anchor API responses contain all required fields before
/// returning them to the SDK consumer. Throws [`Error::ValidationError`] on mismatch.
mod response_validator;
/// Retry configuration for off-chain anchor requests.
///
/// Implements exponential backoff with a cap, jitter and a retryable-error
/// predicate used by the transport layer.
mod retry;
/// Typed storage keys for all contract state.
///
/// Using an enum prevents typos in raw string literals and makes every
/// storage access site self-documenting.
mod storage;
/// SEP-6 response types and canonical helpers.
///
/// Merges the raw SEP-6 response shapes with the validated, SDK-friendly
/// types re-exported from this crate.
mod types;

/// Unified error type for AnchorKit.
///
/// Every error carries a stable [`ErrorCode`](AnchorKitError::code), a human
/// readable message and optional context. Prefer the constructor helpers
/// over building errors directly.
pub use errors::{AnchorKitError, ErrorCode};

/// Lower-level error wrapper used by the off-chain SDK layer.
pub use errors::Error;

/// Per-attestor rate limiter with sliding-window counters.
pub use rate_limiter::{RateLimiter, RateLimitConfig, RateLimitState, RateLimitStatus};

/// Validators for anchor API responses.
pub use response_validator::{
    validate_anchor_info_response, validate_deposit_response, validate_quote_response,
    validate_withdraw_response, AnchorInfoResponse, QuoteResponse,
};

/// Retry-with-backoff helpers for transient anchor API failures.
pub use retry::{retry_with_backoff, is_retryable, RetryConfig, RetryErrorCode};

/// Deterministic payload hashing for attestation signatures.
pub use deterministic_hash::{compute_payload_hash, verify_payload_hash};

/// Stellar.toml / SEP-1 domain validation.
pub use domain_validator::validate_anchor_domain;

/// SEP-6 deposit/withdrawal exchange helpers.
pub use sep6::{
    fetch_transaction_status, initiate_deposit, initiate_withdrawal, withdraw_exchange,
    deposit_exchange, validate_amount, get_fee_estimate, get_transactions,
    RawDepositResponse, RawTransactionResponse, RawWithdrawalResponse, RawWithdrawExchangeResponse,
    TransactionKind, TransactionStatusResponse, RawDepositExchangeRequest, DepositExchangeResponse,
    AssetLimits, AnchorFeeData, FeeEstimate, FeeOperation, FeeSchedule, TransactionFilters,
};

/// Canonical SEP-6 response types.
pub use types::{DepositResponse, WithdrawalResponse, TransactionStatus};

/// Stateful session used by the off-chain SDK.
pub use types::Session;

/// Anchor metadata and routing request types.
pub use types::{AnchorMetadata, RoutingOptions, RoutingRequest};

/// Soroban contract entry point and free helper functions.
pub use contract::{AnchorKitContract, get_admin, get_endpoint, set_endpoint, get_attestation_count};

/// Events re-exported for downstream consumers.
pub use events::EndpointUpdated;

/// Transaction state tracking for long-lived off-chain flows.
pub use transaction_state_tracker::{
    StateTransition, TransactionState, TransactionStateRecord, TransactionStateTracker,
};

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
mod routing_score_tests;

#[cfg(test)]
mod credential_storage_tests;

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
mod issue_1355_1362_1364_tests;

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
mod sdk_config_tests;

#[cfg(test)]
mod streaming_flow_tests;

// Issue #959 – Holder pagination edge cases (empty, exact, partial, out-of-range).
#[cfg(test)]
mod holder_pagination_tests;

// Issue #958 – Forced-transfer audit record integrity.
#[cfg(test)]
mod forced_transfer_audit_tests;

// Issue #956 – Rent vs capital claim separation regression.
#[cfg(test)]
mod typed_distribution_separation_tests;
