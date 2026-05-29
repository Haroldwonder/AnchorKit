# Retry & Exponential Backoff

> **Status: Implemented** — `src/retry.rs` (`retry_with_backoff`, `RetryConfig`, `is_retryable`)

AnchorKit includes robust retry logic with exponential backoff for handling transient failures in anchor communications.

## Implementation

```rust
use anchorkit::retry::{RetryConfig, retry_with_backoff, is_retryable};

let config = RetryConfig::default(); // max_attempts=3, base_delay_ms=100, max_delay_ms=5000, backoff_multiplier=2

let result = retry_with_backoff(
    &config,
    |attempt| fetch_stellar_toml(attempt),
    |e| is_retryable(e.code as u32),
    |delay_ms| std::thread::sleep(std::time::Duration::from_millis(delay_ms)),
);
```

## How `is_retryable` works

The helper `is_retryable(code)` classifies only transient, availability, and cache-related failure codes as retryable. It returns `true` when the numeric error code matches one of the following `ErrorCode` variants:

- `ServicesNotConfigured`
- `AttestationNotFound`
- `StaleQuote`
- `NoQuotesAvailable`
- `CacheExpired`
- `CacheNotFound`

All other `ErrorCode` values are considered non-retryable and should stop immediately.

> Note: `is_retryable(code)` only classifies the numeric error codes above. The lists below describe the broader retry categories used by AnchorKit, while the helper itself is implemented on a concise set of transient codes.

### Retryability summary

| Error code | Retryable? | Reason |
| --- | --- | --- |
| `AlreadyInitialized` | No | Permanent contract state error |
| `AttestorAlreadyRegistered` | No | Duplicate registration |
| `AttestorNotRegistered` | No | Invalid anchor state |
| `UnauthorizedAttestor` | No | Auth failure |
| `InvalidTimestamp` | No | Bad request data |
| `ReplayAttack` | No | Security violation |
| `InvalidQuote` | No | Bad quote data |
| `InvalidServiceType` | No | Invalid request parameter |
| `InvalidTransactionIntent` | No | Invalid transaction shape |
| `StaleQuote` | Yes | Quote expired; retry after refresh |
| `ComplianceNotMet` | No | Permanent policy failure |
| `InvalidEndpointFormat` | No | Bad endpoint configuration |
| `NoQuotesAvailable` | Yes | Transient quote availability |
| `ServicesNotConfigured` | Yes | Anchor not ready yet |
| `ValidationError` | No | Invalid response payload |
| `RateLimitExceeded` | No | Rate-limit failures are not retried by the default helper |
| `NotInitialized` | No | Contract not ready |
| `AttestationNotFound` | Yes | Data may become available soon |
| `InvalidSep10Token` | No | Auth failure |
| `StorageCorrupted` | No | Persistent on-chain corruption |
| `CacheExpired` | Yes | Refreshable cache state |
| `CacheNotFound` | Yes | Cache miss; can refresh |

## Features

- **Exponential Backoff**: Delays increase exponentially between retries (configurable multiplier)
- **Configurable Strategy**: Customize max attempts, base delay, max delay, and backoff multiplier
- **Smart Error Classification**: Automatically distinguishes retryable vs non-retryable errors
- **Network Failure Handling**: Retries on transport errors and timeouts
- **Rate Limit Handling**: Backs off when encountering 429 rate limit responses
- **5xx Response Handling**: Retries on server errors

## Retryable Errors

Common transient errors for retry policies:

### Network Failures
- `TransportError` - General network/transport failures
- `TransportTimeout` - Request timeouts
- `EndpointNotFound` - Endpoint temporarily unavailable

### Rate Limiting
- `RateLimitExceeded` - Rate limit exceeded (429); handled via custom backoff policies rather than the default helper
- `ProtocolRateLimitExceeded` - Protocol-level rate limiting

### Transient Failures
- `ServicesNotConfigured` - Service temporarily unavailable
- `AttestationNotFound` - Attestation not yet available
- `SessionNotFound` - Session not yet created
- `StaleQuote` - Quote expired, can fetch fresh
- `NoQuotesAvailable` - No quotes currently available
- `AnchorMetadataNotFound` - Metadata not yet cached
- `CacheExpired` - Cache expired, can refresh
- `CacheNotFound` - Cache miss, can fetch

## Non-Retryable Errors

The following errors are NOT retried (permanent failures):

