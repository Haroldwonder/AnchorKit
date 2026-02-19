# AnchorKit Session Traceability & Reproducibility Guide

## Overview

AnchorKit now provides comprehensive session management and operation tracing to ensure all anchor interactions are **reproducible** and **traceable**. This enables:

- **Full Audit Trail**: Every operation is logged with complete context
- **Reproducibility**: Sessions can be replayed deterministically
- **Replay Protection**: Session nonces prevent unauthorized replay attacks
- **Traceability**: Track who did what, when, and with what result

## Core Concepts

### Interaction Sessions

An `InteractionSession` represents a logical grouping of related contract operations. Each session:

- Has a unique `session_id` (auto-incremented)
- Tracks the `initiator` (who created the session)
- Records `created_at` timestamp
- Maintains `operation_count` for completeness verification
- Includes a `nonce` for replay protection

```rust
pub struct InteractionSession {
    pub session_id: u64,
    pub initiator: Address,
    pub created_at: u64,
    pub operation_count: u64,
    pub nonce: u64,
}
```

### Operation Context

Each operation within a session is recorded with full context:

```rust
pub struct OperationContext {
    pub session_id: u64,           // Which session this belongs to
    pub operation_index: u64,      // Sequential position (0-based)
    pub operation_type: String,    // "init", "register", "attest", etc.
    pub timestamp: u64,            // When it executed
    pub status: String,            // "success" or error code
    pub result_data: u64,          // Result (e.g., attestation ID)
}
```

### Audit Logs

Complete audit trail entries combine session, operation, and actor information:

```rust
pub struct AuditLog {
    pub log_id: u64,               // Unique log entry ID
    pub session_id: u64,           // Associated session
    pub operation: OperationContext,
    pub actor: Address,            // Who performed the operation
}
```

## Usage Patterns

### Pattern 1: Basic Session Workflow

```javascript
// 1. Create a session
const sessionId = await contract.create_session(initiator);

// 2. Perform operations within the session
const attestationId = await contract.submit_attestation_with_session(
    sessionId,
    issuer,
    subject,
    timestamp,
    payloadHash,
    signature
);

// 3. Verify session completeness
const operationCount = await contract.get_session_operation_count(sessionId);
console.log(`Session ${sessionId} has ${operationCount} operations`);

// 4. Retrieve audit logs for verification
const auditLog = await contract.get_audit_log(logId);
console.log(`Operation: ${auditLog.operation.operation_type}`);
console.log(`Status: ${auditLog.operation.status}`);
```

### Pattern 2: Multi-Operation Session

```javascript
// Create session for batch operations
const sessionId = await contract.create_session(admin);

// Register multiple attestors within same session
await contract.register_attestor_with_session(sessionId, attestor1);
await contract.register_attestor_with_session(sessionId, attestor2);

// Submit attestations within same session
await contract.submit_attestation_with_session(
    sessionId, issuer1, subject1, timestamp1, hash1, sig1
);
await contract.submit_attestation_with_session(
    sessionId, issuer2, subject2, timestamp2, hash2, sig2
);

// Verify all operations are logged
const count = await contract.get_session_operation_count(sessionId);
assert(count === 4, "Expected 4 operations");
```

### Pattern 3: Reproducibility Verification

```javascript
// Retrieve session details
const session = await contract.get_session(sessionId);

// Verify session integrity
console.log(`Session created at: ${session.created_at}`);
console.log(`Initiated by: ${session.initiator}`);
console.log(`Total operations: ${session.operation_count}`);
console.log(`Session nonce: ${session.nonce}`);

// Replay all operations in order
for (let i = 0; i < session.operation_count; i++) {
    // Retrieve audit log for operation i
    const auditLog = await contract.get_audit_log(i);
    
    // Verify operation details
    console.log(`Operation ${i}: ${auditLog.operation.operation_type}`);
    console.log(`Status: ${auditLog.operation.status}`);
    console.log(`Result: ${auditLog.operation.result_data}`);
}
```

## API Reference

### Session Management Methods

#### `create_session(initiator: Address) -> Result<u64, Error>`

Creates a new interaction session.

**Parameters:**
- `initiator`: Address that initiates the session (must sign)

**Returns:**
- `Ok(session_id)`: Unique session identifier
- `Err(Error::NotInitialized)`: Contract not initialized

