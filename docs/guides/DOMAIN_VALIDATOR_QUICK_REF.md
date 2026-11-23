# Domain Validator Quick Reference

## Function Signature
```rust
pub fn validate_anchor_domain(domain: &str) -> Result<(), Error>
```

## Import
```rust
use anchorkit::{validate_anchor_domain, Error};
```

## Basic Usage
```rust
validate_anchor_domain("https://anchor.example.com")?;
```

## Validation Rules

| Rule | Requirement |
|------|-------------|
| Protocol | HTTPS only |
| Min Length | 10 chars |
| Max Length | 2048 chars |
| Domain | Must have TLD (dot required) |
| Port | 1-65535 (optional) |
| Characters | No control chars, no spaces in domain |

## Quick Examples

### ✅ Valid
```rust
"https://example.com"
"https://api.anchor.com:8080"
"https://anchor.com/api/v1"
"https://anchor.com?asset=USDC"
"https://my-anchor.example.com"
```

### ❌ Invalid
```rust
"http://example.com"        // Not HTTPS
"example.com"               // Missing protocol
"https://"                  // No domain
"https://example"           // No TLD
"https://example.com:0"     // Invalid port
"https://example .com"      // Space in domain
```

## Error Handling
```rust
match validate_anchor_domain(url) {
    Ok(()) => /* proceed */,
    Err(Error::InvalidEndpointFormat) => /* handle error */,
}
```

## Testing
```bash
cargo test domain_validator --lib
cargo run --example domain_validation_example
```

## Files
- Implementation: `src/domain_validator.rs`
- Errors: `src/errors.rs`
- Example: `examples/domain_validation_example.rs`
- Docs: `DOMAIN_VALIDATION.md`
