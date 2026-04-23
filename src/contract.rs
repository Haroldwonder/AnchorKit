use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short, Address, Bytes, BytesN,
    Env, String, Symbol, Vec,
};

use crate::deterministic_hash::{compute_payload_hash, verify_payload_hash};
use crate::errors::ErrorCode;
use crate::sep10_jwt;
use crate::storage::{
    StorageKey,
    key_admin, key_counter, key_session_counter, key_quote_counter,
    key_audit_counter, key_anchor_list, key_health_threshold,
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
pub struct Session {
    pub session_id: u64,
    pub initiator: Address,
    pub created_at: u64,
    pub nonce: u64,
    pub operation_count: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct Quote {
    pub quote_id: u64,
    pub anchor: Address,
    pub base_asset: String,
    pub quote_asset: String,
    pub rate: u64,
    pub fee_percentage: u32,
    pub minimum_amount: u64,
    pub maximum_amount: u64,
    pub valid_until: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct OperationContext {
    pub session_id: u64,
    pub operation_index: u64,
    pub operation_type: String,
    pub timestamp: u64,
    pub status: String,
    pub result_data: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct AuditLog {
    pub log_id: u64,
    pub session_id: u64,
    pub actor: Address,
    pub operation: OperationContext,
}

#[contracttype]
#[derive(Clone)]
pub struct RequestId {
    pub id: Bytes,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct Attestation {
    pub id: u64,
    pub issuer: Address,
    pub subject: Address,
    pub timestamp: u64,
    pub payload_hash: Bytes,
    pub signature: Bytes,
}

#[contracttype]
#[derive(Clone)]
pub struct TracingSpan {
    pub request_id: RequestId,
    pub operation: String,
    pub actor: Address,
    pub started_at: u64,
    pub completed_at: u64,
    pub status: String,
}

#[contracttype]
#[derive(Clone)]
pub struct AnchorServices {
    pub anchor: Address,
    pub services: Vec<u32>,
}

pub const SERVICE_DEPOSITS: u32 = 1;
pub const SERVICE_WITHDRAWALS: u32 = 2;
pub const SERVICE_QUOTES: u32 = 3;
pub const SERVICE_KYC: u32 = 4;

/// Typed representation of a service capability an anchor can support.
///
/// Each variant maps to a stable `u32` discriminant stored on-chain.
/// Use [`ServiceType::as_u32`] to convert before passing to contract functions.
#[derive(Clone, PartialEq)]
pub enum ServiceType {
    Deposits,
    Withdrawals,
    Quotes,
    KYC,
}

impl ServiceType {
    pub fn as_u32(&self) -> u32 {
        match self {
            ServiceType::Deposits => SERVICE_DEPOSITS,
            ServiceType::Withdrawals => SERVICE_WITHDRAWALS,
            ServiceType::Quotes => SERVICE_QUOTES,
            ServiceType::KYC => SERVICE_KYC,
        }
    }
}

// ---------------------------------------------------------------------------
// Routing types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
pub struct RoutingAnchorMeta {
    pub anchor: Address,
    pub reputation_score: u32,
    pub average_settlement_time: u64,
    pub liquidity_score: u32,
    pub uptime_percentage: u32,
    pub total_volume: u64,
    pub is_active: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct RoutingRequest {
    pub base_asset: String,
    pub quote_asset: String,
    pub amount: u64,
    pub operation_type: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct RoutingOptions {
    pub request: RoutingRequest,
    pub strategy: Vec<Symbol>,
    pub min_reputation: u32,
    pub max_anchors: u32,
    pub require_kyc: bool,
}

// ---------------------------------------------------------------------------
// Metadata cache types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
pub struct AnchorMetadata {
    pub anchor: Address,
    pub reputation_score: u32,
    pub liquidity_score: u32,
    pub uptime_percentage: u32,
    pub total_volume: u64,
    pub average_settlement_time: u64,
    pub is_active: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct MetadataCache {
    pub metadata: AnchorMetadata,
    pub cached_at: u64,
    pub ttl_seconds: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct CapabilitiesCache {
    pub toml_url: String,
    pub capabilities: String,
    pub cached_at: u64,
    pub ttl_seconds: u64,
}

// ---------------------------------------------------------------------------
// Anchor Info Discovery types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
pub struct AssetInfo {
    pub code: String,
    pub issuer: String,
    pub deposit_enabled: bool,
    pub withdrawal_enabled: bool,
    pub deposit_fee_fixed: u64,
    pub deposit_fee_percent: u32,
    pub withdrawal_fee_fixed: u64,
    pub withdrawal_fee_percent: u32,
    pub deposit_min_amount: u64,
    pub deposit_max_amount: u64,
    pub withdrawal_min_amount: u64,
    pub withdrawal_max_amount: u64,
}

/// Represents a fiat currency supported by an anchor (e.g. USD, EUR).
/// These are not Stellar assets and have no on-chain issuer.
#[contracttype]
#[derive(Clone)]
pub struct FiatCurrency {
    /// ISO 4217 currency code, e.g. "USD", "EUR".
    pub code: String,
    /// Human-readable name, e.g. "US Dollar".
    pub name: String,
    pub deposit_enabled: bool,
    pub withdrawal_enabled: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct StellarToml {
    pub version: String,
    pub network_passphrase: String,
    pub accounts: Vec<String>,
    /// The SIGNING_KEY from stellar.toml, used for SEP-10 verification.
    /// `None` when the anchor does not publish a signing key.
    pub signing_key: Option<String>,
    pub currencies: Vec<AssetInfo>,
    /// Fiat currencies supported by this anchor (USD, EUR, etc.).
    pub fiat_currencies: Vec<FiatCurrency>,
    pub transfer_server: String,
    pub transfer_server_sep0024: String,
    pub kyc_server: String,
    pub web_auth_endpoint: String,
}

#[contracttype]
#[derive(Clone)]
pub struct CachedToml {
    pub toml: StellarToml,
    pub cached_at: u64,
    pub ttl_seconds: u64,
}

const MIN_TEMP_TTL: u32 = 15; // min_temp_entry_ttl - 1

// ---------------------------------------------------------------------------
// Event structs
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
struct SessionCreatedEvent {
    session_id: u64,
    initiator: Address,
    timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
struct QuoteSubmitEvent {
    quote_id: u64,
    anchor: Address,
    base_asset: String,
    quote_asset: String,
    rate: u64,
    valid_until: u64,
}

#[contracttype]
#[derive(Clone)]
struct QuoteReceivedEvent {
    quote_id: u64,
    receiver: Address,
    timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
struct AuditLogEvent {
    log_id: u64,
    session_id: u64,
    operation_index: u64,
    operation_type: String,
    status: String,
}

#[contracttype]
#[derive(Clone)]
struct AttestEvent {
    payload_hash: Bytes,
    timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct EndpointUpdated {
    pub attestor: Address,
    pub endpoint: String,
}

#[contracttype]
#[derive(Clone)]
pub struct HealthStatus {
    pub anchor: Address,
    pub latency_ms: u64,
    pub failure_count: u32,
    pub availability_percent: u32,
}

#[contracttype]
#[derive(Clone)]
struct AnchorDeactivated {
    anchor: Address,
    failure_count: u32,
    threshold: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct AttestorRegistered(pub Address);

#[contracttype]
#[derive(Clone)]
pub struct AttestorRevoked(pub Address);

// ---------------------------------------------------------------------------
// TTLs (in ledgers)
// ---------------------------------------------------------------------------
const PERSISTENT_TTL: u32 = 1_555_200;
const SPAN_TTL: u32 = 17_280;
const INSTANCE_TTL: u32 = 518_400;
const MIN_TEMP_TTL: u32 = 15;

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct AnchorKitContract;

#[contractimpl]
#[allow(clippy::too_many_arguments)]
impl AnchorKitContract {
    pub fn get_attestation_count(env: Env) -> u64 {
        env.storage().instance().get(&symbol_short!("TOTALCNT")).unwrap_or(0)
    }
    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        if admin == env.current_contract_address() {
            panic_with_error!(&env, ErrorCode::ValidationError);
        }
        let inst = env.storage().instance();
        if inst.has(&key_admin(&env)) {
            panic_with_error!(&env, ErrorCode::AlreadyInitialized);
        }
        inst.set(&key_admin(&env), &admin);
        inst.extend_ttl(INSTANCE_TTL, INSTANCE_TTL);
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get::<_, Address>(&key_admin(&env))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::NotInitialized))
    }

    /// Returns `true` if the contract has been initialized, `false` otherwise.
    /// Safe to call at any time — never panics.
    pub fn is_initialized(env: Env) -> bool {
        env.storage().instance().has(&admin_key(&env))
    }

    // -----------------------------------------------------------------------
    // Request ID generation
    // -----------------------------------------------------------------------

    pub fn generate_request_id(env: Env) -> RequestId {
        let ts = env.ledger().timestamp();
        let seq = env.ledger().sequence();

        let mut input = Bytes::new(&env);
        for b in ts.to_be_bytes().iter() {
            input.push_back(*b);
        }
        for b in seq.to_be_bytes().iter() {
            input.push_back(*b);
        }

        let hash = env.crypto().sha256(&input);
        let hash_bytes = Bytes::from_array(&env, &hash.into());
        let mut id = Bytes::new(&env);
        for i in 0..16u32 {
            id.push_back(hash_bytes.get(i).unwrap());
        }

        RequestId { id, created_at: ts }
    }

    // -----------------------------------------------------------------------
    // Attestor management
    // -----------------------------------------------------------------------

    pub fn set_sep10_jwt_verifying_key(env: Env, issuer: Address, public_key: Bytes) {
        Self::require_admin(&env);
        if public_key.len() != 32 {
            panic_with_error!(&env, ErrorCode::ValidationError);
        }
        let mut keys: Vec<Bytes> = Vec::new(&env);
        keys.push_back(public_key);
        let storage_key = (symbol_short!("SEP10KEY"), issuer.clone());
        env.storage().persistent().set(&storage_key, &keys);
        env.storage()
            .persistent()
            .extend_ttl(&storage_key, PERSISTENT_TTL, PERSISTENT_TTL);
    }

    pub fn add_sep10_verifying_key(env: Env, issuer: Address, public_key: Bytes) {
        Self::require_admin(&env);
        if public_key.len() != 32 {
            panic_with_error!(&env, ErrorCode::ValidationError);
        }
        let storage_key = (symbol_short!("SEP10KEY"), issuer.clone());
        let mut keys: Vec<Bytes> = env
            .storage()
            .persistent()
            .get(&storage_key)
            .unwrap_or_else(|| Vec::new(&env));
        if keys.len() >= sep10_jwt::MAX_VERIFYING_KEYS {
            panic_with_error!(&env, ErrorCode::ValidationError);
        }
        keys.push_back(public_key);
        env.storage().persistent().set(&storage_key, &keys);
        env.storage()
            .persistent()
            .extend_ttl(&storage_key, PERSISTENT_TTL, PERSISTENT_TTL);
    }

    pub fn remove_sep10_verifying_key(env: Env, issuer: Address, public_key: Bytes) {
        Self::require_admin(&env);
        let storage_key = (symbol_short!("SEP10KEY"), issuer.clone());
        let keys: Vec<Bytes> = env
            .storage()
            .persistent()
            .get(&storage_key)
            .unwrap_or_else(|| Vec::new(&env));
        let mut new_keys: Vec<Bytes> = Vec::new(&env);
        for i in 0..keys.len() {
            let k = keys.get(i).unwrap();
            if k != public_key {
                new_keys.push_back(k);
            }
        }
        env.storage().persistent().set(&storage_key, &new_keys);
        env.storage()
            .persistent()
            .extend_ttl(&storage_key, PERSISTENT_TTL, PERSISTENT_TTL);
    }

    pub fn verify_sep10_token(env: Env, token: String, issuer: Address) {
        let keys: Vec<Bytes> = env
            .storage()
            .persistent()
            .get(&StorageKey::Sep10Key(issuer.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::InvalidSep10Token));
        if sep10_jwt::verify_sep10_jwt(&env, &token, &keys, None).is_err() {
            panic_with_error!(&env, ErrorCode::InvalidSep10Token);
        }
    }

    fn verify_sep10_token_matches_attestor(
        env: &Env,
        token: &String,
        issuer: &Address,
        attestor: &Address,
    ) {
        let keys: Vec<Bytes> = env
            .storage()
            .persistent()
            .get(&StorageKey::Sep10Key(issuer.clone()))
            .unwrap_or_else(|| panic_with_error!(env, ErrorCode::InvalidSep10Token));
        let expected = attestor.to_string();
        if sep10_jwt::verify_sep10_jwt(env, token, &keys, Some(&expected)).is_err() {
            panic_with_error!(env, ErrorCode::InvalidSep10Token);
        }
    }

    pub fn register_attestor(env: Env, attestor: Address, sep10_token: String, sep10_issuer: Address) {
        Self::require_admin(&env);
        Self::verify_sep10_token_matches_attestor(&env, &sep10_token, &sep10_issuer, &attestor);
        let key = StorageKey::Attestor(attestor.clone());
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, ErrorCode::AttestorAlreadyRegistered);
        }
        env.storage().persistent().set(&key, &true);
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_TTL, PERSISTENT_TTL);
        env.events().publish(
(symbol_short!("attestor"), symbol_short!("reg")),
            AttestorRegistered(attestor),
        );
    }

    pub fn revoke_attestor(env: Env, attestor: Address) {
        Self::require_admin(&env);
        let key = StorageKey::Attestor(attestor.clone());
        if !env.storage().persistent().has(&key) {
            panic_with_error!(&env, ErrorCode::AttestorNotRegistered);
        }
        env.storage().persistent().remove(&key);
        env.events().publish(
            (symbol_short!("attestor"), symbol_short!("revoked")),
            AttestorRevoked(attestor),
        );
    }

    pub fn is_attestor(env: Env, attestor: Address) -> bool {
        env.storage()
            .persistent()
            .get::<_, bool>(&StorageKey::Attestor(attestor))
            .unwrap_or(false)
    }

    // -----------------------------------------------------------------------
    // Attestor endpoint management
    // -----------------------------------------------------------------------

    pub fn set_endpoint(env: Env, attestor: Address, endpoint: String) {
        attestor.require_auth();
        Self::check_attestor(&env, &attestor);

        let len = endpoint.len() as usize;
        let mut rust_buf = [0u8; 128];
        if len > 128 {
            panic_with_error!(&env, ErrorCode::InvalidEndpointFormat);
        }
        endpoint.copy_into_slice(&mut rust_buf[..len]);
        let endpoint_str = core::str::from_utf8(&rust_buf[..len]).unwrap_or("");

        if crate::validate_anchor_domain(endpoint_str).is_err() {
            panic_with_error!(&env, ErrorCode::InvalidEndpointFormat);
        }

        let key = StorageKey::Endpoint(attestor.clone());
        env.storage().persistent().set(&key, &endpoint);
        env.storage().persistent().extend_ttl(&key, PERSISTENT_TTL, PERSISTENT_TTL);
        env.events().publish(
            (symbol_short!("endpoint"), symbol_short!("updated")),
            EndpointUpdated { attestor, endpoint },
        );
    }

    pub fn get_endpoint(env: Env, attestor: Address) -> String {
        if !Self::is_attestor(env.clone(), attestor.clone()) {
            panic_with_error!(&env, ErrorCode::AttestorNotRegistered);
        }
        env.storage().persistent()
            .get::<_, String>(&StorageKey::Endpoint(attestor))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::AttestorNotRegistered))
    }

    // -----------------------------------------------------------------------
    // Service configuration
    // -----------------------------------------------------------------------

    pub fn configure_services(env: Env, anchor: Address, services: Vec<u32>) {
        anchor.require_auth();
        if !env
            .storage()
            .persistent()
            .has(&StorageKey::Attestor(anchor.clone()))
        {
            panic_with_error!(&env, ErrorCode::AttestorNotRegistered);
        }
        if services.is_empty() {
            panic_with_error!(&env, ErrorCode::InvalidServiceType);
        }
        let mut seen = Vec::new(&env);
        for s in services.iter() {
            if seen.contains(s) {
                panic_with_error!(&env, ErrorCode::InvalidServiceType);
            }
            seen.push_back(s);
        }
        let record = AnchorServices {
            anchor: anchor.clone(),
            services: services.clone(),
        };
        let key = StorageKey::Services(anchor.clone());
        env.storage().persistent().set(&key, &record);
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_TTL, PERSISTENT_TTL);
        env.events()
            .publish((symbol_short!("services"), symbol_short!("config")), record);
    }

    pub fn get_supported_services(env: Env, anchor: Address) -> AnchorServices {
        env.storage()
            .persistent()
            .get::<_, AnchorServices>(&StorageKey::Services(anchor))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::ServicesNotConfigured))
    }

    pub fn supports_service(env: Env, anchor: Address, service: u32) -> bool {
        let record = env
            .storage()
            .persistent()
            .get::<_, AnchorServices>(&StorageKey::Services(anchor))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::ServicesNotConfigured));
        record.services.contains(service)
    }

    // -----------------------------------------------------------------------
    // Attestation submission (plain)
    // -----------------------------------------------------------------------

    pub fn submit_attestation(
        env: Env,
        issuer: Address,
        subject: Address,
        timestamp: u64,
        payload_hash: Bytes,
        signature: Bytes,
    ) -> u64 {
        issuer.require_auth();
        Self::check_attestor(&env, &issuer);
        Self::check_timestamp(&env, timestamp);

        let used_key = StorageKey::Used(payload_hash.clone());
        if env.storage().persistent().has(&used_key) {
            panic_with_error!(&env, ErrorCode::ReplayAttack);
        }

        let id = Self::next_attestation_id(&env);
        Self::store_attestation(&env, id, issuer.clone(), subject.clone(), timestamp, payload_hash.clone(), signature);

        env.storage().persistent().set(&used_key, &true);
        env.storage().persistent().extend_ttl(&used_key, PERSISTENT_TTL, PERSISTENT_TTL);

        env.events().publish(
            (symbol_short!("attest"), symbol_short!("recorded"), id, subject),
            AttestEvent { payload_hash, timestamp },
        );

        id
    }

    // -----------------------------------------------------------------------
    // Attestation submission with request ID + tracing span
    // -----------------------------------------------------------------------

    pub fn submit_with_request_id(
        env: Env,
        request_id: RequestId,
        issuer: Address,
        subject: Address,
        timestamp: u64,
        payload_hash: Bytes,
        signature: Bytes,
    ) -> u64 {
        issuer.require_auth();
        Self::check_attestor(&env, &issuer);
        Self::check_timestamp(&env, timestamp);

        let used_key = StorageKey::Used(payload_hash.clone());
        if env.storage().persistent().has(&used_key) {
            panic_with_error!(&env, ErrorCode::ReplayAttack);
        }

        let id = Self::next_attestation_id(&env);
        Self::store_attestation(&env, id, issuer.clone(), subject.clone(), timestamp, payload_hash.clone(), signature);

        env.storage().persistent().set(&used_key, &true);
        env.storage().persistent().extend_ttl(&used_key, PERSISTENT_TTL, PERSISTENT_TTL);

        let now = env.ledger().timestamp();
        Self::store_span(&env, &request_id, String::from_str(&env, "submit_attestation"), issuer.clone(), now, String::from_str(&env, "success"));

        env.events().publish(
            (symbol_short!("attest"), symbol_short!("recorded"), id, subject),
            AttestEvent { payload_hash, timestamp },
        );

        id
    }

    // -----------------------------------------------------------------------
    // Quote submission with request ID + tracing span
    // -----------------------------------------------------------------------

    #[allow(unused_variables)]
    #[allow(clippy::too_many_arguments)]
    pub fn quote_with_request_id(
        env: Env,
        request_id: RequestId,
        anchor: Address,
        from_asset: String,
        to_asset: String,
        amount: u64,
        fee_bps: u32,
        min_amount: u64,
        max_amount: u64,
        expires_at: u64,
    ) {
        anchor.require_auth();

        let services_record = env
            .storage()
            .persistent()
            .get::<_, AnchorServices>(&StorageKey::Services(anchor.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::ServicesNotConfigured));
        if !services_record.services.contains(SERVICE_QUOTES) {
            panic_with_error!(&env, ErrorCode::ServicesNotConfigured);
        }

        let now = env.ledger().timestamp();
        Self::store_span(&env, &request_id, String::from_str(&env, "submit_quote"), anchor, now, String::from_str(&env, "success"));
    }

    // -----------------------------------------------------------------------
    // Tracing span retrieval
    // -----------------------------------------------------------------------

    pub fn get_tracing_span(env: Env, request_id_bytes: Bytes) -> Option<TracingSpan> {
        env.storage()
            .temporary()
            .get::<_, TracingSpan>(&StorageKey::Span(request_id_bytes))
    }

    // -----------------------------------------------------------------------
    // Attestation retrieval
    // -----------------------------------------------------------------------

    pub fn get_attestation(env: Env, id: u64) -> Option<Attestation> {
        env.storage()
            .persistent()
            .get::<_, Attestation>(&(symbol_short!("ATTEST"), id))
    }

    pub fn list_attestations(env: Env, subject: Address, offset: u64, limit: u32) -> Vec<Attestation> {
        let actual_limit = if limit > 50 { 50 } else { limit };
        let mut results = Vec::new(&env);

        let count_key = StorageKey::SubjectCount(subject.clone());
        let total_count: u64 = env.storage().persistent().get(&count_key).unwrap_or(0);

        if offset >= total_count || actual_limit == 0 {
            return results;
        }

        let end = if offset + (actual_limit as u64) > total_count {
            total_count
        } else {
            offset + (actual_limit as u64)
        };

        for i in offset..end {
            let index_key = StorageKey::SubjectAttestation(subject.clone(), i);
            if let Some(attestation_id) = env.storage().persistent().get::<_, u64>(&index_key) {
                let main_key = StorageKey::Attest(attestation_id);
                if let Some(attestation) = env.storage().persistent().get::<_, Attestation>(&main_key) {
                    results.push_back(attestation);
                }
            }
        }

        results
    }

    // -----------------------------------------------------------------------
    // Deterministic hash utilities
    // -----------------------------------------------------------------------

    pub fn compute_payload_hash(env: Env, subject: Address, timestamp: u64, data: Bytes) -> BytesN<32> {
        compute_payload_hash(&env, &subject, timestamp, &data)
    }

    pub fn verify_payload_hash(env: Env, attestation_id: u64, expected_hash: BytesN<32>) -> bool {
        let attestation = env
            .storage()
            .persistent()
            .get::<_, Attestation>(&StorageKey::Attest(attestation_id))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::AttestationNotFound));

        let stored: BytesN<32> = attestation.payload_hash.try_into().unwrap_or_else(|_| {
            panic_with_error!(&env, ErrorCode::StorageCorrupted)
        });
        verify_payload_hash(&stored, &expected_hash)
    }

    // -----------------------------------------------------------------------
    // Session management
    // -----------------------------------------------------------------------

    pub fn create_session(env: Env, initiator: Address) -> u64 {
        initiator.require_auth();
        let inst = env.storage().instance();
        let scnt_key = key_session_counter(&env);
        let session_id: u64 = inst.get(&scnt_key).unwrap_or(0u64);
        inst.set(&scnt_key, &(session_id + 1));
        inst.extend_ttl(INSTANCE_TTL, INSTANCE_TTL);

        let now = env.ledger().timestamp();
        let session = Session {
            session_id,
            initiator: initiator.clone(),
            created_at: now,
            nonce: 0,
            operation_count: 0,
        };
        let sess_key = StorageKey::Session(session_id);
        env.storage().persistent().set(&sess_key, &session);
        env.storage().persistent().extend_ttl(&sess_key, PERSISTENT_TTL, PERSISTENT_TTL);

        let snonce_key = StorageKey::SessionNonce(session_id);
        env.storage().persistent().set(&snonce_key, &0u64);
        env.storage().persistent().extend_ttl(&snonce_key, PERSISTENT_TTL, PERSISTENT_TTL);

        env.events().publish(
            (symbol_short!("session"), symbol_short!("created"), session_id),
            SessionCreatedEvent { session_id, initiator, timestamp: now },
        );

        session_id
    }

    pub fn get_session(env: Env, session_id: u64) -> Session {
        env.storage()
            .persistent()
            .get::<_, Session>(&(symbol_short!("SESS"), session_id))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::AttestationNotFound))
    }

    pub fn get_audit_log(env: Env, log_id: u64) -> AuditLog {
        env.storage()
            .persistent()
            .get::<_, AuditLog>(&(symbol_short!("AUDIT"), log_id))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::AttestationNotFound))
    }

    pub fn get_session_operation_count(env: Env, session_id: u64) -> u64 {
        env.storage()
            .persistent()
            .get::<_, u64>(&(symbol_short!("SOPCNT"), session_id))
            .unwrap_or(0)
    }

    // -----------------------------------------------------------------------
    // Quote management
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn submit_quote(
        env: Env,
        anchor: Address,
        base_asset: String,
        quote_asset: String,
        rate: u64,
        fee_percentage: u32,
        minimum_amount: u64,
        maximum_amount: u64,
        valid_until: u64,
    ) -> u64 {
        anchor.require_auth();
        Self::check_attestor(&env, &anchor);
        let inst = env.storage().instance();
        let qcnt_key = key_quote_counter(&env);
        let next: u64 = inst.get(&qcnt_key).unwrap_or(0u64) + 1;
        inst.set(&qcnt_key, &next);
        inst.extend_ttl(INSTANCE_TTL, INSTANCE_TTL);

        let quote = Quote {
            quote_id: next,
            anchor: anchor.clone(),
            base_asset: base_asset.clone(),
            quote_asset: quote_asset.clone(),
            rate,
            fee_percentage,
            minimum_amount,
            maximum_amount,
            valid_until,
        };
        let q_key = StorageKey::Quote(anchor.clone(), next);
        env.storage().persistent().set(&q_key, &quote);
        env.storage().persistent().extend_ttl(&q_key, PERSISTENT_TTL, PERSISTENT_TTL);

        let lq_key = StorageKey::LatestQuote(anchor.clone());
        env.storage().persistent().set(&lq_key, &next);
        env.storage().persistent().extend_ttl(&lq_key, PERSISTENT_TTL, PERSISTENT_TTL);

        env.events().publish(
            (symbol_short!("quote"), symbol_short!("submit"), next),
            QuoteSubmitEvent { quote_id: next, anchor, base_asset, quote_asset, rate, valid_until },
        );

        next
    }

    pub fn receive_quote(env: Env, receiver: Address, anchor: Address, quote_id: u64) -> Quote {
        receiver.require_auth();
        let q_key = StorageKey::Quote(anchor.clone(), quote_id);
        let quote: Quote = env.storage().persistent().get(&q_key)
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::AttestationNotFound));

        env.events().publish(
            (symbol_short!("quote"), symbol_short!("received"), quote_id),
            QuoteReceivedEvent { quote_id, receiver, timestamp: env.ledger().timestamp() },
        );

        quote
    }

    // -----------------------------------------------------------------------
    // Session-aware attestation
    // -----------------------------------------------------------------------

    pub fn submit_attestation_with_session(
        env: Env,
        session_id: u64,
        issuer: Address,
        subject: Address,
        timestamp: u64,
        payload_hash: Bytes,
        signature: Bytes,
    ) -> u64 {
        issuer.require_auth();
        Self::check_attestor(&env, &issuer);
        Self::check_timestamp(&env, timestamp);

        let used_key = StorageKey::Used(payload_hash.clone());
        if env.storage().persistent().has(&used_key) {
            panic_with_error!(&env, ErrorCode::ReplayAttack);
        }

        let id = Self::next_attestation_id(&env);
        Self::store_attestation(&env, id, issuer.clone(), subject.clone(), timestamp, payload_hash.clone(), signature);

        env.storage().persistent().set(&used_key, &true);
        env.storage().persistent().extend_ttl(&used_key, PERSISTENT_TTL, PERSISTENT_TTL);

        // Get and increment session operation count
        let sopcnt_key = StorageKey::SessionOpCount(session_id);
        let op_index: u64 = env.storage().persistent().get(&sopcnt_key).unwrap_or(0u64);
        env.storage().persistent().set(&sopcnt_key, &(op_index + 1));
        env.storage().persistent().extend_ttl(&sopcnt_key, PERSISTENT_TTL, PERSISTENT_TTL);

        let inst = env.storage().instance();
        let acnt_key = key_audit_counter(&env);
        let log_id: u64 = inst.get(&acnt_key).unwrap_or(0u64);
        inst.set(&acnt_key, &(log_id + 1));
        inst.extend_ttl(INSTANCE_TTL, INSTANCE_TTL);

        let now = env.ledger().timestamp();
        let audit = AuditLog {
            log_id,
            session_id,
            actor: issuer.clone(),
            operation: OperationContext {
                session_id,
                operation_index: op_index,
                operation_type: String::from_str(&env, "attest"),
                timestamp: now,
                status: String::from_str(&env, "success"),
                result_data: id,
            },
        };
        let audit_key = StorageKey::AuditLog(log_id);
        env.storage().persistent().set(&audit_key, &audit);
        env.storage().persistent().extend_ttl(&audit_key, PERSISTENT_TTL, PERSISTENT_TTL);

        env.events().publish(
            (symbol_short!("attest"), symbol_short!("recorded"), id, subject),
            AttestEvent { payload_hash, timestamp },
        );
        env.events().publish(
            (symbol_short!("audit"), symbol_short!("logged"), log_id),
            AuditLogEvent {
                log_id,
                session_id,
                operation_index: op_index,
                operation_type: String::from_str(&env, "attest"),
                status: String::from_str(&env, "success"),
            },
        );

        id
    }

    pub fn register_attestor_with_session(env: Env, session_id: u64, attestor: Address) {
        Self::require_admin(&env);
        let key = StorageKey::Attestor(attestor.clone());
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, ErrorCode::AttestorAlreadyRegistered);
        }
        env.storage().persistent().set(&key, &true);
        env.storage().persistent().extend_ttl(&key, PERSISTENT_TTL, PERSISTENT_TTL);

        let sopcnt_key = StorageKey::SessionOpCount(session_id);
        let op_index: u64 = env.storage().persistent().get(&sopcnt_key).unwrap_or(0u64);
        env.storage().persistent().set(&sopcnt_key, &(op_index + 1));
        env.storage().persistent().extend_ttl(&sopcnt_key, PERSISTENT_TTL, PERSISTENT_TTL);

        let inst = env.storage().instance();
        let acnt_key = key_audit_counter(&env);
        let log_id: u64 = inst.get(&acnt_key).unwrap_or(0u64);
        inst.set(&acnt_key, &(log_id + 1));
        inst.extend_ttl(INSTANCE_TTL, INSTANCE_TTL);

        let admin: Address = inst
            .get::<_, Address>(&key_admin(&env))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::NotInitialized));
        let now = env.ledger().timestamp();
        let audit = AuditLog {
            log_id,
            session_id,
            actor: admin,
            operation: OperationContext {
                session_id,
                operation_index: op_index,
                operation_type: String::from_str(&env, "register"),
                timestamp: now,
                status: String::from_str(&env, "success"),
                result_data: 0,
            },
        };
        let audit_key = StorageKey::AuditLog(log_id);
        env.storage().persistent().set(&audit_key, &audit);
        env.storage().persistent().extend_ttl(&audit_key, PERSISTENT_TTL, PERSISTENT_TTL);

        env.events().publish((symbol_short!("attestor"), symbol_short!("added"), attestor), ());
        env.events().publish(
            (symbol_short!("audit"), symbol_short!("logged"), log_id),
            AuditLogEvent {
                log_id,
                session_id,
                operation_index: op_index,
                operation_type: String::from_str(&env, "register"),
                status: String::from_str(&env, "success"),
            },
        );
    }

    pub fn revoke_attestor_with_session(env: Env, session_id: u64, attestor: Address) {
        Self::require_admin(&env);
        let key = StorageKey::Attestor(attestor.clone());
        if !env.storage().persistent().has(&key) {
            panic_with_error!(&env, ErrorCode::AttestorNotRegistered);
        }
        env.storage().persistent().remove(&key);

        let sopcnt_key = StorageKey::SessionOpCount(session_id);
        let op_index: u64 = env.storage().persistent().get(&sopcnt_key).unwrap_or(0u64);
        env.storage().persistent().set(&sopcnt_key, &(op_index + 1));
        env.storage().persistent().extend_ttl(&sopcnt_key, PERSISTENT_TTL, PERSISTENT_TTL);

        let inst = env.storage().instance();
        let acnt_key = key_audit_counter(&env);
        let log_id: u64 = inst.get(&acnt_key).unwrap_or(0u64);
        inst.set(&acnt_key, &(log_id + 1));
        inst.extend_ttl(INSTANCE_TTL, INSTANCE_TTL);

        let admin: Address = inst
            .get::<_, Address>(&key_admin(&env))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::NotInitialized));
        let now = env.ledger().timestamp();
        let audit = AuditLog {
            log_id,
            session_id,
            actor: admin,
            operation: OperationContext {
                session_id,
                operation_index: op_index,
                operation_type: String::from_str(&env, "revoke"),
                timestamp: now,
                status: String::from_str(&env, "success"),
                result_data: 0,
            },
        };
        let audit_key = StorageKey::AuditLog(log_id);
        env.storage().persistent().set(&audit_key, &audit);
        env.storage().persistent().extend_ttl(&audit_key, PERSISTENT_TTL, PERSISTENT_TTL);

        env.events().publish((symbol_short!("attestor"), symbol_short!("removed"), attestor), ());
        env.events().publish(
            (symbol_short!("audit"), symbol_short!("logged"), log_id),
            AuditLogEvent {
                log_id,
                session_id,
                operation_index: op_index,
                operation_type: String::from_str(&env, "revoke"),
                status: String::from_str(&env, "success"),
            },
        );
    }

    pub fn get_session(env: Env, session_id: u64) -> Session {
        env.storage()
            .persistent()
            .get::<_, Session>(&StorageKey::Session(session_id))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::AttestationNotFound))
    }

    pub fn get_audit_log(env: Env, log_id: u64) -> AuditLog {
        env.storage()
            .persistent()
            .get::<_, AuditLog>(&StorageKey::AuditLog(log_id))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::AttestationNotFound))
    }

    /// Return audit log entries in [from_id, to_id], capped at 100 entries.
    /// IDs that have no stored entry are silently skipped.
    pub fn get_audit_log_range(env: Env, from_id: u64, to_id: u64) -> Vec<AuditLog> {
        let mut result = Vec::new(&env);
        if from_id > to_id {
            return result;
        }
        let cap: u64 = 100;
        let end = if to_id - from_id + 1 > cap { from_id + cap - 1 } else { to_id };
        let mut id = from_id;
        while id <= end {
            if let Some(log) = env.storage().persistent().get::<_, AuditLog>(&StorageKey::AuditLog(id)) {
                result.push_back(log);
            }
            id += 1;
        }
        result
    }

    pub fn get_session_operation_count(env: Env, session_id: u64) -> u64 {
        env.storage()
            .persistent()
            .get::<_, u64>(&StorageKey::SessionOpCount(session_id))
            .unwrap_or(0)
    }

    // -----------------------------------------------------------------------
    // Metadata cache
    // -----------------------------------------------------------------------

    pub fn cache_metadata(env: Env, anchor: Address, metadata: AnchorMetadata, ttl_seconds: u64) {
        Self::require_admin(&env);
        let now = env.ledger().timestamp();
        let entry = MetadataCache { metadata, cached_at: now, ttl_seconds };
        let key = (symbol_short!("METACACHE"), anchor.clone());
        let ledger_ttl = if ttl_seconds as u32 > MIN_TEMP_TTL { ttl_seconds as u32 } else { MIN_TEMP_TTL };
        env.storage().temporary().set(&key, &entry);
        env.storage().temporary().extend_ttl(&key, ledger_ttl, ledger_ttl);

        // Issue #276: maintain CACHED_ANCHORS set
        let list_key = soroban_sdk::vec![&env, symbol_short!("CANCHORS")];
        let mut list: Vec<Address> = env.storage().persistent()
            .get::<_, Vec<Address>>(&list_key)
            .unwrap_or_else(|| Vec::new(&env));
        if !list.contains(&anchor) {
            list.push_back(anchor);
            env.storage().persistent().set(&list_key, &list);
            env.storage().persistent().extend_ttl(&list_key, PERSISTENT_TTL, PERSISTENT_TTL);
        }
    }

    pub fn get_cached_metadata(env: Env, anchor: Address) -> AnchorMetadata {
        let key = StorageKey::MetadataCache(anchor);
        let entry: MetadataCache = env.storage().temporary().get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::CacheNotFound));
        let now = env.ledger().timestamp();
        if entry.cached_at + entry.ttl_seconds <= now {
            panic_with_error!(&env, ErrorCode::CacheExpired);
        }
        entry.metadata
    }

    pub fn refresh_metadata_cache(env: Env, anchor: Address) {
        Self::require_admin(&env);
        let key = (symbol_short!("METACACHE"), anchor.clone());
        env.storage().temporary().remove(&key);

        // Issue #276: remove from CACHED_ANCHORS set
        let list_key = soroban_sdk::vec![&env, symbol_short!("CANCHORS")];
        if let Some(list) = env.storage().persistent().get::<_, Vec<Address>>(&list_key) {
            let mut new_list = Vec::new(&env);
            for a in list.iter() {
                if a != anchor {
                    new_list.push_back(a);
                }
            }
            env.storage().persistent().set(&list_key, &new_list);
            env.storage().persistent().extend_ttl(&list_key, PERSISTENT_TTL, PERSISTENT_TTL);
        }
    }

    /// Issue #276: list all anchors that currently have active metadata cache entries.
    pub fn list_cached_anchors(env: Env) -> Vec<Address> {
        let list_key = soroban_sdk::vec![&env, symbol_short!("CANCHORS")];
        env.storage().persistent()
            .get::<_, Vec<Address>>(&list_key)
            .unwrap_or_else(|| Vec::new(&env))
    }

    // -----------------------------------------------------------------------
    // Capabilities cache
    // -----------------------------------------------------------------------

    pub fn cache_capabilities(env: Env, anchor: Address, toml_url: String, capabilities: String, ttl_seconds: u64) {
        Self::require_admin(&env);

        // Issue #280: Validate toml_url before caching
        let len = toml_url.len() as usize;
        let mut buf = [0u8; 256];
        if len > 256 {
            panic_with_error!(&env, ErrorCode::InvalidEndpointFormat);
        }
        toml_url.copy_into_slice(&mut buf[..len]);
        let url_str = core::str::from_utf8(&buf[..len]).unwrap_or("");
        if crate::domain_validator::validate_anchor_domain(url_str).is_err() {
            panic_with_error!(&env, ErrorCode::InvalidEndpointFormat);
        }

        let now = env.ledger().timestamp();
        let entry = CapabilitiesCache { toml_url, capabilities, cached_at: now, ttl_seconds };
        let key = StorageKey::CapabilitiesCache(anchor);
        let ledger_ttl = if ttl_seconds as u32 > MIN_TEMP_TTL { ttl_seconds as u32 } else { MIN_TEMP_TTL };
        env.storage().temporary().set(&key, &entry);
        env.storage().temporary().extend_ttl(&key, ledger_ttl, ledger_ttl);
    }

    pub fn get_cached_capabilities(env: Env, anchor: Address) -> CapabilitiesCache {
        let key = StorageKey::CapabilitiesCache(anchor);
        let entry: CapabilitiesCache = env.storage().temporary().get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::CacheNotFound));
        let now = env.ledger().timestamp();
        if entry.cached_at + entry.ttl_seconds <= now {
            panic_with_error!(&env, ErrorCode::CacheExpired);
        }
        entry
    }

    pub fn refresh_capabilities_cache(env: Env, anchor: Address) {
        Self::require_admin(&env);
        let key = StorageKey::CapabilitiesCache(anchor);
        env.storage().temporary().remove(&key);
    }

    // -----------------------------------------------------------------------
    // Health monitoring
    // -----------------------------------------------------------------------

    pub fn set_health_failure_threshold(env: Env, threshold: u32) {
        Self::require_admin(&env);
        env.storage().instance().set(&key_health_threshold(&env), &threshold);
        env.storage().instance().extend_ttl(INSTANCE_TTL, INSTANCE_TTL);
    }

    pub fn update_health_status(
        env: Env,
        anchor: Address,
        latency_ms: u64,
        failure_count: u32,
        availability_percent: u32,
    ) {
        Self::require_admin(&env);
        let status = HealthStatus {
            anchor: anchor.clone(),
            latency_ms,
            failure_count,
            availability_percent,
        };
        let key = StorageKey::Health(anchor.clone());
        env.storage().persistent().set(&key, &status);
        env.storage().persistent().extend_ttl(&key, PERSISTENT_TTL, PERSISTENT_TTL);

        let threshold: u32 = env
            .storage()
            .instance()
            .get(&key_health_threshold(&env))
            .unwrap_or(0u32);

        if threshold > 0 && failure_count >= threshold {
            let meta_key = StorageKey::AnchorMeta(anchor.clone());
            if let Some(mut meta) = env
                .storage()
                .persistent()
                .get::<_, RoutingAnchorMeta>(&meta_key)
            {
                if meta.is_active {
                    meta.is_active = false;
                    env.storage().persistent().set(&meta_key, &meta);
                    env.storage().persistent().extend_ttl(&meta_key, PERSISTENT_TTL, PERSISTENT_TTL);
                    env.events().publish(
                        (symbol_short!("anchor"), symbol_short!("deactiv")),
                        AnchorDeactivated { anchor, failure_count, threshold },
                    );
                }
            }
        }
    }

    pub fn get_health_status(env: Env, anchor: Address) -> Option<HealthStatus> {
        env.storage()
            .persistent()
            .get::<_, HealthStatus>(&StorageKey::Health(anchor))
    }

    // -----------------------------------------------------------------------
    // Routing
    // -----------------------------------------------------------------------

    pub fn get_quote(env: Env, anchor: Address, quote_id: u64) -> Quote {
        let key = StorageKey::Quote(anchor.clone(), quote_id);
        env.storage().persistent().get::<_, Quote>(&key)
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::NoQuotesAvailable))
    }

    pub fn set_anchor_metadata(
        env: Env,
        anchor: Address,
        reputation_score: u32,
        average_settlement_time: u64,
        liquidity_score: u32,
        uptime_percentage: u32,
        total_volume: u64,
    ) {
        Self::require_admin(&env);
        let meta = RoutingAnchorMeta {
            anchor: anchor.clone(),
            reputation_score,
            average_settlement_time,
            liquidity_score,
            uptime_percentage,
            total_volume,
            is_active: true,
        };
        let meta_key = StorageKey::AnchorMeta(anchor.clone());
        env.storage().persistent().set(&meta_key, &meta);
        env.storage().persistent().extend_ttl(&meta_key, PERSISTENT_TTL, PERSISTENT_TTL);

        // Maintain ANCHLIST
        let list_key = key_anchor_list(&env);
        let mut list: Vec<Address> = env.storage().persistent()
            .get::<_, Vec<Address>>(&list_key)
            .unwrap_or_else(|| Vec::new(&env));
        if !list.contains(&anchor) {
            list.push_back(anchor);
            env.storage().persistent().set(&list_key, &list);
            env.storage().persistent().extend_ttl(&list_key, PERSISTENT_TTL, PERSISTENT_TTL);
        }
    }

    pub fn get_routing_anchors(env: Env) -> Vec<Address> {
        let list_key = key_anchor_list(&env);
        env.storage().persistent()
            .get::<_, Vec<Address>>(&list_key)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Select the best anchor for a transaction and return its `Quote`.
    ///
    /// Candidates are filtered to those that are active, meet `min_reputation`,
    /// have a non-expired quote, and whose quote range covers `request.amount`.
    ///
    /// The winner is then chosen by `options.strategy[0]`:
    ///
    /// - `"LowestFee"` — lowest `fee_percentage`
    /// - `"FastestSettlement"` — lowest `average_settlement_time`
    /// - `"HighestReputation"` — highest `reputation_score`
    ///
    /// An empty `strategy` vec panics with `NoQuotesAvailable`.
    /// An unrecognised symbol returns the first candidate in iteration order.
    pub fn route_transaction(env: Env, options: RoutingOptions) -> Quote {
        let now = env.ledger().timestamp();
        let list_key = key_anchor_list(&env);
        let anchors: Vec<Address> = env.storage().persistent()
            .get::<_, Vec<Address>>(&list_key)
            .unwrap_or_else(|| Vec::new(&env));

        let mut candidates: Vec<Quote> = Vec::new(&env);
        for anchor in anchors.iter() {
            // Check reputation filter
            let meta_key = StorageKey::AnchorMeta(anchor.clone());
            let meta: RoutingAnchorMeta = match env.storage().persistent().get(&meta_key) {
                Some(m) => m,
                None => continue,
            };
            if !meta.is_active { continue; }
            if meta.reputation_score < options.min_reputation { continue; }

            // Get latest quote for this anchor
            let lq_key = StorageKey::LatestQuote(anchor.clone());
            let quote_id: u64 = match env.storage().persistent().get(&lq_key) {
                Some(id) => id,
                None => continue,
            };
            let q_key = StorageKey::Quote(anchor.clone(), quote_id);
            let quote: Quote = match env.storage().persistent().get(&q_key) {
                Some(q) => q,
                None => continue,
            };

            if quote.valid_until <= now { continue; }
            if options.request.amount < quote.minimum_amount || options.request.amount > quote.maximum_amount {
                continue;
            }

            candidates.push_back(quote);
        }

        if candidates.is_empty() {
            panic_with_error!(&env, ErrorCode::NoQuotesAvailable);
        }

        let strategy_sym = options.strategy.get(0)
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::NoQuotesAvailable));

        let lowest_fee_sym = Symbol::new(&env, "LowestFee");
        let fastest_sym = Symbol::new(&env, "FastestSettlement");
        let reputation_sym = Symbol::new(&env, "HighestReputation");
        let balanced_sym = Symbol::new(&env, "Balanced");

        let mut best: Quote = candidates.get(0).unwrap();

        if strategy_sym == lowest_fee_sym {
            for q in candidates.iter() {
                if q.fee_percentage < best.fee_percentage {
                    best = q;
                }
            }
        } else if strategy_sym == fastest_sym {
            // Need settlement time from metadata
            let meta_key = StorageKey::AnchorMeta(best.anchor.clone());
            let mut best_time: u64 = env.storage().persistent()
                .get::<_, RoutingAnchorMeta>(&meta_key)
                .map(|m| m.average_settlement_time)
                .unwrap_or(u64::MAX);
            for q in candidates.iter() {
                let mk = StorageKey::AnchorMeta(q.anchor.clone());
                let t = env.storage().persistent()
                    .get::<_, RoutingAnchorMeta>(&mk)
                    .map(|m| m.average_settlement_time)
                    .unwrap_or(u64::MAX);
                if t < best_time {
                    best_time = t;
                    best = q;
                }
            }
        } else if strategy_sym == reputation_sym {
            let meta_key = StorageKey::AnchorMeta(best.anchor.clone());
            let mut best_rep: u32 = env.storage().persistent()
                .get::<_, RoutingAnchorMeta>(&meta_key)
                .map(|m| m.reputation_score)
                .unwrap_or(0);
            for q in candidates.iter() {
                let mk = StorageKey::AnchorMeta(q.anchor.clone());
                let rep = env.storage().persistent()
                    .get::<_, RoutingAnchorMeta>(&mk)
                    .map(|m| m.reputation_score)
                    .unwrap_or(0);
                if rep > best_rep {
                    best_rep = rep;
                    best = q;
                }
            }
        } else if strategy_sym == balanced_sym {
            // score = (40_000 / fee_percentage) + (30_000 / settlement_time) + (reputation * 3_000 / 10_000)
            // All terms are dimensionless integers; higher score is better.
            // fee_percentage = 0 or settlement_time = 0 contribute 0 to avoid division by zero.
            let balanced_score = |env: &Env, q: &Quote| -> u64 {
                let mk = (symbol_short!("ANCHMETA"), q.anchor.clone());
                let meta: RoutingAnchorMeta = env.storage().persistent()
                    .get(&mk)
                    .unwrap_or(RoutingAnchorMeta {
                        anchor: q.anchor.clone(),
                        reputation_score: 0,
                        average_settlement_time: 0,
                        liquidity_score: 0,
                        uptime_percentage: 0,
                        total_volume: 0,
                        is_active: false,
                    });
                let fee_term = if q.fee_percentage > 0 { 40_000 / q.fee_percentage as u64 } else { 0 };
                let time_term = if meta.average_settlement_time > 0 { 30_000 / meta.average_settlement_time } else { 0 };
                // Scale reputation (0–10_000) to a 0–3_000 range to match the weight of other terms.
                let rep_term = meta.reputation_score as u64 * 3_000 / 10_000;
                fee_term + time_term + rep_term
            };
            let mut best_score = balanced_score(&env, &best);
            for q in candidates.iter() {
                let score = balanced_score(&env, &q);
                if score > best_score {
                    best_score = score;
                    best = q;
                }
            }
        }

        best
    }

    // -----------------------------------------------------------------------
    // Anchor Info Discovery
    // -----------------------------------------------------------------------

    pub fn fetch_anchor_info(env: Env, anchor: Address, toml_data: StellarToml, ttl_override: Option<u64>) {
        anchor.require_auth();

        // Reject non-HTTPS endpoints to prevent MITM exposure of anchor metadata.
        let ts_len = toml_data.transfer_server.len() as usize;
        if ts_len > 2048 {
            return Err(ErrorCode::InvalidEndpointFormat);
        }
        let mut ts_buf = [0u8; 2048];
        toml_data.transfer_server.copy_into_slice(&mut ts_buf[..ts_len]);
        let transfer_server_str = core::str::from_utf8(&ts_buf[..ts_len]).unwrap_or("");
        if crate::validate_anchor_domain(transfer_server_str).is_err() {
            return Err(ErrorCode::InvalidEndpointFormat);
        }

        let now = env.ledger().timestamp();
        let ttl_seconds = ttl_override.unwrap_or(3600);
        let cached = CachedToml {
            toml: toml_data,
            cached_at: now,
            ttl_seconds,
        };
        let key = StorageKey::TomlCache(anchor);
        let ledger_ttl = if ttl_seconds as u32 > MIN_TEMP_TTL { ttl_seconds as u32 } else { MIN_TEMP_TTL };
        env.storage().temporary().set(&key, &cached);
        env.storage().temporary().extend_ttl(&key, ledger_ttl, ledger_ttl);
        Ok(())
    }

    pub fn get_anchor_toml(env: Env, anchor: Address) -> StellarToml {
        let key = StorageKey::TomlCache(anchor);
        let cached: CachedToml = env.storage().temporary().get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::CacheNotFound));
        let now = env.ledger().timestamp();
        if cached.cached_at + cached.ttl_seconds <= now {
            panic_with_error!(&env, ErrorCode::CacheExpired);
        }
        cached.toml
    }

    pub fn refresh_anchor_info(env: Env, anchor: Address, force: bool) {
        anchor.require_auth();
        let key = StorageKey::TomlCache(anchor);
        
        if force {
            env.storage().temporary().remove(&key);
        } else if let Some(cached) = env.storage().temporary().get::<_, CachedToml>(&key) {
            let now = env.ledger().timestamp();
            if cached.cached_at + cached.ttl_seconds <= now {
                env.storage().temporary().remove(&key);
            }
        }
    }

    pub fn get_anchor_assets(env: Env, anchor: Address) -> Result<Vec<String>, ErrorCode> {
        let key = (symbol_short!("TOMLCACHE"), anchor.clone());
        if !env.storage().temporary().has(&key) {
            return Err(ErrorCode::CacheNotFound);
        }
        let toml = Self::get_anchor_toml(env.clone(), anchor);
        let mut assets = Vec::new(&env);
        for asset in toml.currencies.iter() {
            assets.push_back(asset.code.clone());
        }
        Ok(assets)
    }

 feat/get-anchor-currencies
    /// Return the fiat currencies supported by `anchor` from its cached stellar.toml.
    /// Returns `Err(ErrorCode::CacheNotFound)` when no TOML has been cached for this anchor.
    pub fn get_anchor_currencies(
        env: Env,
        anchor: Address,
    ) -> Result<Vec<FiatCurrency>, ErrorCode> {
        let key = (symbol_short!("TOMLCACHE"), anchor.clone());
        if !env.storage().temporary().has(&key) {
            return Err(ErrorCode::CacheNotFound);
        }
        let toml = Self::get_anchor_toml(env.clone(), anchor);
        Ok(toml.fiat_currencies)
    }

    pub fn get_anchor_asset_info(
        env: Env,
        anchor: Address,
        asset_code: String,
    ) -> AssetInfo {

    pub fn get_anchor_asset_info(env: Env, anchor: Address, asset_code: String) -> AssetInfo {
 main
        let toml = Self::get_anchor_toml(env.clone(), anchor);
        for asset in toml.currencies.iter() {
            if asset.code == asset_code {
                return asset;
            }
        }
        panic_with_error!(&env, ErrorCode::ValidationError);
    }

    pub fn get_anchor_deposit_limits(env: Env, anchor: Address, asset_code: String) -> (u64, u64) {
        let asset = Self::get_anchor_asset_info(env, anchor, asset_code);
        (asset.deposit_min_amount, asset.deposit_max_amount)
    }

    pub fn get_anchor_withdrawal_limits(env: Env, anchor: Address, asset_code: String) -> (u64, u64) {
        let asset = Self::get_anchor_asset_info(env, anchor, asset_code);
        (asset.withdrawal_min_amount, asset.withdrawal_max_amount)
    }

    pub fn get_anchor_deposit_fees(env: Env, anchor: Address, asset_code: String) -> (u64, u32) {
        let asset = Self::get_anchor_asset_info(env, anchor, asset_code);
        (asset.deposit_fee_fixed, asset.deposit_fee_percent)
    }

    pub fn get_anchor_withdrawal_fees(env: Env, anchor: Address, asset_code: String) -> (u64, u32) {
        let asset = Self::get_anchor_asset_info(env, anchor, asset_code);
        (asset.withdrawal_fee_fixed, asset.withdrawal_fee_percent)
    }

    pub fn anchor_supports_deposits(
        env: Env,
        anchor: Address,
        asset_code: String,
    ) -> bool {
        Self::get_anchor_asset_info(env, anchor, asset_code).deposit_enabled
    }

    pub fn anchor_supports_withdrawals(
        env: Env,
        anchor: Address,
        asset_code: String,
    ) -> bool {
        Self::get_anchor_asset_info(env, anchor, asset_code).withdrawal_enabled
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn require_admin(env: &Env) {
        let admin: Address = env
            .storage()
            .instance()
            .get::<_, Address>(&key_admin(env))
            .unwrap_or_else(|| panic_with_error!(env, ErrorCode::NotInitialized));
        admin.require_auth();
    }

    fn check_attestor(env: &Env, attestor: &Address) {
        if !env
            .storage()
            .persistent()
            .has(&StorageKey::Attestor(attestor.clone()))
        {
            panic_with_error!(env, ErrorCode::AttestorNotRegistered);
        }
    }

    fn check_timestamp(env: &Env, timestamp: u64) {
        if timestamp == 0 {
            panic_with_error!(env, ErrorCode::InvalidTimestamp);
        }
    }

    fn next_attestation_id(env: &Env) -> u64 {
        let inst = env.storage().instance();
        let ck = key_counter(env);
        let id: u64 = inst.get(&ck).unwrap_or(0u64);
        let next = id.checked_add(1).unwrap_or_else(|| panic_with_error!(env, ErrorCode::ValidationError));
        inst.set(&ck, &next);
        inst.extend_ttl(INSTANCE_TTL, INSTANCE_TTL);
        id
    }

    fn store_attestation(
        env: &Env,
        id: u64,
        issuer: Address,
        subject: Address,
        timestamp: u64,
        payload_hash: Bytes,
        signature: Bytes,
    ) {
        let attestation = Attestation {
            id,
            issuer,
            subject: subject.clone(),
            timestamp,
            payload_hash,
            signature,
        };
        let key = StorageKey::Attest(id);
        env.storage().persistent().set(&key, &attestation);
        env.storage().persistent().extend_ttl(&key, PERSISTENT_TTL, PERSISTENT_TTL);

        // Subject-specific index for pagination support (#215)
        // Store only the ID to save storage space (O(1) extra space)
        let count_key = StorageKey::SubjectCount(subject.clone());
        let count: u64 = env.storage().persistent().get(&count_key).unwrap_or(0);

        let subj_att_key = StorageKey::SubjectAttestation(subject.clone(), count);
        env.storage().persistent().set(&subj_att_key, &id);
        env.storage().persistent().extend_ttl(&subj_att_key, PERSISTENT_TTL, PERSISTENT_TTL);

        env.storage().persistent().set(&count_key, &(count + 1));
        let total_key = symbol_short!("TOTALCNT");
        let total: u64 = env.storage().instance().get(&total_key).unwrap_or(0);
        env.storage().instance().set(&total_key, &(total + 1));
        env.storage()
            .persistent()
            .extend_ttl(&count_key, PERSISTENT_TTL, PERSISTENT_TTL);
    }

    fn store_span(env: &Env, request_id: &RequestId, operation: String, actor: Address, now: u64, status: String) {
        let span = TracingSpan {
            request_id: request_id.clone(),
            operation,
            actor,
            started_at: now,
            completed_at: now,
            status,
        };
        let key = StorageKey::Span(request_id.id.clone());
        env.storage().temporary().set(&key, &span);
        env.storage().temporary().extend_ttl(&key, SPAN_TTL, SPAN_TTL);
    }
}

pub fn get_endpoint(env: Env, attestor: Address) -> String {
    AnchorKitContract::get_endpoint(env, attestor)
}

pub fn set_endpoint(env: Env, attestor: Address, endpoint: String) {
    AnchorKitContract::set_endpoint(env, attestor, endpoint)
}

pub fn get_admin(env: Env) -> Address {
    AnchorKitContract::get_admin(env)
}
