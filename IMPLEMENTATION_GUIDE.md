# AnchorKit Session & Traceability Implementation Guide

## Overview

This guide documents the implementation of reproducible and traceable anchor interaction sessions in AnchorKit. The system ensures all contract operations are logged, auditable, and can be replayed deterministically.

## Architecture

### Core Components

#### 1. Session Management (`InteractionSession`)
- **Purpose**: Group related operations into logical units
- **Uniqueness**: Auto-incremented session IDs
- **Traceability**: Tracks initiator, creation time, and operation count
- **Replay Protection**: Includes nonce based on ledger sequence

```rust
pub struct InteractionSession {
    pub session_id: u64,        // Unique identifier
    pub initiator: Address,     // Who created the session
    pub created_at: u64,        // Unix timestamp
    pub operation_count: u64,   // Total operations in session
    pub nonce: u64,             // Ledger sequence for replay protection
}
```

#### 2. Operation Context (`OperationContext`)
- **Purpose**: Record detailed information about each operation
- **Sequencing**: Operation index ensures deterministic ordering
- **Status Tracking**: Records success/failure with result data
- **Timestamp**: Precise operation execution time

```rust
pub struct OperationContext {
    pub session_id: u64,        // Which session
    pub operation_index: u64,   // Sequential position (0-based)
    pub operation_type: String, // "init", "register", "attest", etc.
    pub timestamp: u64,         // Execution time
    pub status: String,         // "success" or error code
    pub result_data: u64,       // Result (e.g., attestation ID)
}
```

#### 3. Audit Log (`AuditLog`)
- **Purpose**: Immutable record of all operations
- **Completeness**: Combines session, operation, and actor info
- **Persistence**: Stored in persistent storage for long-term audit

```rust
pub struct AuditLog {
    pub log_id: u64,            // Unique log entry ID
    pub session_id: u64,        // Associated session
    pub operation: OperationContext,
    pub actor: Address,         // Who performed the operation
}
```

### Storage Layer

New storage keys added to support session management:

```rust
enum StorageKey {
    // ... existing keys ...
    SessionCounter,             // Global session ID counter
    Session(u64),              // Session data by ID
    SessionNonce(u64),         // Session nonce for replay protection
    AuditLogCounter,           // Global audit log counter
    AuditLog(u64),             // Audit log entries by ID
    SessionOperationCount(u64), // Operation count per session
}
```

**Storage Lifetime:**
- Instance storage (30 days): Session counter, audit log counter
- Persistent storage (90 days): Session data, audit logs, nonces

### Event System

#### SessionCreated Event
Emitted when a new session is created.

```rust
pub struct SessionCreated {
    pub session_id: u64,
    pub initiator: Address,
    pub timestamp: u64,
}
```

**Topic**: `("session", "created", session_id)`

#### OperationLogged Event
Emitted when an operation is recorded in a session.

```rust
pub struct OperationLogged {
    pub log_id: u64,
    pub session_id: u64,
    pub operation_index: u64,
    pub operation_type: String,
    pub status: String,
}
```

**Topic**: `("audit", "logged", log_id)`

## Implementation Details

### Session Creation Flow

```
1. User calls create_session(initiator)
   ↓
2. Verify contract is initialized
   ↓
3. Generate unique session_id (increment counter)
   ↓
4. Create nonce from current ledger sequence
   ↓
5. Store InteractionSession in persistent storage
   ↓
6. Store nonce for replay protection
   ↓
7. Emit SessionCreated event
   ↓
8. Return session_id
```

### Operation Logging Flow

```
1. User calls session-aware operation (e.g., submit_attestation_with_session)
   ↓
2. Verify session exists
   ↓
3. Execute operation logic
   ↓
4. On success/failure:
   - Increment session operation count
   - Create OperationContext with details
   - Store AuditLog entry
   - Emit OperationLogged event
   ↓
5. Return result
```

### Reproducibility Mechanism

Sessions are reproducible through:

1. **Deterministic Ordering**: Operation indices ensure exact replay order
2. **Complete Context**: All operation details stored (timestamp, actor, result)
3. **Immutable Logs**: Audit logs cannot be modified after creation
4. **Nonce Protection**: Session nonce prevents unauthorized replays
5. **Sequential IDs**: Both session and operation IDs are sequential

### Replay Protection

Two-level replay protection:

1. **Session Level**: Nonce based on ledger sequence prevents session reuse
2. **Operation Level**: Payload hash tracking prevents duplicate attestations

## API Methods

### Session Management

#### `create_session(initiator: Address) -> Result<u64, Error>`

Creates a new interaction session.