**Events:**
- `SessionCreated` with session_id, initiator, and timestamp

**Example:**
```javascript
const sessionId = await contract.create_session(userAddress);
```

#### `get_session(session_id: u64) -> Result<InteractionSession, Error>`

Retrieves session details for reproducibility verification.

**Parameters:**
- `session_id`: The session to retrieve

**Returns:**
- `Ok(session)`: Complete session information
- `Err(Error::SessionNotFound)`: Session doesn't exist

**Example:**
```javascript
const session = await contract.get_session(sessionId);
console.log(`Operations in session: ${session.operation_count}`);
```

#### `get_session_operation_count(session_id: u64) -> Result<u64, Error>`

Gets the total number of operations in a session.

**Parameters:**
- `session_id`: The session to query

**Returns:**
- `Ok(count)`: Number of operations
- `Err(Error::SessionNotFound)`: Session doesn't exist

**Example:**
```javascript
const count = await contract.get_session_operation_count(sessionId);
```

#### `get_audit_log(log_id: u64) -> Result<AuditLog, Error>`

Retrieves a specific audit log entry.

**Parameters:**
- `log_id`: The audit log entry ID

**Returns:**
- `Ok(audit_log)`: Complete audit log with operation context
- `Err(Error::SessionNotFound)`: Log entry doesn't exist

**Example:**
```javascript
const auditLog = await contract.get_audit_log(logId);
console.log(`Actor: ${auditLog.actor}`);
console.log(`Operation: ${auditLog.operation.operation_type}`);
```

### Session-Aware Operation Methods

#### `submit_attestation_with_session(...) -> Result<u64, Error>`

Submits an attestation and logs it within a session.

**Parameters:**
- `session_id`: Session to log operation in
- `issuer`: Attestor address (must sign)
- `subject`: Subject address
- `timestamp`: Operation timestamp
- `payload_hash`: 32-byte payload hash
- `signature`: Ed25519 signature

**Returns:**
- `Ok(attestation_id)`: Created attestation ID
- `Err(Error::InvalidTimestamp)`: Timestamp is 0
- `Err(Error::UnauthorizedAttestor)`: Issuer not registered
- `Err(Error::ReplayAttack)`: Hash already used
- `Err(Error::SessionNotFound)`: Session doesn't exist

**Events:**
- `AttestationRecorded` with attestation details
- `OperationLogged` with audit trail

**Example:**
```javascript
const attestationId = await contract.submit_attestation_with_session(
    sessionId,
    issuer,
    subject,
    timestamp,
    payloadHash,
    signature
);
```

#### `register_attestor_with_session(session_id: u64, attestor: Address) -> Result<(), Error>`

Registers an attestor and logs it within a session.

**Parameters:**
- `session_id`: Session to log operation in
- `attestor`: Address to register

**Returns:**
- `Ok(())`: Success
- `Err(Error::AttestorAlreadyRegistered)`: Already registered
- `Err(Error::SessionNotFound)`: Session doesn't exist

**Events:**
- `AttestorAdded` with attestor address
- `OperationLogged` with audit trail

#### `revoke_attestor_with_session(session_id: u64, attestor: Address) -> Result<(), Error>`

Revokes an attestor and logs it within a session.

**Parameters:**
- `session_id`: Session to log operation in
- `attestor`: Address to revoke

**Returns:**
- `Ok(())`: Success
- `Err(Error::AttestorNotRegistered)`: Not registered
- `Err(Error::SessionNotFound)`: Session doesn't exist

**Events:**
- `AttestorRemoved` with attestor address
- `OperationLogged` with audit trail

## Events for Traceability

### SessionCreated Event

Emitted when a new session is created.

**Topic:** `("session", "created", session_id)`

**Data:**
```rust
struct SessionCreated {
    pub session_id: u64,
    pub initiator: Address,
    pub timestamp: u64,
}
```

**Use Case:** Track session creation for audit purposes

### OperationLogged Event

Emitted when an operation is recorded in a session.

**Topic:** `("audit", "logged", log_id)`

**Data:**
```rust
struct OperationLogged {
    pub log_id: u64,
    pub session_id: u64,
    pub operation_index: u64,
    pub operation_type: String,
    pub status: String,
}
```

