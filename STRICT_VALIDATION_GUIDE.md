# Strict Schema Validation Guide

## Overview

AnchorKit implements **multi-layer strict schema validation** to prevent misconfiguration bugs before runtime. This ensures contract reliability and prevents costly deployment failures.

## Validation Layers

### 1. Compile-Time Validation (build.rs)

Configuration files are validated during the build process. If any config is invalid, the build fails immediately.

```bash
cargo build
```

**What happens:**
- `build.rs` runs Python validator on all config files
- Invalid configs cause build failure with detailed error messages
- Prevents deployment of misconfigured contracts

### 2. Pre-Deployment Validation (Shell Script)

Run before deploying to any network:

```bash
./pre_deploy_validate.sh
```

**Validates:**
- All TOML and JSON config files
- Schema compliance
- Business rule violations
- Security best practices

### 3. Runtime Validation (Smart Contract)

All configuration inputs are validated at runtime with granular error codes.

## Validation Rules

### ContractConfig

```rust
pub struct ContractConfig {
    pub name: String,      // 1-64 chars, lowercase/digits/hyphens, must start with letter
    pub version: String,   // Semantic versioning (e.g., 1.0.0), no leading zeros
    pub network: String,   // Must be: stellar-testnet, stellar-mainnet, or stellar-futurenet
}
```

**Additional Rules:**
- Name cannot contain reserved keywords: `admin`, `system`, `root`
- Version cannot be `0.0.0`
- Must start with lowercase letter

**Error Codes:**
- `InvalidConfigName` (28): Name validation failed
- `InvalidConfigVersion` (29): Version validation failed
- `InvalidConfigNetwork` (30): Network validation failed

### AttestorConfig

```rust
pub struct AttestorConfig {
    pub name: String,      // 1-64 chars, lowercase/digits/hyphens
    pub address: String,   // 54-56 chars, Stellar format (starts with 'G')
    pub endpoint: String,  // 8-256 chars, valid HTTP/HTTPS URL
    pub role: String,      // Valid role enum
    pub enabled: bool,
}
```

**Valid Roles:**
- `kyc-issuer`
- `transfer-verifier`
- `compliance-approver`
- `rate-provider`
- `attestor`

**Batch Validation Rules:**
- Minimum 1 attestor, maximum 100
- No duplicate names
- No duplicate addresses
- At least one enabled attestor required

**Error Codes:**
- `InvalidAttestorName` (31): Name validation failed
- `InvalidAttestorAddress` (32): Address validation failed
- `InvalidAttestorRole` (33): Role validation failed
- `DuplicateAttestor` (26): Duplicate name or address
- `NoEnabledAttestors` (27): No enabled attestors in batch

### SessionConfig

```rust
pub struct SessionConfig {
    pub enable_tracking: bool,
    pub timeout_seconds: u64,   // 60-86400 (1 min to 24 hours)
    pub max_operations: u64,    // 1-10000
}
```

**Business Rules:**
- If tracking enabled, minimum timeout is 300 seconds (5 minutes)
- Maximum operations per session: 5000 (performance limit)

**Error Code:**
- `InvalidConfig` (25): General config validation failed

## Usage Examples

### 1. Initialize with Validated Config

```rust
use anchorkit::{ContractConfig, validate_init_config};

let config = ContractConfig {
    name: String::from_str(&env, "my-anchor"),
    version: String::from_str(&env, "1.0.0"),
    network: String::from_str(&env, "stellar-testnet"),
};

// Validate before initialization
validate_init_config(&config)?;

// Initialize contract
contract.initialize_with_config(env, admin, config)?;
```

### 2. Batch Register Attestors

```rust
use anchorkit::{AttestorConfig, validate_attestor_batch};

let mut attestors = Vec::new(&env);

attestors.push_back(AttestorConfig {
    name: String::from_str(&env, "kyc-provider"),
    address: String::from_str(&env, "GBBD6A7KNZF5WNWQEPZP5DYJD2AYUTLXRB6VXJ4RCX4RTNPPQVNF3GQ"),
    endpoint: String::from_str(&env, "https://kyc.example.com/verify"),
    role: String::from_str(&env, "kyc-issuer"),
    enabled: true,
});

// Validate batch before registration
validate_attestor_batch(&attestors)?;

// Register all attestors
contract.batch_register_attestors(env, attestors)?;
```

### 3. Configure Session Settings

```rust
use anchorkit::{SessionConfig, validate_session_config};

let config = SessionConfig {
    enable_tracking: true,
    timeout_seconds: 3600,  // 1 hour
    max_operations: 1000,
};

// Validate before configuration
validate_session_config(&config)?;

// Apply configuration
contract.configure_session_settings(env, config)?;
```

## Error Handling

### Granular Error Codes