- `InvalidConfig` - Configuration error
- `UnauthorizedAttestor` - Authentication failure
- `TransportUnauthorized` - Authorization failure
- `InvalidQuote` - Invalid quote data
- `InvalidTimestamp` - Invalid timestamp
- `ReplayAttack` - Replay attack detected
- `ComplianceNotMet` - Compliance check failed
- `CredentialExpired` - Expired credentials
- `ProtocolError` - Protocol violation
- `ProtocolInvalidPayload` - Invalid payload format

## Configuration

### Default Configuration

```rust
use anchorkit::retry::RetryConfig;

let config = RetryConfig::default();
// max_attempts: 3
// base_delay_ms: 100
// max_delay_ms: 5000
// backoff_multiplier: 2
```

### Custom Configuration

```rust
use anchorkit::retry::RetryConfig;

// Aggressive: many attempts, short delays
let aggressive = RetryConfig::new(
    10,    // max_attempts
    10,    // base_delay_ms
    1000,  // max_delay_ms
    2      // backoff_multiplier
);

// Conservative: few attempts, long delays
let conservative = RetryConfig::new(
    3,     // max_attempts
    1000,  // base_delay_ms
    10000, // max_delay_ms
    3      // backoff_multiplier
);

// Custom for rate limiting: longer delays
let rate_limit = RetryConfig::new(
    5,     // max_attempts
    500,   // base_delay_ms
    30000, // max_delay_ms (30 seconds)
    3      // backoff_multiplier
);
```

## Usage Examples

### Non-retryable errors stop immediately

```rust
use anchorkit::retry::{retry_with_backoff, RetryConfig, is_retryable};
use anchorkit::{AnchorKitError, ErrorCode};

let config = RetryConfig::default();
let mut attempts = 0;

let result = retry_with_backoff(
    &config,
    |_| {
        attempts += 1;
        Err::<(), _>(AnchorKitError::invalid_quote())
    },
    |e: &AnchorKitError| is_retryable(e.code as u32),
    |_| {},
);

assert_eq!(attempts, 1);
assert!(matches!(result, Err(err) if err.code == ErrorCode::InvalidQuote));
```

### Basic Retry

```rust
use anchorkit::retry::{RetryConfig, RetryEngine};

let config = RetryConfig::default();
let engine = RetryEngine::new(config);

let result = engine.execute(|attempt| {
    // Your operation here
    // Returns Ok(value) on success or Err(error) on failure
    make_network_request()
});

if result.is_success() {
    println!("Success after {} attempts", result.attempts);
    println!("Total delay: {}ms", result.total_delay_ms);
    let value = result.value.unwrap();
} else {
    println!("Failed after {} attempts", result.attempts);
    let error = result.error.unwrap();
}
```

### With Transport Layer

```rust
use anchorkit::{
    retry::{RetryConfig, RetryEngine},
    transport::{AnchorTransport, TransportRequest, TransportResponse},
};

let config = RetryConfig::new(5, 100, 5000, 2);
let engine = RetryEngine::new(config);

let result = engine.execute(|attempt| {
    println!("Attempt {}", attempt + 1);
    
    let request = TransportRequest::GetQuote {
        endpoint: endpoint.clone(),
        base_asset: base.clone(),
        quote_asset: quote.clone(),
        amount: 1000,
    };
    
    transport.send_request(&env, request)
});

match result.value {
    Some(TransportResponse::Quote(quote)) => {
        println!("Got quote: rate={}", quote.rate);
    }
    _ => {
        println!("Failed to get quote: {:?}", result.error);
    }
}
```

### Rate Limit Handling

```rust
use anchorkit::retry::{RetryConfig, RetryEngine};

// Configure longer delays for rate limiting
let config = RetryConfig::new(
    5,     // max_attempts
    500,   // base_delay_ms
    30000, // max_delay_ms (30 seconds)
    3      // backoff_multiplier (aggressive backoff)
);

let engine = RetryEngine::new(config);

let result = engine.execute(|attempt| {
    match make_api_call() {
        Ok(response) => Ok(response),
        Err(Error::RateLimitExceeded) => {
            println!("Rate limited, backing off...");
            Err(Error::RateLimitExceeded)
        }
        Err(e) => Err(e),
    }
});
```

### Network Failure Recovery

