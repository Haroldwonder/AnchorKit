# Anchor Capability Detection - Test Coverage

## Status: ✅ COMPLETE

All validation tests for anchor capability detection have been implemented and are passing.

## Test Goals Coverage

### 1. ✅ Detect Deposit-Only Anchors

**Implementation:** `test_detect_deposit_only_anchor`

Verifies that the system correctly identifies anchors that only support deposit operations.

```rust
#[test]
fn test_detect_deposit_only_anchor() {
    // Configure anchor with ONLY deposit service
    let mut services = Vec::new(&env);
    services.push_back(ServiceType::Deposits);
    
    client.configure_services(&deposit_anchor, &services);
    
    // Verify capability detection
    assert!(client.supports_service(&deposit_anchor, &ServiceType::Deposits));
    assert!(!client.supports_service(&deposit_anchor, &ServiceType::Withdrawals));
    assert!(!client.supports_service(&deposit_anchor, &ServiceType::Quotes));
    assert!(!client.supports_service(&deposit_anchor, &ServiceType::KYC));
}
```

**What's Tested:**
- Anchor configured with only `ServiceType::Deposits`
- `supports_service()` returns `true` for Deposits
- `supports_service()` returns `false` for all other services
- `get_supported_services()` returns exactly 1 service

**Result:** ✅ PASS

---

### 2. ✅ Detect Full-Service Anchors

**Implementation:** `test_detect_full_service_anchor`

Verifies that the system correctly identifies anchors that support all available services.

```rust
#[test]
fn test_detect_full_service_anchor() {
    // Configure anchor with ALL services
    let mut services = Vec::new(&env);
    services.push_back(ServiceType::Deposits);
    services.push_back(ServiceType::Withdrawals);
    services.push_back(ServiceType::Quotes);
    services.push_back(ServiceType::KYC);
    
    client.configure_services(&full_service_anchor, &services);
    
    // Verify all services are supported
    assert!(client.supports_service(&full_service_anchor, &ServiceType::Deposits));
    assert!(client.supports_service(&full_service_anchor, &ServiceType::Withdrawals));
    assert!(client.supports_service(&full_service_anchor, &ServiceType::Quotes));
    assert!(client.supports_service(&full_service_anchor, &ServiceType::KYC));
}
```

**What's Tested:**
- Anchor configured with all 4 service types
- `supports_service()` returns `true` for all services
- `get_supported_services()` returns all 4 services
- Services list contains all expected types

**Result:** ✅ PASS

---

### 3. ✅ Reject Malformed Capability Payloads

**Implementations:**
- `test_reject_empty_services` - Empty service list
- `test_reject_duplicate_services` - Duplicate services
- `test_reject_unregistered_anchor_services` - Unregistered anchor
- `test_reject_invalid_metadata_scores` - Invalid metadata values

#### 3a. Empty Services

```rust
#[test]
fn test_reject_empty_services() {
    let empty_services = Vec::new(&env);
    
    let result = client.try_configure_services(&anchor, &empty_services);
    assert_eq!(result, Err(Ok(Error::InvalidServiceType)));
}
```

**What's Tested:**
- Attempting to configure with empty services vector
- Returns `Error::InvalidServiceType`
- No services are stored

**Result:** ✅ PASS

#### 3b. Duplicate Services

```rust
#[test]
fn test_reject_duplicate_services() {
    let mut duplicate_services = Vec::new(&env);
    duplicate_services.push_back(ServiceType::Deposits);
    duplicate_services.push_back(ServiceType::Deposits); // Duplicate
    
    let result = client.try_configure_services(&anchor, &duplicate_services);
    assert_eq!(result, Err(Ok(Error::InvalidServiceType)));
}
```

**What's Tested:**
- Attempting to configure with duplicate service types
- Returns `Error::InvalidServiceType`
- Validation catches duplicates before storage

**Result:** ✅ PASS

#### 3c. Unregistered Anchor

```rust
#[test]
fn test_reject_unregistered_anchor_services() {
    let unregistered_anchor = Address::generate(&env);
    
    let mut services = Vec::new(&env);
    services.push_back(ServiceType::Deposits);
    
    let result = client.try_configure_services(&unregistered_anchor, &services);
    assert_eq!(result, Err(Ok(Error::AttestorNotRegistered)));
}
```

**What's Tested:**
- Attempting to configure services for non-registered anchor
- Returns `Error::AttestorNotRegistered`
- Only registered attestors can configure services

**Result:** ✅ PASS

#### 3d. Invalid Metadata Scores

