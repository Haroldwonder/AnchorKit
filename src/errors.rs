//! Error types for AnchorKit
//!
//! All errors are represented as [`AnchorKitError`], a unified base error type
//! carrying a [`code`](AnchorKitError::code), [`message`](AnchorKitError::message),
//! and optional [`context`](AnchorKitError::context).
//!
//! The [`ErrorCode`] enum enumerates every distinct error kind. Use the
//! provided constructor helpers (e.g. [`AnchorKitError::already_initialized`])
//! to build errors without touching raw codes.
//!
//! ## no-std / WASM builds
//!
//! When compiled without the `std` feature (e.g. for Soroban WASM), heap
//! allocation via `alloc::string::String` is unavailable on the hot path.
//! In that case `AnchorKitError` stores only an `ErrorCode` discriminant and
//! a `&'static str` message slice — no heap allocation required.
//! The full `String`-based struct is only compiled when `feature = "std"` is
//! active (the default for host-side / test builds).

use soroban_sdk::contracterror;

#[cfg(feature = "std")]
extern crate alloc;
#[cfg(feature = "std")]
use alloc::string::String;

// ---------------------------------------------------------------------------
// ErrorCode — the canonical list of all error kinds (replaces the old Error enum)
// ---------------------------------------------------------------------------

/// Numeric error codes for every AnchorKit error kind.
///
/// The `#[contracterror]` attribute keeps Soroban on-chain compatibility.
///
/// ## Migration note
///
/// Prior to this fix the codes were non-contiguous: values 1-19 were followed
/// by a gap (20-47) and then 48-54, with `NotInitialized` at 101.
/// All codes have been renumbered to the contiguous range **1-30**.
/// Clients that matched on raw numeric values must update their mappings:
///
/// | Old code | New code | Name                    |
/// |----------|----------|-------------------------|
/// | 1        | 1        | AlreadyInitialized      |
/// | 2        | 2        | AttestorAlreadyRegistered |
/// | 3        | 3        | AttestorNotRegistered   |
/// | 4        | 4        | UnauthorizedAttestor    |
/// | 5        | 5        | InvalidTimestamp        |
/// | 6        | 6        | ReplayAttack            |
/// | 7        | 7        | InvalidQuote            |
/// | 8        | 8        | InvalidServiceType      |
/// | 9        | 9        | InvalidTransactionIntent |
/// | 10       | 10       | StaleQuote              |
/// | 11       | 11       | ComplianceNotMet        |
/// | 12       | 12       | InvalidEndpointFormat   |
/// | 13       | 13       | NoQuotesAvailable       |
/// | 14       | 14       | ServicesNotConfigured   |
/// | 15       | 15       | ValidationError         |
/// | 16       | 16       | RateLimitExceeded       |
/// | 17       | 17       | AttestationNotFound     |
/// | 18       | 18       | InvalidSep10Token       |
/// | 19       | 19       | StorageCorrupted        |
/// | 48       | 20       | CacheExpired            |
/// | 49       | 21       | CacheNotFound           |
/// | 51       | 22       | AuditLogMaxSizeInvalid  |
/// | 52       | 23       | UnauthorizedProposeAdmin |
/// | 53       | 24       | NoPendingAdmin          |
/// | 54       | 25       | NotPendingAdmin         |
/// | 101      | 26       | NotInitialized          |
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ErrorCode {
    AlreadyInitialized = 1,
    AttestorAlreadyRegistered = 2,
    AttestorNotRegistered = 3,
    UnauthorizedAttestor = 4,
    InvalidTimestamp = 5,
    ReplayAttack = 6,
    InvalidQuote = 7,
    InvalidServiceType = 8,
    InvalidTransactionIntent = 9,
    StaleQuote = 10,
    ComplianceNotMet = 11,
    InvalidEndpointFormat = 12,
    NoQuotesAvailable = 13,
    ServicesNotConfigured = 14,
    ValidationError = 15,
    RateLimitExceeded = 16,
    AttestationNotFound = 17,
    InvalidSep10Token = 18,
    StorageCorrupted = 19,
    CacheExpired = 48,
    CacheNotFound = 49,
    AuditLogMaxSizeInvalid = 51,
    PendingAdminAlreadyExists = 52,
    NoPendingAdmin = 53,
    NotPendingAdmin = 54,
    SessionNotFound = 55,
    SessionExpired = 56,
    MissingSigningKey = 57,
}

