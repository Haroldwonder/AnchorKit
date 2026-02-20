# AnchorKit API Specification

## Overview
This document defines the stable API contract for the AnchorKit smart contract, including error codes, event schemas, and method signatures.

## Error Codes

All error codes are in the range 100-120 and are guaranteed to remain stable across contract versions.

| Code | Name | Description | When It Occurs |
|------|------|-------------|----------------|
| 100 | `AlreadyInitialized` | Contract has already been initialized | Attempting to call `initialize()` more than once |
| 101 | `NotInitialized` | Contract has not been initialized yet | Calling methods that require initialization before `initialize()` is called |
| 102 | `UnauthorizedAttestor` | Caller is not a registered attestor | Attempting to submit an attestation without being registered as an attestor |
| 103 | `AttestorAlreadyRegistered` | Attestor is already registered | Attempting to register an attestor that is already registered |
| 104 | `AttestorNotRegistered` | Attestor is not registered | Attempting to revoke an attestor that is not registered |
| 105 | `ReplayAttack` | Attestation with this hash has already been submitted | Attempting to submit an attestation with a payload hash that has already been used |
| 106 | `InvalidTimestamp` | Timestamp is invalid (zero or in the future) | Submitting an attestation with a timestamp of 0 |
| 107 | `AttestationNotFound` | Attestation with the given ID was not found | Querying an attestation ID that doesn't exist |
| 108 | `InvalidPublicKey` | Public key format is invalid | Reserved for future use with signature verification |
| 113 | `ServicesNotConfigured` | Anchor has not configured supported services | Querying services for an anchor that hasn't configured them |
| 114 | `InvalidServiceType` | Service type list is invalid | Providing empty list or duplicate services |

### Error Code Stability Guarantee
Error codes 100-120 are reserved for AnchorKit and will never be reassigned. New errors will be added to the end of the range, ensuring backward compatibility.

## Public Methods

All public methods return `Result<T, Error>` where `T` is the return type and `Error` is the error enum defined above.

### `initialize(env: Env, admin: Address) -> Result<(), Error>`
Initializes the contract with an admin address. Can only be called once.

**Returns:**
- `Ok(())` on success
- `Err(Error::AlreadyInitialized)` if already initialized

### `register_attestor(env: Env, attestor: Address) -> Result<(), Error>`
Registers a new attestor. Only callable by admin.

**Returns:**
- `Ok(())` on success
- `Err(Error::NotInitialized)` if contract not initialized
- `Err(Error::AttestorAlreadyRegistered)` if attestor already registered

### `revoke_attestor(env: Env, attestor: Address) -> Result<(), Error>`
Revokes an attestor. Only callable by admin.

**Returns:**
- `Ok(())` on success
- `Err(Error::NotInitialized)` if contract not initialized
- `Err(Error::AttestorNotRegistered)` if attestor not registered

### `submit_attestation(env: Env, issuer: Address, subject: Address, timestamp: u64, payload_hash: BytesN<32>, signature: Bytes) -> Result<u64, Error>`
Submits an attestation. Must be signed by a registered attestor.

**Returns:**
- `Ok(attestation_id)` on success
- `Err(Error::InvalidTimestamp)` if timestamp is 0
- `Err(Error::UnauthorizedAttestor)` if issuer is not registered
- `Err(Error::ReplayAttack)` if payload hash already used

### `get_attestation(env: Env, id: u64) -> Result<Attestation, Error>`
Retrieves an attestation by ID.

**Returns:**
- `Ok(attestation)` on success
- `Err(Error::AttestationNotFound)` if ID doesn't exist

### `get_admin(env: Env) -> Result<Address, Error>`
Gets the admin address.

**Returns:**
- `Ok(admin_address)` on success
- `Err(Error::NotInitialized)` if contract not initialized

### `is_attestor(env: Env, attestor: Address) -> bool`
Checks if an address is a registered attestor. This method never fails.

**Returns:**
- `true` if registered
- `false` if not registered

### `configure_services(env: Env, anchor: Address, services: Vec<ServiceType>) -> Result<(), Error>`
Configures the list of supported services for an anchor. Only callable by the anchor or admin.

**Service Types:**
- `Deposits` - Anchor supports deposit operations
- `Withdrawals` - Anchor supports withdrawal operations
- `Quotes` - Anchor supports quote generation
- `KYC` - Anchor supports KYC verification

**Returns:**
- `Ok(())` on success
- `Err(Error::NotInitialized)` if contract not initialized
- `Err(Error::AttestorNotRegistered)` if anchor is not a registered attestor
- `Err(Error::InvalidServiceType)` if services list is empty or contains duplicates