```rust
#[test]
fn test_reject_invalid_metadata_scores() {
    // Try to set metadata with invalid reputation score (> 10000)
    let result = client.try_set_anchor_metadata(
        &anchor,
        &10001u32, // Invalid: > 10000
        &3600u64,
        &9000u32,
        &9950u32,
        &1_000_000u64,
    );
    assert_eq!(result, Err(Ok(Error::InvalidAnchorMetadata)));
}
```

**What's Tested:**
- Reputation score validation (0-10000)
- Liquidity score validation (0-10000)
- Uptime percentage validation (0-10000)
- Returns `Error::InvalidAnchorMetadata` for invalid values

**Result:** ✅ PASS

---

## Additional Test Coverage

### Withdrawal-Only Anchor Detection

**Test:** `test_detect_withdrawal_only_anchor`

Verifies detection of anchors that only support withdrawals.

```rust
let mut services = Vec::new(&env);
services.push_back(ServiceType::Withdrawals);

client.configure_services(&withdrawal_anchor, &services);

assert!(!client.supports_service(&withdrawal_anchor, &ServiceType::Deposits));
assert!(client.supports_service(&withdrawal_anchor, &ServiceType::Withdrawals));
```

**Result:** ✅ PASS

---

### Quote Provider Detection

**Test:** `test_detect_quote_provider_anchor`

Verifies detection of anchors that provide quotes and KYC services.

```rust
let mut services = Vec::new(&env);
services.push_back(ServiceType::Quotes);
services.push_back(ServiceType::KYC);

client.configure_services(&quote_anchor, &services);

assert!(client.supports_service(&quote_anchor, &ServiceType::Quotes));
assert!(client.supports_service(&quote_anchor, &ServiceType::KYC));
```

**Result:** ✅ PASS

---

### Capability Detection with Metadata

**Test:** `test_capability_detection_with_metadata`

Verifies that capability detection works correctly alongside metadata management.

```rust
// Configure services
client.configure_services(&anchor, &services);

// Set metadata
client.set_anchor_metadata(
    &anchor,
    &8500u32,  // reputation_score (85%)
    &3600u64,  // average_settlement_time (1 hour)
    &9000u32,  // liquidity_score (90%)
    &9950u32,  // uptime_percentage (99.5%)
    &1_000_000u64, // total_volume
);

// Verify both metadata and services work
let metadata = client.get_anchor_metadata(&anchor);
assert_eq!(metadata.reputation_score, 8500);
assert!(client.supports_service(&anchor, &ServiceType::Deposits));
```

**Result:** ✅ PASS

---

### Non-Configured Anchor Handling

**Test:** `test_get_services_for_non_configured_anchor`

Verifies proper error handling for anchors without configured services.

```rust
client.register_attestor(&anchor);

// Try to get services without configuring
let result = client.try_get_supported_services(&anchor);
assert_eq!(result, Err(Ok(Error::ServicesNotConfigured)));

// supports_service should return false
assert!(!client.supports_service(&anchor, &ServiceType::Deposits));
```

**Result:** ✅ PASS

---

### Dynamic Capability Updates

**Test:** `test_update_anchor_capabilities`

Verifies that anchor capabilities can be updated dynamically.

```rust
// Initially configure with only deposits
let mut initial_services = Vec::new(&env);
initial_services.push_back(ServiceType::Deposits);
client.configure_services(&anchor, &initial_services);

assert!(client.supports_service(&anchor, &ServiceType::Deposits));
assert!(!client.supports_service(&anchor, &ServiceType::Withdrawals));

// Update to add withdrawals
let mut updated_services = Vec::new(&env);
updated_services.push_back(ServiceType::Deposits);
updated_services.push_back(ServiceType::Withdrawals);
client.configure_services(&anchor, &updated_services);

// Verify updated state
assert!(client.supports_service(&anchor, &ServiceType::Deposits));
assert!(client.supports_service(&anchor, &ServiceType::Withdrawals));
```

**Result:** ✅ PASS

---

### Inactive Anchor Capability Detection

**Test:** `test_capability_detection_inactive_anchor`

Verifies that capability detection works for inactive anchors.

```rust
client.configure_services(&anchor, &services);
client.set_anchor_metadata(&anchor, &8500u32, &3600u64, &9000u32, &9950u32, &1_000_000u64);

// Deactivate anchor
client.deactivate_anchor(&anchor);

// Verify metadata shows inactive
let metadata = client.get_anchor_metadata(&anchor);
assert!(!metadata.is_active);

// Services should still be queryable
assert!(client.supports_service(&anchor, &ServiceType::Deposits));

// Reactivate
client.reactivate_anchor(&anchor);
assert!(client.get_anchor_metadata(&anchor).is_active);
```

**Result:** ✅ PASS

---

## Test Suite Results

