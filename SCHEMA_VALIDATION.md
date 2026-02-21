# Schema Validation for AnchorKit

## Overview

AnchorKit now includes **strict schema validation** to prevent misconfiguration bugs before runtime. This validation happens at two levels:

1. **Pre-deployment validation** - Python script validates config files
2. **Runtime validation** - Smart contract validates all configuration inputs

## Why Schema Validation?

Misconfiguration bugs can cause:
- Contract initialization failures
- Invalid attestor registrations
- Security vulnerabilities
- Wasted gas on failed transactions

Schema validation catches these issues **before deployment**.

## Pre-Deployment Validation

### Running the Validator

```bash
python3 validate_config.py
```

This validates all JSON configuration files in the `configs/` directory.

### What Gets Validated

#### Contract Configuration
- **name**: 1-64 characters
- **version**: 1-16 characters  
- **network**: 1-32 characters

#### Attestor Configuration
- **name**: 1-64 characters
- **address**: 54-56 characters (Stellar address format)
- **address prefix**: Must start with 'G'
- **endpoint**: 8-256 characters, must start with http:// or https://
- **role**: 1-32 characters
- **registry size**: 1-100 attestors maximum

#### Session Configuration
- **session_timeout_seconds**: 1-86400 (1 second to 24 hours)
- **operations_per_session**: 1-10000

### Example Output

```
Validating fiat-on-off-ramp.json...
✓ fiat-on-off-ramp.json is valid

✅ All 3 configuration files are valid
```

### Validation Errors

```
❌ Validation failed:

  • fiat-on-off-ramp.json: Attestor 0 (kyc-provider): address must be 54-56 chars, got 52
  • remittance-anchor.json: Attestor 2 (corridor-operator): endpoint must be 8-256 chars
```

## Runtime Validation

### Contract Methods with Validation

#### 1. Initialize with Config

```rust
pub fn initialize_with_config(
    env: Env,
    admin: Address,
    config: ContractConfig,
) -> Result<(), Error>
```

Validates configuration before initializing the contract.

**Example:**
```rust
let config = ContractConfig {
    name: String::from_str(&env, "my-anchor"),
    version: String::from_str(&env, "1.0.0"),
    network: String::from_str(&env, "testnet"),
};

contract.initialize_with_config(env, admin, config)?;
```

#### 2. Batch Register Attestors

```rust
pub fn batch_register_attestors(
    env: Env,
    attestors: Vec<AttestorConfig>,
) -> Result<(), Error>
```

Validates all attestors before registration.

**Example:**
```rust
let mut attestors = Vec::new(&env);
attestors.push_back(AttestorConfig {
    name: String::from_str(&env, "kyc-provider"),
    address: String::from_str(&env, "GBBD6A7KNZF5WNWQEPZP5DYJD2AYUTLXRB6VXJ4RCX4RTNPPQVNF3GQ"),
    endpoint: String::from_str(&env, "https://kyc.example.com/verify"),
    role: String::from_str(&env, "kyc-issuer"),
    enabled: true,
});

contract.batch_register_attestors(env, attestors)?;
```

#### 3. Configure Session Settings

```rust
pub fn configure_session_settings(
    env: Env,
    config: SessionConfig,
) -> Result<(), Error>
```

Validates session configuration.

**Example:**
```rust
let config = SessionConfig {
    enable_tracking: true,
    timeout_seconds: 3600,
    max_operations: 1000,
};

contract.configure_session_settings(env, config)?;
```

## Error Codes

| Error | Code | Description |
|-------|------|-------------|
| `InvalidConfig` | 25 | Configuration validation failed |
| `InvalidEndpointFormat` | 10 | Endpoint URL format invalid |
| `AttestorAlreadyRegistered` | 4 | Attestor already exists |

## Validation Rules Reference

### ContractConfig

```rust
pub struct ContractConfig {
    pub name: String,      // 1-64 chars
    pub version: String,   // 1-16 chars
    pub network: String,   // 1-32 chars
}
```

### AttestorConfig

```rust
pub struct AttestorConfig {
    pub name: String,      // 1-64 chars
    pub address: String,   // 54-56 chars (Stellar format)
    pub endpoint: String,  // 8-256 chars (valid URL)
    pub role: String,      // 1-32 chars
    pub enabled: bool,
}
```

### SessionConfig

```rust
pub struct SessionConfig {
    pub enable_tracking: bool,
    pub timeout_seconds: u64,   // 1-86400
    pub max_operations: u64,    // 1-10000
}
```

## Testing

Run the validation tests:

```bash
cargo test config_tests
```

Tests cover:
- Valid configurations
- Empty/missing fields
- Oversized fields
- Invalid formats
- Boundary conditions

## Integration into CI/CD

Add to your deployment pipeline:

```yaml
# .github/workflows/deploy.yml
- name: Validate Configuration
  run: python3 validate_config.py
  
- name: Run Tests
  run: cargo test
  
- name: Build Contract
  run: cargo build --release --target wasm32-unknown-unknown
```

## Best Practices

1. **Always validate before deployment** - Run `validate_config.py`
2. **Use typed configs** - Use `ContractConfig`, `AttestorConfig`, `SessionConfig` types
3. **Handle validation errors** - Check `Result<(), Error>` return values
4. **Test edge cases** - Validate boundary conditions in tests
5. **Document constraints** - Keep this guide updated with validation rules

## Migration Guide

### Before (No Validation)

```rust
// Direct initialization - no validation
contract.initialize(env, admin)?;

// Manual attestor registration - error-prone
contract.register_attestor(env, attestor1)?;
contract.register_attestor(env, attestor2)?;
```

### After (With Validation)

```rust
// Validated initialization
let config = ContractConfig { /* ... */ };
config.validate()?;
contract.initialize_with_config(env, admin, config)?;

// Batch registration with validation
let attestors = vec![attestor1_config, attestor2_config];
validate_attestors(&attestors)?;
contract.batch_register_attestors(env, attestors)?;
```

## Troubleshooting

### "InvalidConfig" Error

Check that all fields meet length requirements:
- Contract name: 1-64 chars
- Version: 1-16 chars
- Network: 1-32 chars

### "InvalidEndpointFormat" Error

Ensure endpoints:
- Are 8-256 characters
- Start with `http://` or `https://`
- Are valid URLs

### "AttestorAlreadyRegistered" Error

The attestor address is already registered. Use `revoke_attestor` first if you need to re-register.

## Support

For issues or questions:
1. Check validation error messages
2. Review this documentation
3. Run tests: `cargo test config_tests`
4. Check example configs in `configs/` directory