```rust
match contract.initialize_with_config(env, admin, config) {
    Ok(_) => println!("Initialized successfully"),
    Err(Error::InvalidConfigName) => println!("Invalid contract name format"),
    Err(Error::InvalidConfigVersion) => println!("Invalid version format"),
    Err(Error::InvalidConfigNetwork) => println!("Invalid network specified"),
    Err(Error::AlreadyInitialized) => println!("Contract already initialized"),
    Err(e) => println!("Unexpected error: {:?}", e),
}
```

### Batch Validation Errors

```rust
match contract.batch_register_attestors(env, attestors) {
    Ok(_) => println!("All attestors registered"),
    Err(Error::InvalidAttestorName) => println!("Invalid attestor name"),
    Err(Error::InvalidAttestorAddress) => println!("Invalid Stellar address"),
    Err(Error::InvalidAttestorRole) => println!("Invalid role specified"),
    Err(Error::DuplicateAttestor) => println!("Duplicate attestor detected"),
    Err(Error::NoEnabledAttestors) => println!("At least one attestor must be enabled"),
    Err(e) => println!("Unexpected error: {:?}", e),
}
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Deploy Contract

on:
  push:
    branches: [main]

jobs:
  validate-and-deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Python Dependencies
        run: pip install jsonschema toml
      
      - name: Validate Configurations
        run: ./pre_deploy_validate.sh
      
      - name: Build Contract
        run: cargo build --release --target wasm32-unknown-unknown
      
      - name: Run Tests
        run: cargo test
      
      - name: Deploy
        run: ./deploy.sh
```

## Testing

### Run Validation Tests

```bash
cargo test validation
```

### Test Coverage

- Valid configurations
- Invalid name formats
- Invalid version formats
- Invalid network values
- Duplicate attestors
- Empty attestor lists
- Invalid Stellar addresses
- Invalid endpoint URLs
- Invalid roles
- Session timeout boundaries
- Operations per session limits

## Best Practices

1. **Always validate before deployment**
   ```bash
   ./pre_deploy_validate.sh && cargo build --release
   ```

2. **Use typed configs in code**
   - Never construct configs with raw strings
   - Use the provided structs and validation functions

3. **Handle all error cases**
   - Match on specific error codes
   - Provide meaningful error messages to users

4. **Test edge cases**
   - Boundary values (min/max lengths)
   - Invalid formats
   - Duplicate entries

5. **Keep configs in version control**
   - Track all configuration changes
   - Review config changes in PRs

6. **Document custom validation rules**
   - If you add business rules, document them
   - Update error codes list

## Troubleshooting

### Build Fails with Config Error

```
error: Configuration validation failed for fiat-on-off-ramp.json:
  • Attestor 'kyc-provider': address must be 54-56 chars, got 52
```

**Solution:** Fix the configuration file and rebuild.

### Runtime InvalidConfigName Error

**Causes:**
- Name contains uppercase letters
- Name starts with digit or hyphen
- Name contains reserved keywords (admin, system, root)

**Solution:** Use lowercase letters, digits, and hyphens only. Start with a letter.

### DuplicateAttestor Error

**Causes:**
- Two attestors have the same name
- Two attestors have the same address

**Solution:** Ensure all attestor names and addresses are unique.

### NoEnabledAttestors Error

**Cause:** All attestors in the batch have `enabled: false`

**Solution:** Enable at least one attestor.

## Migration from Unvalidated Code

### Before (No Validation)

```rust
// Direct initialization - risky
contract.initialize(env, admin)?;

// Manual registration - error-prone
contract.register_attestor(env, attestor1)?;
contract.register_attestor(env, attestor2)?;
```

### After (With Strict Validation)

```rust
// Validated initialization
let config = ContractConfig { /* ... */ };
validate_init_config(&config)?;
contract.initialize_with_config(env, admin, config)?;

// Batch registration with validation
let attestors = vec![attestor1_config, attestor2_config];
validate_attestor_batch(&attestors)?;
contract.batch_register_attestors(env, attestors)?;
```

## Performance Impact

Validation adds minimal overhead:
- Compile-time: ~1-2 seconds for config validation
- Runtime: <0.1ms per validation call
- Gas cost: Negligible (validation is efficient)

## Security Benefits

1. **Prevents misconfiguration attacks**
   - Invalid addresses rejected
   - Malformed URLs blocked
   - Reserved keywords prevented

2. **Ensures data integrity**
   - No duplicate attestors
   - Consistent format enforcement
   - Type safety at all layers

3. **Audit trail**
   - All validation failures logged
   - Clear error messages for debugging
   - Traceable configuration changes

## Support

For issues or questions:
1. Check error codes in `src/errors.rs`
2. Review validation rules in `src/validation.rs`
3. Run tests: `cargo test validation`
4. Check example configs in `configs/` directory
