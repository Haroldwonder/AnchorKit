# AnchorKit Session Traceability - Quick Start

## What Was Added

AnchorKit now supports **reproducible and traceable anchor interaction sessions**. Every operation can be logged, audited, and replayed deterministically.

## Key Features

✅ **Session Management** - Group related operations  
✅ **Operation Logging** - Track every action with full context  
✅ **Audit Trail** - Immutable record of all operations  
✅ **Replay Protection** - Prevent unauthorized replays  
✅ **Reproducibility** - Deterministic operation replay  

## Quick Example

```javascript
// 1. Create a session
const sessionId = await contract.create_session(userAddress);

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
console.log(`Session has ${operationCount} operations`);

// 4. Retrieve audit logs
const auditLog = await contract.get_audit_log(0);
console.log(`Operation: ${auditLog.operation.operation_type}`);
console.log(`Status: ${auditLog.operation.status}`);
```

## New Methods

### Session Management

| Method | Purpose |
|--------|---------|
| `create_session(initiator)` | Create new session |
| `get_session(session_id)` | Get session details |
| `get_session_operation_count(session_id)` | Get operation count |
| `get_audit_log(log_id)` | Get audit log entry |

### Session-Aware Operations

| Method | Purpose |
|--------|---------|
| `submit_attestation_with_session(...)` | Submit attestation with logging |
| `register_attestor_with_session(...)` | Register attestor with logging |
| `revoke_attestor_with_session(...)` | Revoke attestor with logging |

## New Data Structures

### InteractionSession
```rust
pub struct InteractionSession {
    pub session_id: u64,        // Unique ID
    pub initiator: Address,     // Who created it
    pub created_at: u64,        // Timestamp
    pub operation_count: u64,   // Total operations
    pub nonce: u64,             // Replay protection
}
```

### OperationContext
```rust
pub struct OperationContext {
    pub session_id: u64,        // Which session
    pub operation_index: u64,   // Position in session
    pub operation_type: String, // Type of operation
    pub timestamp: u64,         // When it happened
    pub status: String,         // Success/failure
    pub result_data: u64,       // Result (e.g., ID)
}
```

### AuditLog
```rust
pub struct AuditLog {
    pub log_id: u64,            // Unique log ID
    pub session_id: u64,        // Associated session
    pub operation: OperationContext,
    pub actor: Address,         // Who did it
}
```

## New Events

### SessionCreated
Emitted when session is created.
```
Topic: ("session", "created", session_id)
Data: { session_id, initiator, timestamp }
```

### OperationLogged
Emitted when operation is logged.
```
Topic: ("audit", "logged", log_id)
Data: { log_id, session_id, operation_index, operation_type, status }
```

## New Error Codes

| Code | Name | Meaning |
|------|------|---------|
| 13 | `SessionNotFound` | Session doesn't exist |
| 14 | `InvalidSessionId` | Invalid session ID |
| 15 | `SessionReplayAttack` | Nonce mismatch |

## Usage Patterns

### Pattern 1: Single Operation Session
```javascript
const sessionId = await contract.create_session(admin);
await contract.register_attestor_with_session(sessionId, attestor);
```

### Pattern 2: Batch Operations
```javascript
const sessionId = await contract.create_session(admin);

// Register multiple attestors
await contract.register_attestor_with_session(sessionId, attestor1);
await contract.register_attestor_with_session(sessionId, attestor2);

// Submit multiple attestations
await contract.submit_attestation_with_session(sessionId, issuer1, ...);
await contract.submit_attestation_with_session(sessionId, issuer2, ...);

// Verify all logged
const count = await contract.get_session_operation_count(sessionId);
assert(count === 4);
```

### Pattern 3: Audit Verification
```javascript
const session = await contract.get_session(sessionId);

for (let i = 0; i < session.operation_count; i++) {
    const auditLog = await contract.get_audit_log(i);
    console.log(`Op ${i}: ${auditLog.operation.operation_type}`);
}
```

## Files Modified

- `src/lib.rs` - Added session management methods
- `src/storage.rs` - Added session storage operations
- `src/events.rs` - Added session events
- `src/types.rs` - Added session data structures
- `src/errors.rs` - Added session error codes

## Documentation

- **SESSION_TRACEABILITY.md** - Complete feature guide
- **IMPLEMENTATION_GUIDE.md** - Technical implementation details
- **QUICK_START.md** - This file

## Backward Compatibility

✅ All existing methods still work unchanged  
✅ No breaking changes to API  
✅ Session features are opt-in  
✅ Gradual adoption possible  

## Next Steps

1. Read `SESSION_TRACEABILITY.md` for complete feature guide
2. Review `IMPLEMENTATION_GUIDE.md` for technical details
3. Use session-aware methods for new operations
4. Monitor `SessionCreated` and `OperationLogged` events
5. Implement audit log verification in your application

## Support

For questions or issues:
1. Check SESSION_TRACEABILITY.md FAQ section
2. Review IMPLEMENTATION_GUIDE.md error handling
3. Examine test cases in src/lib.rs
