use soroban_sdk::{contracttype, Address, Bytes, BytesN, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attestation {
    pub id: u64,
    pub issuer: Address,
    pub subject: Address,
    pub timestamp: u64,
    pub payload_hash: BytesN<32>,
    pub signature: Bytes,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Endpoint {
    pub url: String,
    pub attestor: Address,
    pub is_active: bool,
}

/// Represents a reproducible interaction session.
/// Each session is uniquely identified and tracks all operations within it.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractionSession {
    /// Unique session identifier
    pub session_id: u64,
    /// Address that initiated the session
    pub initiator: Address,
    /// Unix timestamp when session was created
    pub created_at: u64,
    /// Total number of operations in this session
    pub operation_count: u64,
    /// Session nonce for replay protection
    pub nonce: u64,
}

/// Context for each operation within a session.
/// Enables full traceability of all contract interactions.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationContext {
    /// Session ID this operation belongs to
    pub session_id: u64,
    /// Sequential operation index within the session (0-based)
    pub operation_index: u64,
    /// Type of operation (e.g., "init", "register", "attest", "endpoint")
    pub operation_type: String,
    /// Timestamp of operation execution
    pub timestamp: u64,
    /// Result status: "success" or error code
    pub status: String,
    /// Optional result data (e.g., attestation ID, session ID)
    pub result_data: u64,
}

/// Full audit log entry for reproducibility
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditLog {
    /// Log entry ID (incremental)
    pub log_id: u64,
    /// Associated session ID
    pub session_id: u64,
    /// The operation context
    pub operation: OperationContext,
    /// Actor performing the operation
    pub actor: Address,
}
