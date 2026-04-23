//! Error types for AnchorKit
//!
//! All errors are represented as [`AnchorKitError`], a unified base error type
//! carrying a [`code`](AnchorKitError::code), [`message`](AnchorKitError::message),
//! and optional [`context`](AnchorKitError::context).
//!
//! The [`ErrorCode`] enum enumerates every distinct error kind. Use the
//! provided constructor helpers (e.g. [`AnchorKitError::already_initialized`])
//! to build errors without touching raw codes.


extern crate alloc;

use alloc::string::String;
use soroban_sdk::contracterror;

// ---------------------------------------------------------------------------
// ErrorCode — the canonical list of all error kinds (replaces the old Error enum)
// ---------------------------------------------------------------------------

/// Numeric error codes for every AnchorKit error kind.
///
/// The `#[contracterror]` attribute keeps Soroban on-chain compatibility.
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
    NotInitialized = 101,
    AttestationNotFound = 17,
    InvalidSep10Token = 18,
    StorageCorrupted = 50,
    CacheExpired = 48,
StorageCorrupted = 19,
    CacheNotFound = 49,
    StorageCorrupted = 50,
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
        }
    }
}

// ---------------------------------------------------------------------------
// AnchorKitError — the unified base error type
// ---------------------------------------------------------------------------

/// The base error type for all AnchorKit errors.
///
/// Every error carries:
/// - `code`: the [`ErrorCode`] identifying the error kind
/// - `message`: a human-readable description
/// - `context`: optional extra detail (field name, received value, etc.)
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorKitError {
    pub code: ErrorCode,
    pub message: String,
    pub context: Option<String>,
}

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

    // ------------------------------------------------------------------
    // Named constructors — one per ErrorCode variant
    // ------------------------------------------------------------------

    pub fn already_initialized() -> Self {
        Self::from_code(ErrorCode::AlreadyInitialized)
    }

    pub fn attestor_already_registered() -> Self {
        Self::from_code(ErrorCode::AttestorAlreadyRegistered)
    }

    pub fn attestor_not_registered() -> Self {
        Self::from_code(ErrorCode::AttestorNotRegistered)
    }

    pub fn unauthorized_attestor() -> Self {
        Self::from_code(ErrorCode::UnauthorizedAttestor)
    }

    pub fn invalid_timestamp() -> Self {
        Self::from_code(ErrorCode::InvalidTimestamp)
    }

    pub fn replay_attack() -> Self {
        Self::from_code(ErrorCode::ReplayAttack)
    }

    pub fn invalid_quote() -> Self {
        Self::from_code(ErrorCode::InvalidQuote)
    }

    pub fn invalid_service_type() -> Self {
        Self::from_code(ErrorCode::InvalidServiceType)
    }

    pub fn invalid_transaction_intent() -> Self {
        Self::from_code(ErrorCode::InvalidTransactionIntent)
    }

    pub fn stale_quote() -> Self {
        Self::from_code(ErrorCode::StaleQuote)
    }

    pub fn compliance_not_met() -> Self {
        Self::from_code(ErrorCode::ComplianceNotMet)
    }

    pub fn invalid_endpoint_format() -> Self {
        Self::from_code(ErrorCode::InvalidEndpointFormat)
    }

    pub fn no_quotes_available() -> Self {
        Self::from_code(ErrorCode::NoQuotesAvailable)
    }

    pub fn services_not_configured() -> Self {
        Self::from_code(ErrorCode::ServicesNotConfigured)
    }

    pub fn not_initialized() -> Self {
        Self::from_code(ErrorCode::NotInitialized)
    }

    pub fn attestation_not_found() -> Self {
        Self::from_code(ErrorCode::AttestationNotFound)
    }

    pub fn invalid_sep10_token() -> Self {
        Self::from_code(ErrorCode::InvalidSep10Token)
    }

    pub fn validation_error(context: &str) -> Self {
        Self::with_context(ErrorCode::ValidationError, ErrorCode::ValidationError.default_message(), context)
    }

    pub fn rate_limit_exceeded() -> Self {
        Self::from_code(ErrorCode::RateLimitExceeded)
    }

    pub fn storage_corrupted() -> Self {
        Self::from_code(ErrorCode::StorageCorrupted)
    }

    pub fn cache_expired() -> Self {
        Self::from_code(ErrorCode::CacheExpired)
    }

    pub fn cache_not_found() -> Self {
        Self::from_code(ErrorCode::CacheNotFound)
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

#[cfg(test)]
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
}
