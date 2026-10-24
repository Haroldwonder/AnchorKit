# Error Implementation Guide

This guide describes how to use AnchorKit's error handling system in your code.

## Overview

AnchorKit uses a unified error model with two main components:

- **`ErrorCode`** — An enum of all distinct error kinds (non-std / WASM safe)
- **`AnchorKitError`** — A result type carrying a code, message, and optional context

The `Error` type alias is provided for backward compatibility.

## Error Codes

Every error is identified by a numeric code. The canonical list appears in `src/errors.rs`:

| Code | Error Name | Message |
|------|-----------|---------|
| 1 | AlreadyInitialized | Contract is already initialized |
| 2 | AttestorAlreadyRegistered | Attestor is already registered |
| 3 | AttestorNotRegistered | Attestor is not registered |
| 4 | UnauthorizedAttestor | Attestor is not authorized |
| 5 | InvalidTimestamp | Timestamp is invalid |
| 6 | ReplayAttack | Replay attack detected |
| 7 | InvalidQuote | Quote is invalid |
| 8 | InvalidServiceType | Service type is invalid |
| 9 | InvalidTransactionIntent | Transaction intent is invalid |
| 10 | StaleQuote | Quote has expired |
| 11 | ComplianceNotMet | Compliance requirements not met |
| 12 | InvalidEndpointFormat | Endpoint format is invalid |
| 13 | NoQuotesAvailable | No quotes are available |
| 14 | ServicesNotConfigured | Services are not configured |
| 15 | ValidationError | Input validation failed |
| 16 | RateLimitExceeded | Rate limit exceeded |
| 17 | AttestationNotFound | Attestation not found |
| 18 | InvalidSep10Token | SEP-10 JWT is missing, expired, or invalid |
| 19 | StorageCorrupted | On-chain storage entry is corrupted or unreadable |
| 26 | NotInitialized | Contract is not initialized |
| 48 | CacheExpired | Cache entry has expired |
| 49 | CacheNotFound | Cache entry not found |
| 51 | AuditLogMaxSizeInvalid | max_audit_log_size must be at least 1 |
| 52 | PendingAdminAlreadyExists | An admin transfer is already pending |
| 53 | NoPendingAdmin | No pending admin transfer found |
| 54 | NotPendingAdmin | Caller is not the pending admin |
| 55 | SessionNotFound | Session not found |
| 56 | SessionExpired | Session has expired |
| 57 | MissingSigningKey | No SEP-10 verifying key registered for issuer |
| 58 | UnauthorizedProposeAdmin | Only admin can propose new admin |
| 59 | InvalidStrategy | Routing strategy symbol is not recognized |
| 60 | AttestationLimitReached | Attestation ID counter has reached its maximum value |
| 61 | AttestorCapExceeded | Maximum number of attestors has been reached |
| 62 | PathTraversalDetected | URL contains a path traversal sequence |
| 63 | InvalidAmount | Amount is outside the allowed min/max range for this asset |
| 64 | AttestationRevoked | Attestation has been revoked |
| 121 | AttestationExpired | Attestation has expired |
| 122 | ContractPaused | Contract is paused |
| 123 | AdminTransferPending | Admin transfer is already pending |

## Creating Errors

### Quick Start: Using Named Constructors

For most use cases, use the named constructor methods on `AnchorKitError`:

```rust
use crate::errors::AnchorKitError;

// Simple error with default message
let err = AnchorKitError::already_initialized();

// Other common patterns
let err = AnchorKitError::unauthorized_attestor();
let err = AnchorKitError::invalid_timestamp();
```

### Custom Messages

Use `AnchorKitError::new()` to provide a custom message with an error code:

```rust
let err = AnchorKitError::new(ErrorCode::InvalidQuote, "Quote amount is zero");
```

### Errors with Context

Use `AnchorKitError::with_context()` to attach additional diagnostic information:

```rust
let err = AnchorKitError::with_context(
    ErrorCode::ValidationError,
    "Schema validation failed",
    "missing field: transaction_id"
);
```

### Validation Errors

Validation errors commonly carry context describing what validation failed:

```rust
let err = AnchorKitError::validation_error("missing field: status");
```

### From Error Code

Create an error using a code's default message:

```rust
let err = AnchorKitError::from_code(ErrorCode::InvalidEndpointFormat);
```

## Displaying Errors

Errors implement `Display` and format as:

- Without context: `[E15] Input validation failed`
- With context: `[E15] Schema mismatch (field: transaction_id)`

The numeric code (e.g., `15`) is always included for logging and upstream handling.

## Error Type Structure

### In `std` Builds (default)

```rust
pub struct AnchorKitError {
    pub code: ErrorCode,
    pub message: String,              // Heap-allocated
    pub context: Option<String>,      // Heap-allocated when present
}
```

### In `no_std` / WASM Builds

```rust
#[cfg(not(feature = "std"))]
pub struct AnchorKitError {
    pub code: ErrorCode,
    pub message: &'static str,        // Thin reference, zero allocation
    pub context: Option<&'static str>, // Thin reference when present
}
```

The no-std variant allows errors to be created inside Soroban smart contracts (which run under `#![no_std]`) without heap allocation.

## Testing Errors

Basic error creation tests are included in `src/errors.rs`. When writing tests that check error handling, verify:

1. The error code is correct
2. The message is appropriate
3. Context is included when relevant

Example:

```rust
#[test]
fn test_validation_error() {
    let err = AnchorKitError::validation_error("missing field");
    assert_eq!(err.code, ErrorCode::ValidationError);
    assert_eq!(err.context, Some(String::from("missing field")));
}
```

## Backward Compatibility

`Error` is a type alias for `AnchorKitError`:

```rust
pub type Error = AnchorKitError;
```

Existing code using `Error::already_initialized()` will continue to work.

## No Error Variants in Enums

There is no `Error::InvalidConfig` variant in the `ErrorCode` enum. Use `AnchorKitError::invalid_config()` (a constructor) instead:

```rust
let err = AnchorKitError::invalid_config();
```

This creates an error with code `ValidationError` and message `"Invalid SDK configuration"`.

## Best Practices

1. **Use named constructors** for the common case — they have clear semantics
2. **Attach context** whenever callers need to debug why validation failed
3. **Panic with `panic_with_error!`** in Soroban contracts to return errors to the caller
4. **Log before panicking** if detailed diagnostics are needed
5. **Prefer default messages** unless you have a good reason to customize