**Implementation:**
```rust
pub fn create_session(env: Env, initiator: Address) -> Result<u64, Error> {
    initiator.require_auth();
    Storage::get_admin(&env)?;  // Verify initialized
    
    let session_id = Storage::create_session(&env, &initiator);
    let timestamp = env.ledger().timestamp();
    
    SessionCreated::publish(&env, session_id, &initiator, timestamp);
    
    Ok(session_id)
}
```

**Storage Operations:**
- Increment session counter
- Create and store InteractionSession
- Store session nonce
- Initialize operation count to 0

#### `get_session(session_id: u64) -> Result<InteractionSession, Error>`

Retrieves session details.

**Implementation:**
```rust
pub fn get_session(env: Env, session_id: u64) -> Result<InteractionSession, Error> {
    Storage::get_session(&env, session_id)
}
```

#### `get_session_operation_count(session_id: u64) -> Result<u64, Error>`

Gets total operations in a session.

**Implementation:**
```rust
pub fn get_session_operation_count(env: Env, session_id: u64) -> Result<u64, Error> {
    Storage::get_session(&env, session_id)?;
    Ok(Storage::get_session_operation_count(&env, session_id))
}
```

#### `get_audit_log(log_id: u64) -> Result<AuditLog, Error>`

Retrieves a specific audit log entry.

**Implementation:**
```rust
pub fn get_audit_log(env: Env, log_id: u64) -> Result<AuditLog, Error> {
    Storage::get_audit_log(&env, log_id)
}
```

### Session-Aware Operations

#### `submit_attestation_with_session(...) -> Result<u64, Error>`

Submits attestation with full session logging.

**Key Differences from Standard Method:**
- Takes `session_id` parameter
- Logs operation on success and failure
- Returns attestation ID
- Emits both AttestationRecorded and OperationLogged events

**Implementation Flow:**
```rust
1. Verify issuer auth
2. Validate timestamp
3. Check attestor registration
4. Check replay attack
5. Verify signature
6. Create attestation
7. Store attestation
8. Mark hash as used
9. Emit AttestationRecorded event
10. Log operation with result_data = attestation_id
11. Emit OperationLogged event
12. Return attestation_id
```

#### `register_attestor_with_session(session_id, attestor) -> Result<(), Error>`

Registers attestor with session logging.

**Implementation Flow:**
```rust
1. Verify admin auth
2. Check attestor not already registered
3. Set attestor as registered
4. Emit AttestorAdded event
5. Log operation with status = "success"
6. Emit OperationLogged event
```

#### `revoke_attestor_with_session(session_id, attestor) -> Result<(), Error>`

Revokes attestor with session logging.

**Implementation Flow:**
```rust
1. Verify admin auth
2. Check attestor is registered
3. Set attestor as unregistered
4. Emit AttestorRemoved event
5. Log operation with status = "success"
6. Emit OperationLogged event
```

### Internal Helper

#### `log_session_operation(...) -> Result<u64, Error>`

Internal method to log operations within sessions.

**Parameters:**
- `session_id`: Session to log in
- `actor`: Address performing operation
- `operation_type`: Type of operation
- `status`: "success" or error code
- `result_data`: Operation result

**Implementation:**
```rust
fn log_session_operation(
    env: &Env,
    session_id: u64,
    actor: &Address,
    operation_type: &str,
    status: &str,
    result_data: u64,
) -> Result<u64, Error> {
    // Verify session exists
    Storage::get_session(env, session_id)?;
    
    // Get next operation index
    let operation_index = Storage::increment_session_operation_count(env, session_id);
    let timestamp = env.ledger().timestamp();
    
    // Create operation context
    let operation = OperationContext {
        session_id,
        operation_index,
        operation_type: String::from_str(env, operation_type),
        timestamp,
        status: String::from_str(env, status),
        result_data,
    };
    
    // Store audit log
    let log_id = Storage::log_operation(env, session_id, actor, &operation);
    
    // Emit event
    OperationLogged::publish(
        env,
        log_id,
        session_id,
        operation_index,
        &operation.operation_type,
        &operation.status,
    );
    
    Ok(log_id)
}
```

## Storage Operations

### Session Creation

