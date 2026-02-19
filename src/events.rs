use soroban_sdk::{contracttype, Address, BytesN, Env, String};

/// Event emitted when an attestor is added.
/// Format: (Topic, SubjectAddr)
/// Topic: ("attestor", "added")
/// SubjectAddr: The attestor address
/// Minimized for gas efficiency - no data payload
pub struct AttestorAdded;

impl AttestorAdded {
    pub fn publish(env: &Env, attestor: &Address) {
        env.events().publish(
            (soroban_sdk::symbol_short!("attestor"), soroban_sdk::symbol_short!("added"), attestor),
            (),
        );
    }
}

/// Event emitted when an attestor is removed.
/// Format: (Topic, SubjectAddr)
/// Topic: ("attestor", "removed")
/// SubjectAddr: The attestor address
/// Minimized for gas efficiency - no data payload
pub struct AttestorRemoved;

impl AttestorRemoved {
    pub fn publish(env: &Env, attestor: &Address) {
        env.events().publish(
            (soroban_sdk::symbol_short!("attestor"), soroban_sdk::symbol_short!("removed"), attestor),
            (),
        );
    }
}

/// Event emitted when an attestation is recorded.
/// Format: (Topic, AttestationID, SubjectAddr, Data)
/// Topic: ("attest", "recorded")
/// AttestationID: The unique attestation ID
/// SubjectAddr: The subject address
/// Data: Minimal data containing only timestamp and payload_hash
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationRecordedData {
    pub timestamp: u64,
    pub payload_hash: BytesN<32>,
}

pub struct AttestationRecorded;

impl AttestationRecorded {
    pub fn publish(env: &Env, id: u64, subject: &Address, timestamp: u64, payload_hash: BytesN<32>) {
        env.events().publish(
            (soroban_sdk::symbol_short!("attest"), soroban_sdk::symbol_short!("recorded"), id, subject),
            AttestationRecordedData {
                timestamp,
                payload_hash,
            },
        );
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointConfigured {
    pub attestor: Address,
    pub url: String,
}

impl EndpointConfigured {
    pub fn publish(&self, env: &Env) {
        env.events().publish(
            (soroban_sdk::symbol_short!("endpoint"), soroban_sdk::symbol_short!("config")),
            self.clone(),
        );
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointRemoved {
    pub attestor: Address,
}

impl EndpointRemoved {
    pub fn publish(&self, env: &Env) {
        env.events().publish(
            (soroban_sdk::symbol_short!("endpoint"), soroban_sdk::symbol_short!("removed")),
            self.clone(),
        );
    }
}

/// Event emitted when a session is created.
/// Enables tracing of all operations within the session.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionCreated {
    pub session_id: u64,
    pub initiator: Address,
    pub timestamp: u64,
}

impl SessionCreated {
    pub fn publish(env: &Env, session_id: u64, initiator: &Address, timestamp: u64) {
        env.events().publish(
            (soroban_sdk::symbol_short!("session"), soroban_sdk::symbol_short!("created"), session_id),
            SessionCreated {
                session_id,
                initiator: initiator.clone(),
                timestamp,
            },
        );
    }
}

/// Event emitted when an operation is recorded in a session.
/// Provides full traceability of contract interactions.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationLogged {
    pub log_id: u64,
    pub session_id: u64,
    pub operation_index: u64,
    pub operation_type: String,
    pub status: String,
}

impl OperationLogged {
    pub fn publish(env: &Env, log_id: u64, session_id: u64, operation_index: u64, operation_type: &String, status: &String) {
        env.events().publish(
            (soroban_sdk::symbol_short!("audit"), soroban_sdk::symbol_short!("logged"), log_id),
            OperationLogged {
                log_id,
                session_id,
                operation_index,
                operation_type: operation_type.clone(),
                status: status.clone(),
            },
        );
    }
}