impl ErrorCode {
    /// Returns the canonical human-readable message for this error code.
    pub fn default_message(&self) -> &'static str {
        match self {
            ErrorCode::AlreadyInitialized => "Contract is already initialized",
            ErrorCode::AttestorAlreadyRegistered => "Attestor is already registered",
            ErrorCode::AttestorNotRegistered => "Attestor is not registered",
            ErrorCode::UnauthorizedAttestor => "Attestor is not authorized",
            ErrorCode::InvalidTimestamp => "Timestamp is invalid",
            ErrorCode::ReplayAttack => "Replay attack detected",
            ErrorCode::InvalidQuote => "Quote is invalid",
            ErrorCode::InvalidServiceType => "Service type is invalid",
            ErrorCode::InvalidTransactionIntent => "Transaction intent is invalid",
            ErrorCode::StaleQuote => "Quote has expired",
            ErrorCode::ComplianceNotMet => "Compliance requirements not met",
            ErrorCode::InvalidEndpointFormat => "Endpoint format is invalid",
            ErrorCode::NoQuotesAvailable => "No quotes are available",
            ErrorCode::ServicesNotConfigured => "Services are not configured",
            ErrorCode::ValidationError => "Response schema validation failed",
            ErrorCode::RateLimitExceeded => "Rate limit exceeded",
            ErrorCode::NotInitialized => "Contract is not initialized",
            ErrorCode::AttestationNotFound => "Attestation not found",
            ErrorCode::InvalidSep10Token => "SEP-10 JWT is missing, expired, or invalid",
            ErrorCode::StorageCorrupted => "On-chain storage entry is corrupted or unreadable",
            ErrorCode::CacheExpired => "Cache entry has expired",
            ErrorCode::CacheNotFound => "Cache entry not found",
            ErrorCode::AuditLogMaxSizeInvalid => "max_audit_log_size must be at least 1",
            ErrorCode::PendingAdminAlreadyExists => "An admin transfer is already pending",
            ErrorCode::NoPendingAdmin => "No pending admin transfer found",
            ErrorCode::NotPendingAdmin => "Caller is not the pending admin",
            ErrorCode::SessionNotFound => "Session not found",
            ErrorCode::SessionExpired => "Session has expired",
            ErrorCode::MissingSigningKey => "Anchor TOML does not publish a signing key",
        }
    }

}

// ---------------------------------------------------------------------------
// AnchorKitError — the unified base error type
//
// std build  : full struct with heap-allocated String fields (message + context)
// no-std/WASM: thin wrapper around ErrorCode + &'static str — zero heap alloc
// ---------------------------------------------------------------------------

/// The base error type for all AnchorKit errors.
///
/// **std builds** (default): carries a heap-allocated `message` and optional
/// `context` string for rich diagnostics.
///
/// **no-std / WASM builds** (`wasm` feature, no `std`): stores only the
/// [`ErrorCode`] discriminant and a `&'static str` message slice so that no
/// heap allocation is required on the hot path inside a Soroban contract.
#[cfg(feature = "std")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorKitError {
    pub code: ErrorCode,
    pub message: String,
    pub context: Option<String>,
}

/// Thin no-std / WASM variant — no heap allocation.
#[cfg(not(feature = "std"))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorKitError {
    pub code: ErrorCode,
    pub message: &'static str,
    pub context: Option<&'static str>,
}

// ---------------------------------------------------------------------------
// std implementation
// ---------------------------------------------------------------------------

#[cfg(feature = "std")]
impl AnchorKitError {
    /// Create a new error with a custom message and no context.
    pub fn new(code: ErrorCode, message: &str) -> Self {
        AnchorKitError {
            code,
            message: String::from(message),
            context: None,
        }
    }

    /// Create a new error with a custom message and context detail.
    pub fn with_context(code: ErrorCode, message: &str, context: &str) -> Self {
        AnchorKitError {
            code,
            message: String::from(message),
            context: Some(String::from(context)),
        }
    }

