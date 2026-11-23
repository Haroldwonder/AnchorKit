# Retry & Exponential Backoff - Quick Reference

## Quick Start

```rust
use anchorkit::retry::{retry_with_backoff, RetryConfig};

// Use default config (3 attempts, 100ms initial delay)
let config = RetryConfig::default();

// Execute with retry
let result = retry_with_backoff(config, |attempt| {
    make_network_request()
});
```

## Common Configurations

### Network Failures
```rust
RetryConfig::new(5, 100, 5000, 2)
// 5 attempts, 100ms initial, 5s max, 2x multiplier
// Timing: 0, 100, 200, 400, 800ms
```

### Rate Limiting (429)
```rust
RetryConfig::new(5, 500, 30000, 3)
// 5 attempts, 500ms initial, 30s max, 3x multiplier
// Timing: 0, 500, 1500, 4500, 13500ms
```

### Server Errors (5xx)
```rust
RetryConfig::new(4, 200, 10000, 2)
// 4 attempts, 200ms initial, 10s max, 2x multiplier
// Timing: 0, 200, 400, 800ms
```

## Retryable Errors

✅ **Will Retry:**
- `TransportError` - Network failures
- `TransportTimeout` - Timeouts
- `RateLimitExceeded` - Rate limiting (429)
- `ProtocolRateLimitExceeded` - Protocol rate limits
- `EndpointNotFound` - Endpoint unavailable
- `StaleQuote` - Quote expired
- `CacheExpired` - Cache expired

❌ **Won't Retry:**
- `InvalidConfig` - Configuration error
- `TransportUnauthorized` - Auth failure (401/403)
- `InvalidQuote` - Validation error
- `ReplayAttack` - Security violation
- `ComplianceNotMet` - Compliance failure
- `ProtocolError` - Protocol violation

## Usage Patterns

### Basic Retry
```rust
let result = retry_with_backoff(config, |_| {
    operation()
});

if result.is_success() {
    let value = result.value.unwrap();
}
```

### With Logging
```rust
let result = retry_with_backoff(config, |attempt| {
    println!("Attempt {}", attempt + 1);
    operation()
});

println!("Attempts: {}", result.attempts);
println!("Total delay: {}ms", result.total_delay_ms);
```

### Error Handling
```rust
let result = retry_with_backoff(config, |_| {
    match operation() {
        Ok(v) => Ok(v),
        Err(Error::TransportTimeout) => {
            log("Timeout, retrying...");
            Err(Error::TransportTimeout)
        }
        Err(e) => Err(e),
    }
});
```

## Testing

```bash
# All retry tests
cargo test retry --lib

# Network failure tests
cargo test test_network_failure --lib

# Rate limit tests  
cargo test test_rate_limit --lib
```

## Documentation

- Full docs: `RETRY_BACKOFF.md`
- Implementation: `RETRY_IMPLEMENTATION_SUMMARY.md`
- Code: `src/retry.rs`
- Tests: `src/retry_tests.rs`
