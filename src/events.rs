use soroban_sdk::{contracttype, Address, Bytes, String};

#[contracttype]
#[derive(Clone)]
pub(crate) struct SessionCreatedEvent {
    pub session_id: u64,
    pub initiator: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub(crate) struct QuoteSubmitEvent {
    pub quote_id: u64,
    pub anchor: Address,
    pub base_asset: String,
    pub quote_asset: String,
    pub rate: u64,
    pub valid_until: u64,
}

#[contracttype]
#[derive(Clone)]
pub(crate) struct QuoteReceivedEvent {
    pub quote_id: u64,
    pub receiver: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub(crate) struct AuditLogEvent {
    pub log_id: u64,
    pub session_id: u64,
    pub operation_index: u64,
    pub operation_type: String,
    pub status: String,
}

#[contracttype]
#[derive(Clone)]
pub(crate) struct AttestEvent {
    pub payload_hash: Bytes,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct EndpointUpdated {
    pub attestor: Address,
    pub endpoint: String,
}

#[contracttype]
#[derive(Clone)]
pub(crate) struct AnchorDeactivated {
    pub anchor: Address,
    pub failure_count: u32,
    pub threshold: u32,
}