    /// Create an error using the default message for the given code.
    pub fn from_code(code: ErrorCode) -> Self {
        let message = code.default_message();
        AnchorKitError::new(code, message)
    }

    pub fn already_initialized() -> Self { Self::from_code(ErrorCode::AlreadyInitialized) }
    pub fn attestor_already_registered() -> Self { Self::from_code(ErrorCode::AttestorAlreadyRegistered) }
    pub fn attestor_not_registered() -> Self { Self::from_code(ErrorCode::AttestorNotRegistered) }
    pub fn unauthorized_attestor() -> Self { Self::from_code(ErrorCode::UnauthorizedAttestor) }
    pub fn invalid_timestamp() -> Self { Self::from_code(ErrorCode::InvalidTimestamp) }
    pub fn replay_attack() -> Self { Self::from_code(ErrorCode::ReplayAttack) }
    pub fn invalid_quote() -> Self { Self::from_code(ErrorCode::InvalidQuote) }
    pub fn invalid_service_type() -> Self { Self::from_code(ErrorCode::InvalidServiceType) }
    pub fn invalid_transaction_intent() -> Self { Self::from_code(ErrorCode::InvalidTransactionIntent) }
    pub fn stale_quote() -> Self { Self::from_code(ErrorCode::StaleQuote) }
    pub fn compliance_not_met() -> Self { Self::from_code(ErrorCode::ComplianceNotMet) }
    pub fn invalid_endpoint_format() -> Self { Self::from_code(ErrorCode::InvalidEndpointFormat) }
    pub fn no_quotes_available() -> Self { Self::from_code(ErrorCode::NoQuotesAvailable) }
    pub fn services_not_configured() -> Self { Self::from_code(ErrorCode::ServicesNotConfigured) }
    pub fn not_initialized() -> Self { Self::from_code(ErrorCode::NotInitialized) }
    pub fn attestation_not_found() -> Self { Self::from_code(ErrorCode::AttestationNotFound) }
    pub fn invalid_sep10_token() -> Self { Self::from_code(ErrorCode::InvalidSep10Token) }
    pub fn rate_limit_exceeded() -> Self { Self::from_code(ErrorCode::RateLimitExceeded) }
    pub fn storage_corrupted() -> Self { Self::from_code(ErrorCode::StorageCorrupted) }
    pub fn cache_expired() -> Self { Self::from_code(ErrorCode::CacheExpired) }
    pub fn cache_not_found() -> Self { Self::from_code(ErrorCode::CacheNotFound) }
    pub fn missing_signing_key() -> Self { Self::from_code(ErrorCode::MissingSigningKey) }

    pub fn validation_error(context: &str) -> Self {
        Self::with_context(ErrorCode::ValidationError, ErrorCode::ValidationError.default_message(), context)
    }
}

#[cfg(feature = "std")]
impl core::fmt::Display for AnchorKitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.context {
            Some(ctx) => write!(f, "[E{}] {} ({})", self.code as u32, self.message, ctx),
            None => write!(f, "[E{}] {}", self.code as u32, self.message),
        }
    }
}