```rust
pub fn create_session(env: &Env, initiator: &Address) -> u64 {
    let session_id = Self::get_and_increment_session_counter(env);
    let nonce = env.ledger().sequence() as u64;
    
    let session = InteractionSession {
        session_id,
        initiator: initiator.clone(),
        created_at: env.ledger().timestamp(),
        operation_count: 0,
        nonce,
    };
    
    // Store session
    let key = StorageKey::Session(session_id).to_storage_key(env);
    env.storage().persistent().set(&key, &session);
    env.storage()
        .persistent()
        .extend_ttl(&key, Self::PERSISTENT_LIFETIME, Self::PERSISTENT_LIFETIME);
    
    // Store nonce
    let nonce_key = StorageKey::SessionNonce(session_id).to_storage_key(env);
    env.storage().persistent().set(&nonce_key, &nonce);
    env.storage()
        .persistent()
        .extend_ttl(&nonce_key, Self::PERSISTENT_LIFETIME, Self::PERSISTENT_LIFETIME);
    
    session_id
}
```

### Operation Logging

```rust
pub fn log_operation(
    env: &Env,
    session_id: u64,
    actor: &Address,
    operation: &OperationContext,
) -> u64 {
    let log_id = Self::get_and_increment_audit_counter(env);
    
    let audit_log = AuditLog {
        log_id,
        session_id,
        operation: operation.clone(),
        actor: actor.clone(),
    };
    
    // Store audit log
    let key = StorageKey::AuditLog(log_id).to_storage_key(env);
    env.storage().persistent().set(&key, &audit_log);
    env.storage()
        .persistent()
        .extend_ttl(&key, Self::PERSISTENT_LIFETIME, Self::PERSISTENT_LIFETIME);
    
    log_id
}
```

## Error Handling

### New Error Codes

| Code | Name | Cause |
|------|------|-------|
| 13 | `SessionNotFound` | Session ID doesn't exist |
| 14 | `InvalidSessionId` | Session ID is invalid |
| 15 | `SessionReplayAttack` | Nonce mismatch detected |

### Error Scenarios

**SessionNotFound:**
- Calling operation with non-existent session_id
- Retrieving session that was never created

**InvalidSessionId:**
- Reserved for future use with session validation

**SessionReplayAttack:**
- Attempting to use session with different nonce
- Indicates potential unauthorized replay attempt

## Testing Strategy

### Unit Tests

1. **Session Creation**
   - Create session successfully
   - Verify session ID increments
   - Verify nonce is set correctly

2. **Operation Logging**
   - Log operation in session
   - Verify operation index increments
   - Verify audit log is stored

3. **Reproducibility**
   - Create session with multiple operations
   - Retrieve all operations in order
   - Verify operation indices are sequential

4. **Error Cases**
   - Session not found
   - Invalid session ID
   - Replay attack detection

### Integration Tests

1. **Multi-Operation Session**
   - Create session
   - Register attestor
   - Submit attestation
   - Verify all operations logged

2. **Session Verification**
   - Create session
   - Perform operations
   - Retrieve session details
   - Verify operation count matches

3. **Audit Trail**
   - Create session
   - Perform operations
   - Retrieve audit logs
   - Verify complete history

## Performance Considerations

### Storage Efficiency

- **Session Counter**: Single instance storage entry (minimal)
- **Session Data**: One persistent entry per session
- **Audit Logs**: One persistent entry per operation
- **Nonce Storage**: One persistent entry per session

### Gas Optimization

- Minimal event data (only essential fields)
- Efficient storage key generation
- TTL management prevents storage bloat
- Sequential IDs avoid hash lookups

### Scalability

- Session IDs are u64 (supports 18 quintillion sessions)
- Operation indices are u64 (supports 18 quintillion operations per session)
- Audit log IDs are u64 (supports 18 quintillion total operations)

## Security Considerations

### Replay Protection

1. **Session Nonce**: Based on ledger sequence, unique per session
2. **Payload Hash Tracking**: Prevents duplicate attestations
3. **Operation Sequencing**: Ensures deterministic ordering

### Audit Trail Integrity

1. **Immutable Logs**: Audit logs cannot be modified
2. **Complete Context**: All operation details recorded
3. **Actor Tracking**: Records who performed each operation

### Authorization

1. **Session Creation**: Requires initiator auth
2. **Operation Logging**: Automatic, no additional auth needed
3. **Audit Retrieval**: Public read access (no auth required)

## Migration Path

For existing contracts:

1. **Backward Compatibility**: Old methods still work unchanged
2. **Gradual Adoption**: Use session-aware methods for new operations
3. **Audit Trail**: Historical operations not logged (only new ones)
4. **No Breaking Changes**: Existing functionality unaffected

## Conclusion

The session and traceability system provides:

- **Reproducibility**: Sessions can be replayed deterministically
- **Traceability**: Complete audit trail of all operations
- **Security**: Replay protection at multiple levels
- **Scalability**: Efficient storage and gas usage
- **Compliance**: Full audit trail for regulatory requirements

This implementation ensures AnchorKit can be used in regulated environments where complete auditability and reproducibility are critical requirements.