**Use Case:** Real-time tracking of all contract operations

## Reproducibility Guarantees

### Deterministic Replay

Sessions are designed for deterministic replay:

1. **Sequential Operations**: Each operation has an `operation_index` ensuring order
2. **Complete Context**: All operation details are stored (timestamp, actor, result)
3. **Immutable Logs**: Audit logs cannot be modified after creation
4. **Nonce Protection**: Session nonces prevent unauthorized replays

### Verification Checklist

To verify a session is reproducible:

```javascript
async function verifySessionReproducibility(sessionId) {
    const session = await contract.get_session(sessionId);
    
    // 1. Verify session exists and has operations
    if (session.operation_count === 0) {
        throw new Error("Session has no operations");
    }
    
    // 2. Verify all operations are logged
    for (let i = 0; i < session.operation_count; i++) {
        const auditLog = await contract.get_audit_log(i);
        if (auditLog.session_id !== sessionId) {
            throw new Error(`Operation ${i} not in session`);
        }
        if (auditLog.operation.operation_index !== i) {
            throw new Error(`Operation ${i} has wrong index`);
        }
    }
    
    // 3. Verify session integrity
    console.log(`✓ Session ${sessionId} is reproducible`);
    console.log(`  - Created: ${new Date(session.created_at * 1000)}`);
    console.log(`  - Operations: ${session.operation_count}`);
    console.log(`  - Initiator: ${session.initiator}`);
    
    return true;
}
```

## Error Handling

### Session-Related Errors

| Error | Code | Cause | Resolution |
|-------|------|-------|-----------|
| `SessionNotFound` | 13 | Session ID doesn't exist | Verify session was created |
| `InvalidSessionId` | 14 | Session ID is invalid | Use valid session ID |
| `SessionReplayAttack` | 15 | Nonce mismatch detected | Don't replay with different nonce |

### Common Issues

**Issue**: "SessionNotFound when calling submit_attestation_with_session"
- **Cause**: Session ID doesn't exist
- **Solution**: Create session first with `create_session()`

**Issue**: "Operation count doesn't match expected"
- **Cause**: Some operations failed silently
- **Solution**: Check audit logs for failed operations with status != "success"

**Issue**: "Can't replay session operations"
- **Cause**: Session nonce changed
- **Solution**: Use original session nonce from `get_session()`

## Best Practices

1. **Always Create Sessions**: Use session-aware methods for all critical operations
2. **Verify Completeness**: Check operation count matches expected operations
3. **Log Audit Trail**: Store audit log IDs for external verification
4. **Monitor Events**: Listen to `OperationLogged` events for real-time tracking
5. **Periodic Verification**: Regularly verify session reproducibility
6. **Archive Sessions**: Store session data for long-term audit compliance

## Integration Example

```javascript
class AnchorKitSessionManager {
    constructor(contract) {
        this.contract = contract;
        this.sessions = new Map();
    }

    async startSession(initiator) {
        const sessionId = await this.contract.create_session(initiator);
        this.sessions.set(sessionId, {
            id: sessionId,
            initiator,
            operations: [],
            startTime: Date.now()
        });
        return sessionId;
    }

    async submitAttestation(sessionId, issuer, subject, timestamp, hash, sig) {
        const attestationId = await this.contract.submit_attestation_with_session(
            sessionId, issuer, subject, timestamp, hash, sig
        );
        
        const session = this.sessions.get(sessionId);
        session.operations.push({
            type: 'attest',
            result: attestationId,
            timestamp: Date.now()
        });
        
        return attestationId;
    }

    async verifySession(sessionId) {
        const session = await this.contract.get_session(sessionId);
        const localSession = this.sessions.get(sessionId);
        
        if (session.operation_count !== localSession.operations.length) {
            throw new Error("Operation count mismatch");
        }
        
        return {
            verified: true,
            duration: Date.now() - localSession.startTime,
            operations: session.operation_count
        };
    }
}
```

## Conclusion

AnchorKit's session and traceability system ensures that all anchor interactions are:

- **Reproducible**: Can be replayed deterministically
- **Traceable**: Full audit trail of all operations
- **Secure**: Protected against replay attacks
- **Verifiable**: Complete context for verification

Use these features to build trustworthy, auditable anchor systems on Stellar.