// ---------------------------------------------------------------------------
// no-std / WASM implementation — zero heap allocation
// ---------------------------------------------------------------------------

    pub fn cache_not_found() -> Self {
        Self::from_code(ErrorCode::CacheNotFound)
    }

    pub fn already_initialized() -> Self { Self::from_code(ErrorCode::AlreadyInitialized) }
    pub fn attestor_already_registered() -> Self { Self::from_code(ErrorCode::AttestorAlreadyRegistered) }
    pub fn attestor_not_registered() -> Self { Self::from_code(ErrorCode::AttestorNotRegistered) }
    pub fn unauthorized_attestor() -> Self { Self::from_code(ErrorCode::UnauthorizedAttestor) }
    pub fn invalid_timestamp() -> Self { Self::from_code(ErrorCode::InvalidTimestamp) }
    pub fn replay_attack() -> Self { Self::from_code(ErrorCode::ReplayAttack) }
    pub fn invalid_quote() -> Self { Self::from_code(ErrorCode::InvalidQuote) }
    pub fn invalid_service_type() -> Self { Self::from_code(ErrorCode::InvalidServiceType) }
    pub fn invalid_transaction_intent() -> Self { Self::from_code(ErrorCode::InvalidTransactionIntent) }
    pub fn stale_quote() -> Self { Self::from_code(ErrorCode::StaleQuote) }
    pub fn compliance_not_met() -> Self { Self::from_code(ErrorCode::ComplianceNotMet) }
    pub fn invalid_endpoint_format() -> Self { Self::from_code(ErrorCode::InvalidEndpointFormat) }
    pub fn no_quotes_available() -> Self { Self::from_code(ErrorCode::NoQuotesAvailable) }
    pub fn services_not_configured() -> Self { Self::from_code(ErrorCode::ServicesNotConfigured) }
    pub fn not_initialized() -> Self { Self::from_code(ErrorCode::NotInitialized) }
    pub fn attestation_not_found() -> Self { Self::from_code(ErrorCode::AttestationNotFound) }
    pub fn invalid_sep10_token() -> Self { Self::from_code(ErrorCode::InvalidSep10Token) }
    pub fn rate_limit_exceeded() -> Self { Self::from_code(ErrorCode::RateLimitExceeded) }
    pub fn storage_corrupted() -> Self { Self::from_code(ErrorCode::StorageCorrupted) }
    pub fn cache_expired() -> Self { Self::from_code(ErrorCode::CacheExpired) }
    pub fn cache_not_found() -> Self { Self::from_code(ErrorCode::CacheNotFound) }
    pub fn missing_signing_key() -> Self { Self::from_code(ErrorCode::MissingSigningKey) }
    pub fn validation_error(_context: &str) -> Self { Self::from_code(ErrorCode::ValidationError) }
}

#[cfg(not(feature = "std"))]
impl core::fmt::Display for AnchorKitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[E{}] {}", self.code as u32, self.message)
    }
}

// ---------------------------------------------------------------------------
// Backward-compat type alias so existing code using `Error` still compiles
// ---------------------------------------------------------------------------

