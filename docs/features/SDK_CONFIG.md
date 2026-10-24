# SDK Configuration

## Overview

The SDK Configuration module provides a type-safe way to configure AnchorKit SDK client connections with network settings, anchor domains, timeouts, and custom HTTP headers.

## Table of Contents

- [Data Structures](#data-structures)
- [Feature Flags](#feature-flags)
- [Validation Rules](#validation-rules)
- [Usage Example](#usage-example)
- [Configuration Form](#configuration-form)
- [Security Considerations](#security-considerations)
- [Best Practices](#best-practices)
- [Integration with AnchorKit](#integration-with-anchorkit)
- [Testing](#testing)
- [Error Handling](#error-handling)
- [Future Enhancements](#future-enhancements)
- [Related Documentation](#related-documentation)

## Data Structures

### SdkConfig

Main configuration structure for SDK clients.

```rust
pub struct SdkConfig {
    pub network: Network,
    pub timeout_seconds: u32,
    pub retry_attempts: u32,
    pub default_anchor: String,
}
```

### Network

Enum for Stellar network selection.

```rust
pub enum Network {
    Testnet,
    Mainnet,
}
```

## Feature Flags

### mock-only

The `mock-only` feature flag is defined in `Cargo.toml` but is **currently not implemented** in the codebase.

#### Current Status

⚠️ **Important**: This feature flag exists as a placeholder but does not currently affect code behavior. No conditional compilation directives (`#[cfg(feature = "mock-only")]`) are present in the source code.

#### Intended Purpose

When implemented, the `mock-only` feature would be designed for:
- **Unit testing**: Run tests without external dependencies
- **Development**: Develop and debug without live anchor services  
- **CI/CD pipelines**: Ensure tests run reliably in isolated environments
- **Integration testing**: Test application logic with predictable responses

#### Current Usage

Currently, you can build with the feature flag, but it has no effect:

```bash
# This builds successfully but mock-only has no effect
cargo build --no-default-features --features mock-only
cargo test --no-default-features --features mock-only
```

#### Implementation Status

To implement this feature, the codebase would need:

1. **Conditional compilation directives** in relevant modules:
```rust
#[cfg(feature = "mock-only")]
fn fetch_anchor_info() -> MockResponse {
    // Mock implementation
}

#[cfg(not(feature = "mock-only"))]
fn fetch_anchor_info() -> RealResponse {
    // Real implementation  
}
```

2. **Mock implementations** for:
   - Network requests and HTTP calls
   - SEP-10 authentication flows
   - Transaction operations (deposits/withdrawals)
   - Anchor discovery and info fetching
   - Rate limiting and timing

#### Mock Testing (Current Approach)

Currently, mock testing is achieved through:

1. **Soroban SDK test utilities**:
```rust
#[cfg(test)]
mod tests {
    use soroban_sdk::Env;
    
    #[test]
    fn test_function() {
        let env = Env::default();
        env.mock_all_auths(); // Mock authentication
        // Test logic here
    }
}
```

2. **Mock server for HTTP testing**:
```bash
# Start the mock anchor server
python3 mock-server.py

# Test against mock server
export ANCHOR_URL=http://localhost:8080
cargo test
```

#### Future Implementation

To implement the `mock-only` feature:

1. Add conditional compilation to network-related functions
2. Create mock implementations that return predictable responses
3. Ensure mock responses match real API schemas
4. Add feature-specific tests to verify mock behavior

#### CI/CD Integration

The feature flag is tested in CI pipelines to ensure it compiles:

```yaml
# .github/workflows/feature-flag-matrix.yml
- name: Build (mock-only)
  run: cargo build --no-default-features --features mock-only
```

However, since the feature isn't implemented, this only verifies compilation compatibility.

## Validation Rules

The `SdkConfig::validate()` method enforces the following constraints and returns `Result<(), Error>`:

### Default Anchor
- **Minimum length**: 3 characters
- **Maximum length**: 256 characters
- **Field name**: `default_anchor`

### Timeout
- **Minimum**: 5 seconds
- **Maximum**: 300 seconds (5 minutes)
- **Default**: 10 seconds (`DEFAULT_TIMEOUT_SECONDS`)

### Retry Attempts
- **Minimum**: 0 (no retries)
- **Maximum**: 10
- **Default**: 3 (set by `SdkConfig::with_defaults`)

## Usage Example

### Rust (Contract)

```rust
use soroban_sdk::{Env, String};

let env = Env::default();

// Create config using the constructor (validates on creation)
let config = SdkConfig::new(
    Network::Testnet,
    30,                                              // timeout_seconds (5–300)
    3,                                               // retry_attempts (0–10)
    String::from_str(&env, "anchor.example.com"),   // default_anchor (3–256 chars)
).expect("valid config");

// Or use with_defaults for timeout=10s, retry_attempts=3
let config = SdkConfig::with_defaults(
    Network::Mainnet,
    String::from_str(&env, "anchor.example.com"),
).expect("valid config");

// Validate an existing config explicitly
match config.validate() {
    Ok(()) => { /* use config */ }
    Err(e) => { /* handle Error::invalid_config() */ }
}
```

### JavaScript (Client SDK)

```javascript
const config = {
    network: 'Testnet',
    default_anchor: 'anchor.example.com',
    timeout_seconds: 30,
    retry_attempts: 3,
};
```

## Configuration Form

An HTML form is provided in `sdk_config_form.html` for easy configuration generation. The form includes:

- Network selection (Testnet/Mainnet)
- Anchor domain input with validation
- Timeout configuration
- Dynamic custom header management
- JSON output generation

### Using the Form

1. Open `sdk_config_form.html` in a web browser
2. Select your network (Testnet or Mainnet)
3. Enter the anchor domain
4. Set the timeout (default: 30 seconds)
5. Add custom headers as needed
6. Click "Generate Configuration" to get JSON output

## Security Considerations

### Header Security
- Never include sensitive credentials directly in headers
- Use secure credential management (see `credential_management.sh` in examples)
- Rotate tokens regularly
- Use HTTPS for all anchor communications

### Domain Validation
- Validate anchor domains against a whitelist
- Use DNS verification for production
- Implement certificate pinning for critical operations

### Timeout Configuration
- Set appropriate timeouts based on network conditions
- Consider retry logic for transient failures
- Monitor timeout rates for performance tuning

## Best Practices

### Network Selection
- Use **Testnet** for development and testing
- Use **Mainnet** only for production deployments
- Never mix testnet and mainnet configurations

### Timeout Settings
- **Development**: 60-120 seconds (for debugging)
- **Production**: 30 seconds (recommended)
- **High-latency networks**: 60-90 seconds
- **Low-latency networks**: 15-30 seconds

### Custom Headers
- Use headers for:
  - Authentication tokens
  - API versioning
  - Request tracing
  - Custom metadata
- Avoid headers for:
  - Large payloads (use request body)
  - Sensitive data without encryption
  - Unnecessary metadata

## Integration with AnchorKit

The SDK configuration integrates with:

- **Session Management**: Timeout settings affect session duration
- **Credential Management**: Headers can include auth tokens
- **Health Monitoring**: Timeout affects health check intervals
- **Rate Comparison**: Network selection determines available anchors
- **Mock Testing**: Currently achieved through Soroban SDK test utilities and mock server

### Mock Testing Integration

Currently, mock testing is handled through existing mechanisms:

```rust
// Using Soroban SDK test utilities (current approach)
let env = Env::default();
env.mock_all_auths(); // Mock authentication for tests

let config = SdkConfig::new(
    Network::Testnet,
    30,
    3,
    String::from_str(&env, "test.anchor.com"),
).expect("valid config");

// Tests use real implementations but with mocked Soroban environment
```

**Note**: The `mock-only` feature flag is defined but not yet implemented. When implemented, it would provide compile-time mock behavior.

## Testing

Run the SDK configuration tests:

```bash
# Run all tests with default features
cargo test sdk_config_tests --lib

# Build with mock-only feature (currently no behavioral difference)
cargo build --no-default-features --features mock-only

# Run specific configuration tests
cargo test sdk_config --lib
```

### Test Coverage

Test coverage includes:
- Valid configuration validation
- Domain length constraints
- Timeout boundary conditions
- Header count limits
- Header size constraints
- Network type enum values
- Feature flag compilation compatibility

### Mock Testing (Current Implementation)

Currently, mock testing uses Soroban SDK utilities:

```bash
# Standard test approach with Soroban mocking
cargo test

# Tests use env.mock_all_auths() for authentication mocking
# Use mock-server.py for HTTP endpoint testing
python3 mock-server.py &
cargo test -- --test-threads=1
```

**Note**: The `mock-only` feature flag compiles but doesn't change test behavior yet.

## Error Handling

`validate()` returns `Result<(), Error>`, not a boolean. Use `match` or the `?` operator:

```rust
// Pattern 1: match
match config.validate() {
    Ok(()) => {
        // config is valid, proceed
    }
    Err(_) => {
        // Check which constraint was violated
        if config.default_anchor.len() < 3 || config.default_anchor.len() > 256 {
            // default_anchor length out of range
        }
        if config.timeout_seconds < 5 || config.timeout_seconds > 300 {
            // timeout_seconds out of range (must be 5–300)
        }
        if config.retry_attempts > 10 {
            // retry_attempts exceeds maximum of 10
        }
    }
}

// Pattern 2: propagate with ?
fn setup_sdk(env: &Env) -> Result<SdkConfig, Error> {
    let config = SdkConfig::new(
        Network::Testnet,
        30,
        3,
        String::from_str(env, "anchor.example.com"),
    )?;
    Ok(config)
}
```

## Future Enhancements

Potential improvements:
- Support for connection pooling settings
- Circuit breaker configuration
- Rate limiting settings
- Custom DNS resolver configuration
- Proxy support
- Custom HTTP headers (`custom_headers: Vec<HttpHeader>`)

## Related Documentation

- [Credential Management](../../examples/credential_management.sh) - Credential management
- [STATUS_MONITOR.md](./STATUS_MONITOR.md) - Status monitoring and health checks
- [API_REFERENCE.md](../API_REFERENCE.md) - API specifications
- [QUICK_START.md](../../QUICK_START.md) - Getting started guide
- [Mock Mode Example](../../examples/mock_mode_example.sh) - Mock testing script
