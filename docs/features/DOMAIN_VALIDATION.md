# Anchor Domain Validation Utility

A robust utility function for validating anchor domain URLs before making requests in the AnchorKit SDK.

## Features

- ✅ Validates URL format
- ✅ Enforces HTTPS-only connections
- ✅ Rejects malformed domains
- ✅ Comprehensive unit tests included
- ✅ `no_std` compatible for embedded environments

## Usage

```rust
use anchorkit::{validate_anchor_domain, Error};

fn make_anchor_request(domain: &str) -> Result<(), Error> {
    // Validate domain before making request
    validate_anchor_domain(domain)?;
    
    // Proceed with request...
    Ok(())
}
```

## Validation Rules

### Required
- Must use HTTPS protocol (HTTP is rejected)
- Must have valid domain structure with at least one dot (TLD required)
- Must not be empty or whitespace-only
- Minimum length: 10 characters (`https://a.b`)
- Maximum length: 2048 characters

### Domain Structure
- No leading or trailing dots
- No consecutive dots (`..`)
- Labels must start and end with alphanumeric characters
- Labels can contain hyphens in the middle
- Must contain at least one dot (TLD required)

### Port Validation (Optional)
- Port must be numeric
- Valid range: 1-65535
- Port 0 is rejected

### Character Restrictions
- No control characters (newlines, tabs, null bytes, etc.)
- No spaces in domain portion

## Examples

### Valid Domains

```rust
// Basic domains
validate_anchor_domain("https://example.com").unwrap();
validate_anchor_domain("https://api.example.com").unwrap();

// With subdomains
validate_anchor_domain("https://api.v2.anchor.example.com").unwrap();

// With ports
validate_anchor_domain("https://example.com:8080").unwrap();
validate_anchor_domain("https://example.com:443").unwrap();

// With paths
validate_anchor_domain("https://example.com/api/v1").unwrap();
validate_anchor_domain("https://example.com/sep24/info").unwrap();

// With query parameters
validate_anchor_domain("https://example.com?asset=USDC").unwrap();
validate_anchor_domain("https://example.com/api?version=1").unwrap();

// With hyphens
validate_anchor_domain("https://my-anchor.com").unwrap();
validate_anchor_domain("https://api-v2.example.com").unwrap();
```

### Invalid Domains

```rust
// Not HTTPS
assert!(validate_anchor_domain("http://example.com").is_err());
assert!(validate_anchor_domain("ftp://example.com").is_err());

// Missing protocol
assert!(validate_anchor_domain("example.com").is_err());

// Malformed structure
assert!(validate_anchor_domain("https://").is_err());
assert!(validate_anchor_domain("https://.example.com").is_err());
assert!(validate_anchor_domain("https://example..com").is_err());
assert!(validate_anchor_domain("https://example.com.").is_err());

// No TLD
assert!(validate_anchor_domain("https://localhost").is_err());
assert!(validate_anchor_domain("https://example").is_err());

// Invalid ports
assert!(validate_anchor_domain("https://example.com:0").is_err());
assert!(validate_anchor_domain("https://example.com:99999").is_err());
assert!(validate_anchor_domain("https://example.com:abc").is_err());

// Control characters
assert!(validate_anchor_domain("https://example.com\n").is_err());
assert!(validate_anchor_domain("https://example.com\t").is_err());

// Spaces
assert!(validate_anchor_domain("https://exam ple.com").is_err());
```

## Error Handling

The function returns `Result<(), Error>`:
- `Ok(())` - Domain is valid
- `Err(Error::InvalidEndpointFormat)` - Domain validation failed

```rust
match validate_anchor_domain(user_input) {
    Ok(()) => {
        // Safe to proceed with request
        make_request(user_input)
    }
    Err(Error::InvalidEndpointFormat) => {
        // Handle invalid domain
        eprintln!("Invalid anchor domain: {}", user_input);
        Err(Error::InvalidEndpointFormat)
    }
    Err(e) => {
        // Handle other errors
        Err(e)
    }
}
```

## Testing

Run the comprehensive test suite:

```bash
cargo test domain_validator --lib
```

Run the example:

```bash
cargo run --example domain_validation_example
```

## Test Coverage

The utility includes 8 comprehensive test suites covering:

1. **Valid domains** - Various valid URL formats
2. **HTTPS enforcement** - Rejection of non-HTTPS protocols
3. **Malformed domains** - Detection of structural issues
4. **Port validation** - Valid and invalid port numbers
5. **Length limits** - Minimum and maximum URL lengths
6. **Control characters** - Rejection of special characters
7. **Double slashes** - Handling of path separators
8. **Edge cases** - Boundary conditions and special scenarios

## Integration

The domain validator is integrated into the AnchorKit SDK and can be used before any anchor API calls:

```rust
use anchorkit::{validate_anchor_domain, Error};

pub fn fetch_anchor_info(domain: &str) -> Result<AnchorInfo, Error> {
    // Validate domain first
    validate_anchor_domain(domain)?;
    
    // Make the actual request
    let url = format!("{}/sep24/info", domain);
    // ... rest of implementation
}
```

## Security Considerations

- **HTTPS-only**: Prevents man-in-the-middle attacks by rejecting unencrypted connections
- **Input validation**: Protects against injection attacks and malformed URLs
- **Length limits**: Prevents buffer overflow and DoS attacks
- **Character filtering**: Blocks control characters that could cause parsing issues

## Performance

The validator is designed for efficiency:
- No heap allocations in the validation logic
- Early returns on obvious failures
- Minimal string operations
- `no_std` compatible for resource-constrained environments

## Future Enhancements

Potential improvements for future versions:
- IDN (Internationalized Domain Names) support
- IP address validation (IPv4/IPv6)
- Custom port range restrictions
- Configurable validation rules
- Domain whitelist/blacklist support