```rust
use anchorkit::retry::{RetryConfig, RetryEngine};

let config = RetryConfig::new(4, 100, 5000, 2);
let engine = RetryEngine::new(config);

let result = engine.execute(|attempt| {
    match fetch_anchor_data() {
        Ok(data) => Ok(data),
        Err(Error::TransportTimeout) => {
            println!("Timeout on attempt {}, retrying...", attempt + 1);
            Err(Error::TransportTimeout)
        }
        Err(Error::TransportError) => {
            println!("Network error on attempt {}, retrying...", attempt + 1);
            Err(Error::TransportError)
        }
        Err(e) => Err(e),
    }
});
```

## Backoff Timing

The delay between retries follows an exponential pattern:

```
Attempt 0: 0ms (immediate)
Attempt 1: base_delay_ms
Attempt 2: base_delay_ms * multiplier^1
Attempt 3: base_delay_ms * multiplier^2
Attempt 4: base_delay_ms * multiplier^3
...
```

### Example with Default Config

```
max_attempts: 3
base_delay_ms: 100
backoff_multiplier: 2

Attempt 0: 0ms
Attempt 1: 100ms
Attempt 2: 200ms
Total: 300ms
```

### Example with Aggressive Config

```
max_attempts: 5
base_delay_ms: 50
backoff_multiplier: 3

Attempt 0: 0ms
Attempt 1: 50ms
Attempt 2: 150ms
Attempt 3: 450ms
Attempt 4: 1350ms
Total: 2000ms
```

### Example with Rate Limit Config

```
max_attempts: 5
base_delay_ms: 500
backoff_multiplier: 3
max_delay_ms: 30000

Attempt 0: 0ms
Attempt 1: 500ms
Attempt 2: 1500ms
Attempt 3: 4500ms
Attempt 4: 13500ms
Total: 20000ms
```

## Best Practices

### 1. Choose Appropriate Strategy

- **Network failures**: Moderate attempts (3-5), short delays (100-500ms)
- **Rate limiting**: Fewer attempts (3-4), longer delays (500-1000ms), higher multiplier (3-4)
- **Server errors (5xx)**: Moderate attempts (3-5), moderate delays (200-500ms)

### 2. Set Reasonable Max Delays

```rust
// For user-facing operations: keep max delay low
let user_facing = RetryConfig::new(3, 100, 2000, 2);

// For background operations: can use longer delays
let background = RetryConfig::new(5, 500, 30000, 3);
```

### 3. Monitor Retry Metrics

```rust
let result = engine.execute(|attempt| {
    // Your operation
});

// Log retry metrics
println!("Attempts: {}", result.attempts);
println!("Total delay: {}ms", result.total_delay_ms);
println!("Success: {}", result.is_success());
```

### 4. Handle Non-Retryable Errors

```rust
let result = engine.execute(|attempt| {
    match operation() {
        Err(Error::InvalidConfig) => {
            // Non-retryable, fail fast
            return Err(Error::InvalidConfig);
        }
        Err(Error::TransportTimeout) => {
            // Retryable, will retry
            return Err(Error::TransportTimeout);
        }
        Ok(value) => Ok(value),
    }
});
```

## Testing

The retry logic includes comprehensive tests:

```bash
# Run all retry tests
cargo test retry --lib

# Run specific test categories
cargo test test_network_failure --lib
cargo test test_rate_limit --lib
cargo test test_exponential_backoff --lib
```

## Integration with Transport

The retry logic is designed to work seamlessly with the transport layer:

```rust
use anchorkit::{
    retry::{RetryConfig, RetryEngine},
    transport::{AnchorTransport, MockTransport, TransportRequest},
};

let mut transport = MockTransport::new();
let config = RetryConfig::default();
let engine = RetryEngine::new(config);

let result = engine.execute(|_| {
    transport.send_request(&env, request.clone())
});
```

## Error Classification

To check if an error is retryable:

```rust
use anchorkit::retry::is_retryable_error;
use anchorkit::errors::Error;

if is_retryable_error(&Error::TransportTimeout) {
    println!("This error will be retried");
}

if !is_retryable_error(&Error::InvalidConfig) {
    println!("This error will NOT be retried");
}
```

## Summary

- ✅ Exponential backoff with configurable parameters
- ✅ Smart error classification (retryable vs non-retryable)
- ✅ Network failure handling (timeouts, transport errors)
- ✅ Rate limit handling (429 responses)
- ✅ 5xx server error handling
- ✅ Configurable retry strategies
- ✅ Comprehensive test coverage
- ✅ Integration with transport layer