/// Backward-compatible alias. Prefer [`AnchorKitError`] for new code.
pub type Error = AnchorKitError;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_from_code_sets_message() {
        let err = AnchorKitError::from_code(ErrorCode::AlreadyInitialized);
        assert_eq!(err.code, ErrorCode::AlreadyInitialized);
        assert_eq!(err.message, "Contract is already initialized");
        assert!(err.context.is_none());
    }

    #[test]
    fn test_new_custom_message() {
        let err = AnchorKitError::new(ErrorCode::InvalidQuote, "Quote amount is zero");
        assert_eq!(err.code, ErrorCode::InvalidQuote);
        assert_eq!(err.message, "Quote amount is zero");
        assert!(err.context.is_none());
    }

    #[test]
    fn test_with_context() {
        let err = AnchorKitError::with_context(
            ErrorCode::ValidationError,
            "Schema mismatch",
            "field: transaction_id",
        );
        assert_eq!(err.code, ErrorCode::ValidationError);
        assert_eq!(err.message, "Schema mismatch");
        assert_eq!(err.context, Some(String::from("field: transaction_id")));
    }

    #[test]
    fn test_named_constructors() {
        assert_eq!(AnchorKitError::already_initialized().code, ErrorCode::AlreadyInitialized);
        assert_eq!(AnchorKitError::attestor_already_registered().code, ErrorCode::AttestorAlreadyRegistered);
        assert_eq!(AnchorKitError::attestor_not_registered().code, ErrorCode::AttestorNotRegistered);
        assert_eq!(AnchorKitError::unauthorized_attestor().code, ErrorCode::UnauthorizedAttestor);
        assert_eq!(AnchorKitError::invalid_timestamp().code, ErrorCode::InvalidTimestamp);
        assert_eq!(AnchorKitError::replay_attack().code, ErrorCode::ReplayAttack);
        assert_eq!(AnchorKitError::invalid_quote().code, ErrorCode::InvalidQuote);
        assert_eq!(AnchorKitError::invalid_service_type().code, ErrorCode::InvalidServiceType);
        assert_eq!(AnchorKitError::invalid_transaction_intent().code, ErrorCode::InvalidTransactionIntent);
        assert_eq!(AnchorKitError::stale_quote().code, ErrorCode::StaleQuote);
        assert_eq!(AnchorKitError::compliance_not_met().code, ErrorCode::ComplianceNotMet);
        assert_eq!(AnchorKitError::invalid_endpoint_format().code, ErrorCode::InvalidEndpointFormat);
        assert_eq!(AnchorKitError::no_quotes_available().code, ErrorCode::NoQuotesAvailable);
        assert_eq!(AnchorKitError::services_not_configured().code, ErrorCode::ServicesNotConfigured);
        assert_eq!(AnchorKitError::invalid_sep10_token().code, ErrorCode::InvalidSep10Token);
        assert_eq!(AnchorKitError::cache_expired().code, ErrorCode::CacheExpired);
        assert_eq!(AnchorKitError::cache_not_found().code, ErrorCode::CacheNotFound);
        assert_eq!(AnchorKitError::audit_log_max_size_invalid().code, ErrorCode::AuditLogMaxSizeInvalid);
        assert_eq!(AnchorKitError::unauthorized_propose_admin().code, ErrorCode::UnauthorizedProposeAdmin);
        assert_eq!(AnchorKitError::no_pending_admin().code, ErrorCode::NoPendingAdmin);
        assert_eq!(AnchorKitError::not_pending_admin().code, ErrorCode::NotPendingAdmin);
    }

    #[test]
    fn test_validation_error_has_context() {
        let err = AnchorKitError::validation_error("missing field: status");
        assert_eq!(err.code, ErrorCode::ValidationError);
        assert_eq!(err.context, Some(String::from("missing field: status")));
    }

    #[test]
    fn test_error_code_default_messages_are_non_empty() {
let codes = [
            ErrorCode::AlreadyInitialized,
            ErrorCode::PendingAdminAlreadyExists,
            ErrorCode::NoPendingAdmin,
            ErrorCode::NotPendingAdmin,
            ErrorCode::InvalidStrategy,
            ErrorCode::AttestorAlreadyRegistered,
            ErrorCode::AttestorNotRegistered,
            ErrorCode::UnauthorizedAttestor,
            ErrorCode::InvalidTimestamp,
            ErrorCode::ReplayAttack,
            ErrorCode::InvalidQuote,
            ErrorCode::InvalidServiceType,
            ErrorCode::InvalidTransactionIntent,
            ErrorCode::StaleQuote,
            ErrorCode::ComplianceNotMet,
            ErrorCode::InvalidEndpointFormat,
            ErrorCode::NoQuotesAvailable,
            ErrorCode::ServicesNotConfigured,
            ErrorCode::ValidationError,
            ErrorCode::RateLimitExceeded,
            ErrorCode::NotInitialized,
            ErrorCode::AttestationNotFound,
            ErrorCode::InvalidSep10Token,
            ErrorCode::StorageCorrupted,
            ErrorCode::CacheExpired,
            ErrorCode::CacheNotFound,
            ErrorCode::SessionNotFound,
            ErrorCode::SessionExpired,
            ErrorCode::MissingSigningKey,
        ];
        for code in codes {
            assert!(!code.default_message().is_empty());
        }
    }

    #[test]
    fn test_type_alias_error_works() {
        // Ensure backward-compat alias compiles and behaves identically
        let err: Error = AnchorKitError::from_code(ErrorCode::InvalidEndpointFormat);
        assert_eq!(err.code, ErrorCode::InvalidEndpointFormat);
    }

    #[test]
    fn test_errors_are_cloneable_and_comparable() {
        let a = AnchorKitError::from_code(ErrorCode::StaleQuote);
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn test_display_format_without_context() {
        let err = AnchorKitError::new(ErrorCode::RateLimitExceeded, "Rate limit exceeded");
        let formatted = alloc::format!("{}", err);
        assert_eq!(formatted, "[E16] Rate limit exceeded");
    }

    #[test]
    fn test_display_format_with_context() {
        let err = AnchorKitError::with_context(ErrorCode::ValidationError, "Schema mismatch", "field: transaction_id");
        let formatted = alloc::format!("{}", err);
        assert_eq!(formatted, "[E15] Schema mismatch (field: transaction_id)");
    }
}