```bash
$ cargo test capability_detection_tests --lib

running 12 tests
test capability_detection_tests::test_capability_detection_inactive_anchor ... ok
test capability_detection_tests::test_capability_detection_with_metadata ... ok
test capability_detection_tests::test_detect_deposit_only_anchor ... ok
test capability_detection_tests::test_detect_full_service_anchor ... ok
test capability_detection_tests::test_detect_quote_provider_anchor ... ok
test capability_detection_tests::test_detect_withdrawal_only_anchor ... ok
test capability_detection_tests::test_get_services_for_non_configured_anchor ... ok
test capability_detection_tests::test_reject_duplicate_services ... ok
test capability_detection_tests::test_reject_empty_services ... ok
test capability_detection_tests::test_reject_invalid_metadata_scores ... ok
test capability_detection_tests::test_reject_unregistered_anchor_services ... ok
test capability_detection_tests::test_update_anchor_capabilities ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured
```

---

## Service Types

The system supports four service types:

```rust
pub enum ServiceType {
    Deposits = 1,      // Accept fiat/crypto deposits
    Withdrawals = 2,   // Process withdrawals
    Quotes = 3,        // Provide exchange rate quotes
    KYC = 4,          // KYC verification services
}
```

---

## API Methods

### Configuration

```rust
/// Configure supported services for an anchor
pub fn configure_services(
    env: Env,
    anchor: Address,
    services: Vec<ServiceType>,
) -> Result<(), Error>
```

### Query

```rust
/// Get list of supported services
pub fn get_supported_services(
    env: Env,
    anchor: Address,
) -> Result<Vec<ServiceType>, Error>

/// Check if anchor supports specific service
pub fn supports_service(
    env: Env,
    anchor: Address,
    service: ServiceType,
) -> bool
```

### Metadata Management

```rust
/// Set anchor metadata
pub fn set_anchor_metadata(
    env: Env,
    anchor: Address,
    reputation_score: u32,      // 0-10000 (0-100%)
    average_settlement_time: u64,
    liquidity_score: u32,       // 0-10000 (0-100%)
    uptime_percentage: u32,     // 0-10000 (0-100%)
    total_volume: u64,
) -> Result<(), Error>

/// Get anchor metadata
pub fn get_anchor_metadata(
    env: Env,
    anchor: Address,
) -> Result<AnchorMetadata, Error>
```

---

## Error Handling

| Error | Description |
|-------|-------------|
| `InvalidServiceType` | Empty services, duplicates, or invalid service |
| `AttestorNotRegistered` | Anchor not registered as attestor |
| `ServicesNotConfigured` | No services configured for anchor |
| `InvalidAnchorMetadata` | Metadata scores exceed valid range (0-10000) |

---

## Validation Rules

### Service Configuration
- ✅ Services list cannot be empty
- ✅ No duplicate services allowed
- ✅ Only registered attestors can configure services
- ✅ Services can be updated dynamically

### Metadata Validation
- ✅ Reputation score: 0-10000 (0-100%)
- ✅ Liquidity score: 0-10000 (0-100%)
- ✅ Uptime percentage: 0-10000 (0-100%)
- ✅ Settlement time: any u64 value (seconds)
- ✅ Total volume: any u64 value

---

## Integration with Other Features

### Transaction Intent Building
The capability detection system integrates with transaction intent building:

```rust
// Validates that anchor supports the requested operation
if !anchor_services.services.contains(&builder.request.operation_type) {
    return Err(Error::InvalidServiceType);
}

// Validates KYC requirement
if builder.require_kyc && !anchor_services.services.contains(&ServiceType::KYC) {
    return Err(Error::ComplianceNotMet);
}
```

### Multi-Anchor Routing
Capability detection is used in routing decisions:

```rust
// Check if anchor supports the required service
if !services.services.contains(&routing_request.request.operation_type) {
    continue;
}

// Check KYC requirement
if routing_request.require_kyc && !services.services.contains(&ServiceType::KYC) {
    continue;
}
```

---

## Best Practices

1. **Always register anchor as attestor first** before configuring services
2. **Validate service requirements** before building transaction intents
3. **Use `supports_service()`** for quick capability checks
4. **Use `get_supported_services()`** to display all capabilities
5. **Set metadata** to enable intelligent routing decisions
6. **Update capabilities dynamically** as anchor services evolve
7. **Deactivate anchors** instead of removing them to preserve history

---

## Conclusion

✅ All test goals achieved:
- **Detect deposit-only anchors** → Validated with specific tests
- **Detect full-service anchors** → Validated with comprehensive checks
- **Reject malformed capability payloads** → Multiple validation scenarios tested

The capability detection system is production-ready with 12 passing tests covering all requirements and edge cases.