### `get_supported_services(env: Env, anchor: Address) -> Result<Vec<ServiceType>, Error>`
Retrieves the list of services supported by an anchor.

**Returns:**
- `Ok(Vec<ServiceType>)` containing the list of supported services
- `Err(Error::ServicesNotConfigured)` if anchor hasn't configured services

### `supports_service(env: Env, anchor: Address, service: ServiceType) -> bool`
Checks if an anchor supports a specific service. This method never fails.

**Returns:**
- `true` if the anchor supports the service
- `false` if the anchor doesn't support the service or hasn't configured services

## Event Schema

All events follow a standardized format to minimize gas costs while maintaining queryability.

### Event Format
Events use the following structure:
```
(Topic, ID/SubjectAddr, Data)
```

Where:
- **Topic**: A tuple of symbols identifying the event type
- **ID/SubjectAddr**: The primary identifier (attestation ID or subject address)
- **Data**: Minimal data structure containing only essential information

### AttestorAdded Event

**Topic:** `("attestor", "added", attestor_address)`

**Data:**
```rust
struct AttestorAddedData {}  // Empty for gas efficiency
```

**Example:**
```
Topic: ("attestor", "added", GABC...XYZ)
Data: {}
```

### AttestorRemoved Event

**Topic:** `("attestor", "removed", attestor_address)`

**Data:**
```rust
struct AttestorRemovedData {}  // Empty for gas efficiency
```

**Example:**
```
Topic: ("attestor", "removed", GABC...XYZ)
Data: {}
```

### AttestationRecorded Event

**Topic:** `("attest", "recorded", attestation_id, subject_address)`

**Data:**
```rust
struct AttestationRecordedData {
    pub timestamp: u64,
    pub payload_hash: BytesN<32>,
}
```

**Example:**
```
Topic: ("attest", "recorded", 42, GDEF...ABC)
Data: {
    timestamp: 1234567890,
    payload_hash: 0x1234...5678
}
```

### ServicesConfigured Event

**Topic:** `("services", "config")`

**Data:**
```rust
struct ServicesConfigured {
    pub anchor: Address,
    pub services: Vec<ServiceType>,
}
```

**Example:**
```
Topic: ("services", "config")
Data: {
    anchor: GABC...XYZ,
    services: [Deposits, Withdrawals, KYC]
}
```

### Event Indexing

Events are designed for efficient querying:
- Filter by event type using the first topic symbol
- Filter by subject address using the address in the topic
- Filter by attestation ID using the ID in the topic
- Access timestamp and payload hash from the data payload

## Frontend Integration

### Parsing Error Codes

Frontend applications should handle error codes in the 100-120 range:

```javascript
const ERROR_CODES = {
  100: 'Contract already initialized',
  101: 'Contract not initialized',
  102: 'Unauthorized attestor',
  103: 'Attestor already registered',
  104: 'Attestor not registered',
  105: 'Replay attack detected',
  106: 'Invalid timestamp',
  107: 'Attestation not found',
  108: 'Invalid public key',
  113: 'Services not configured',
  114: 'Invalid service type'
};

function parseError(error) {
  const code = error.code;
  if (code >= 100 && code <= 120) {
    return ERROR_CODES[code] || 'Unknown error';
  }
  return 'Unexpected error';
}
```

### Event Listening

Frontend applications can listen for events:

```javascript
// Listen for attestor additions
contract.on('attestor:added', (attestorAddress) => {
  console.log(`Attestor added: ${attestorAddress}`);
});

// Listen for attestations
contract.on('attest:recorded', (attestationId, subjectAddress, data) => {
  console.log(`Attestation ${attestationId} recorded for ${subjectAddress}`);
  console.log(`Timestamp: ${data.timestamp}`);
  console.log(`Payload hash: ${data.payload_hash}`);
});

// Listen for services configuration
contract.on('services:config', (data) => {
  console.log(`Services configured for anchor: ${data.anchor}`);
  console.log(`Supported services: ${data.services.join(', ')}`);
});
```

## Versioning

This API specification follows semantic versioning:
- **Major version**: Breaking changes to error codes or event schemas
- **Minor version**: New error codes or events added
- **Patch version**: Documentation updates

Current version: **1.0.0**

## Compatibility Guarantees

1. **Error codes 100-120 will never be reassigned**
2. **Event topic structures will remain stable**
3. **Event data structures may only add optional fields**
4. **Method signatures will not change in breaking ways**

Any breaking changes will result in a new major version and a migration guide.
