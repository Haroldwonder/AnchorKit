# Anchor Services Feature

## Overview

The Anchor Services feature allows AnchorKit to query and return the list of supported services for a given anchor. This enables clients to discover which operations an anchor supports before attempting to use them.

## Supported Service Types

The following service types are available:

1. **Deposits** - Anchor accepts incoming deposits from users
2. **Withdrawals** - Anchor processes withdrawal requests
3. **Quotes** - Anchor provides exchange rate quotes
4. **KYC** - Anchor performs Know Your Customer verification

## Public API Methods

### `configure_services(env: Env, anchor: Address, services: Vec<ServiceType>) -> Result<(), Error>`

Configures the list of supported services for an anchor.

**Authorization:**
- Callable by the anchor itself OR the contract admin
- Requires authentication from the caller

**Validation:**
- Anchor must be a registered attestor
- Services list cannot be empty
- Services list cannot contain duplicates
- Can be called multiple times to update services

**Example:**
```rust
let mut services = Vec::new(&env);
services.push_back(ServiceType::Deposits);
services.push_back(ServiceType::Withdrawals);
services.push_back(ServiceType::KYC);

contract.configure_services(&anchor, &services)?;
```

### `get_supported_services(env: Env, anchor: Address) -> Result<Vec<ServiceType>, Error>`

Retrieves the complete list of services supported by an anchor.

**Returns:**
- `Ok(Vec<ServiceType>)` - List of supported services
- `Err(Error::ServicesNotConfigured)` - If anchor hasn't configured services

**Example:**
```rust
let services = contract.get_supported_services(&anchor)?;
for service in services.iter() {
    // Process each service
}
```

### `supports_service(env: Env, anchor: Address, service: ServiceType) -> bool`

Checks if an anchor supports a specific service. This method never fails.

**Returns:**
- `true` - Anchor supports the service
- `false` - Anchor doesn't support the service or hasn't configured services

**Example:**
```rust
if contract.supports_service(&anchor, &ServiceType::Deposits) {
    // Process deposit
} else {
    // Handle unsupported service
}
```

## Error Codes

| Code | Name | Description |
|------|------|-------------|
| 13 | `ServicesNotConfigured` | Anchor has not configured supported services |
| 14 | `InvalidServiceType` | Service type list is invalid (empty or contains duplicates) |

## Events

### ServicesConfigured Event

Emitted when an anchor configures or updates their supported services.

**Topic:** `("services", "config")`

**Data:**
```rust
struct ServicesConfigured {
    pub anchor: Address,
    pub services: Vec<ServiceType>,
}
```

## Security Considerations

1. **Authorization**: Only the anchor or admin can configure services
2. **Validation**: Services list is validated to prevent empty or duplicate entries
3. **Attestor Requirement**: Only registered attestors can configure services
4. **No Breaking Changes**: Error codes are in the stable 100-120 range

## Storage

Services are stored in persistent storage with:
- Key: `("SERVICES", anchor_address)`
- Value: `AnchorServices` struct containing anchor address and services list
- TTL: 90 days (PERSISTENT_LIFETIME)

## Integration Example

```rust
// 1. Initialize contract and register anchor
contract.initialize(&admin);
contract.register_attestor(&anchor);

// 2. Configure supported services
let mut services = Vec::new(&env);
services.push_back(ServiceType::Deposits);
services.push_back(ServiceType::Withdrawals);
services.push_back(ServiceType::Quotes);
services.push_back(ServiceType::KYC);
contract.configure_services(&anchor, &services)?;

// 3. Query services before operations
if contract.supports_service(&anchor, &ServiceType::Deposits) {
    // Proceed with deposit
    let deposit_result = process_deposit(&anchor, amount);
}

// 4. Get all supported services
let all_services = contract.get_supported_services(&anchor)?;
display_services_to_user(all_services);
```

## Testing

The feature includes comprehensive tests covering:
- ✅ Configuring services for an anchor
- ✅ Configuring all four service types
- ✅ Checking individual service support
- ✅ Handling unconfigured anchors
- ✅ Rejecting unregistered anchors
- ✅ Rejecting empty service lists
- ✅ Rejecting duplicate services
- ✅ Updating services
- ✅ Multiple anchors with different services
- ✅ Event emission verification

All tests pass without errors.
