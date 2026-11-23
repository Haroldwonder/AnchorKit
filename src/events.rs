use soroban_sdk::{contracttype, Address, Bytes, String, Symbol};

#[contracttype]
#[derive(Clone)]
pub struct SessionCreatedEvent {
    pub session_id: u64,
    pub initiator: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct QuoteSubmitEvent {
    pub quote_id: u64,
    pub anchor: Address,
    pub base_asset: String,
    pub quote_asset: String,
    pub rate: u64,
    pub valid_until: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct QuoteReceivedEvent {
    pub quote_id: u64,
    pub receiver: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct AuditLogEvent {
    pub log_id: u64,
    pub session_id: u64,
    pub operation_index: u64,
    pub operation_type: String,
    pub status: String,
}

#[contracttype]
#[derive(Clone)]
pub struct AttestEvent {
    pub subject: Address,
    pub payload_hash: Bytes,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct AuditLogPruned {
    pub pruned_count: u64,
    pub new_offset: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct EndpointUpdated {
    pub attestor: Address,
    pub endpoint: String,
}

#[contracttype]
#[derive(Clone)]
pub struct AnchorDeactivated {
    pub anchor: Address,
    pub failure_count: u32,
    pub threshold: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct RateLimitReset {
    pub attestor: Address,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct RateLimitWindowReset {
    pub attestor: Address,
    pub window_start: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct RoutingDecisionEvent {
    pub anchor: Address,
    pub strategy: String,
    pub quote_id: u64,
    pub ledger_sequence: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct QuoteExpiredEvent {
    pub anchor: Address,
    pub quote_id: u64,
    pub valid_until: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct AttestationRevoked {
    pub attestation_id: u64,
    pub issuer: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct AdminTransferred {
    pub old_admin: Address,
    pub new_admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct AdminTransferCancelled {
    pub admin: Address,
    pub cancelled_proposed: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct ContractPaused {
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct ContractUnpaused {
    pub admin: Address,
    pub timestamp: u64,
}

// ---------------------------------------------------------------------------
// #1361 – Admin governance events (previously silent)
// ---------------------------------------------------------------------------

/// Emitted by `set_health_failure_threshold`.
#[contracttype]
#[derive(Clone)]
pub struct HealthThresholdSet {
    pub threshold: u32,
    pub timestamp: u64,
}

/// Emitted by `update_health_status` on every update (the deactivation path
/// additionally emits `AnchorDeactivated`).
#[contracttype]
#[derive(Clone)]
pub struct HealthStatusUpdated {
    pub anchor: Address,
    pub latency_ms: u64,
    pub failure_count: u32,
    pub availability_percent: u32,
    pub timestamp: u64,
}

/// Emitted by `set_anchor_metadata`.
#[contracttype]
#[derive(Clone)]
pub struct AnchorMetadataSet {
    pub anchor: Address,
    pub reputation_score: u32,
    pub uptime_percentage: u32,
    pub is_active: bool,
    pub timestamp: u64,
}

/// Emitted by `set_anchor_jurisdiction`.
#[contracttype]
#[derive(Clone)]
pub struct AnchorJurisdictionSet {
    pub anchor: Address,
    /// `None` means the jurisdiction was cleared.
    pub jurisdiction: Option<String>,
    pub timestamp: u64,
}

/// Emitted by `cache_metadata`.
#[contracttype]
#[derive(Clone)]
pub struct MetadataCached {
    pub anchor: Address,
    pub ttl_seconds: u64,
    pub timestamp: u64,
}

/// Emitted by `cache_capabilities`.
#[contracttype]
#[derive(Clone)]
pub struct CapabilitiesCached {
    pub anchor: Address,
    pub ttl_seconds: u64,
    pub timestamp: u64,
}

/// Emitted by `upsert_sep10_verifying_key` and `add_sep10_verifying_key`.
#[contracttype]
#[derive(Clone)]
pub struct Sep10KeyAdded {
    pub issuer: Address,
    pub timestamp: u64,
}

/// Emitted by `remove_sep10_verifying_key`.
#[contracttype]
#[derive(Clone)]
pub struct Sep10KeyRemoved {
    pub issuer: Address,
    pub timestamp: u64,
}

/// Emitted by `set_credential_policy`.
#[contracttype]
#[derive(Clone)]
pub struct CredentialPolicySet {
    pub attestor: Address,
    pub rotation_interval_seconds: u64,
    pub require_encryption: bool,
    pub timestamp: u64,
}

/// Emitted by `store_encrypted_credential`.
#[contracttype]
#[derive(Clone)]
pub struct CredentialStored {
    pub attestor: Address,
    pub credential_type: Symbol,
    pub expires_at: u64,
    pub timestamp: u64,
}

/// Emitted by `rotate_credential`.
#[contracttype]
#[derive(Clone)]
pub struct CredentialRotated {
    pub attestor: Address,
    pub credential_type: Symbol,
    pub expires_at: u64,
    pub timestamp: u64,
}

/// Emitted by `update_rate_limit_config` (global config change).
#[contracttype]
#[derive(Clone)]
pub struct RateLimitConfigUpdated {
    pub timestamp: u64,
}

/// Emitted by `set_attestor_rate_limit` (per-attestor override).
#[contracttype]
#[derive(Clone)]
pub struct AttestorRateLimitSet {
    pub attestor: Address,
    pub timestamp: u64,
}
