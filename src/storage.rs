use soroban_sdk::{contracttype, Address, Bytes};

/// Typed storage keys for all contract state.
///
/// Using an enum prevents typos in raw string literals and makes every
/// storage access site self-documenting.
///
/// Note: the admin address is stored under the `key_admin(env)` Vec<Symbol>
/// key in instance storage (see the helper below). There is no `Admin` variant
/// here to avoid having two representations for the same logical key.
#[contracttype]
#[derive(Clone)]
pub enum StorageKey {
    /// SEP-10 JWT verifying key for an attestor (persistent).
    Sep10Key(Address),
    /// Whether an address is a registered attestor (persistent).
    Attestor(Address),
    /// Revocation marker for an attestor — present when the attestor has been
    /// revoked. Used by `get_attestation` to populate `issuer_revoked` without
    /// rewriting every stored attestation (persistent).
    AttestorRevoked(Address),
    /// HTTPS endpoint URL for an attestor (persistent).
    Endpoint(Address),
    /// Supported services record for an anchor (persistent).
    Services(Address),
    /// Replay-protection flag for a payload hash (persistent).
    Used(Bytes),
    /// Attestation record by ID (persistent).
    Attest(u64),
    /// Per-subject attestation count (persistent).
    SubjectCount(Address),
    /// Per-subject attestation index entry (persistent).
    SubjectAttestation(Address, u64),
    /// Tracing span keyed by request-ID bytes (temporary).
    Span(Bytes),
    /// Session record by session ID (persistent).
    Session(u64),
    /// Session operation count by session ID (persistent).
    SessionOpCount(u64),
    /// Audit log entry by log ID (persistent).
    AuditLog(u64),
    /// Maximum number of audit log entries to retain (instance storage).
    AuditLogMaxSize,
    /// Maximum page size allowed when listing attestations (instance storage).
    MaxPageSize,
    /// Quote record keyed by anchor + quote ID (persistent).
    Quote(Address, u64),
    /// Latest quote ID for an anchor (persistent).
    LatestQuote(Address),
    /// Metadata cache for an anchor (temporary).
    MetadataCache(Address),
    /// Capabilities cache for an anchor (temporary).
    CapabilitiesCache(Address),
    /// Health status for an anchor (persistent).
    Health(Address),
    /// Routing metadata for an anchor (persistent).
    AnchorMeta(Address),
    /// Stellar.toml cache for an anchor (temporary).
    TomlCache(Address),
    /// Per-attestor rate-limit state — submission count + window start (persistent).
    RateLimitState(Address),
    /// Per-attestor rate-limit configuration override (persistent).
    RateLimitOverride(Address),
    // --- Instance-storage counters (stored as Vec<Symbol> keys) ---
    // These are kept as plain symbol_short! vecs because instance storage
    // requires a Vec<Symbol> key; they are defined as named constants below.
}

// Instance-storage counter keys (Vec<Symbol>).
// Defined as functions returning the canonical key to avoid repetition.
use soroban_sdk::{symbol_short, Env, Symbol, Vec};

pub fn key_admin(env: &Env) -> Vec<Symbol> {
    soroban_sdk::vec![env, symbol_short!("ADMIN")]
}
pub fn key_counter(env: &Env) -> Vec<Symbol> {
    soroban_sdk::vec![env, symbol_short!("COUNTER")]
}
pub fn key_session_counter(env: &Env) -> Vec<Symbol> {
    soroban_sdk::vec![env, symbol_short!("SCNT")]
}
pub fn key_quote_counter(env: &Env) -> Vec<Symbol> {
    soroban_sdk::vec![env, symbol_short!("QCNT")]
}
pub fn key_audit_counter(env: &Env) -> Vec<Symbol> {
    soroban_sdk::vec![env, symbol_short!("ACNT")]
}
pub fn key_audit_log_offset(env: &Env) -> Vec<Symbol> {
    soroban_sdk::vec![env, symbol_short!("AOFF")]
}
pub fn key_anchor_list(env: &Env) -> Vec<Symbol> {
    soroban_sdk::vec![env, symbol_short!("ANCHLIST")]
}
pub fn key_health_threshold(env: &Env) -> Vec<Symbol> {
    soroban_sdk::vec![env, symbol_short!("HTHRESH")]
}
pub fn key_replay_window(env: &Env) -> Vec<Symbol> {
    soroban_sdk::vec![env, symbol_short!("RPWINDOW")]
}
