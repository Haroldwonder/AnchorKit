use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, symbol_short, Address, Bytes, BytesN,
    Env, String, Symbol, Vec,
};

use crate::deterministic_hash::{compute_payload_hash, verify_payload_hash};
use crate::errors::ErrorCode;
use crate::sep10_jwt;
use crate::storage::{
    StorageKey,
    key_admin, key_counter, key_session_counter, key_quote_counter,
    key_audit_counter, key_anchor_list, key_health_threshold, key_replay_window,
    key_audit_log_offset,
};

// ---------------------------------------------------------------------------
// Types (re-exported from types module)
// ---------------------------------------------------------------------------

pub use crate::types::{
    AnchorMetadata, AnchorServices, AssetInfo, Attestation, AuditLog, CapabilitiesCache,
    CachedToml, FiatCurrency, HealthStatus, MetadataCache, OperationContext, Quote, RequestId,
    RoutingOptions, RoutingRequest, Session, StellarToml, TracingSpan,
    SERVICE_DEPOSITS, SERVICE_WITHDRAWALS, SERVICE_QUOTES, SERVICE_KYC, ServiceType,
};

const MIN_TEMP_TTL: u32 = 15; // min_temp_entry_ttl - 1
const LEDGER_PERIOD_SECS: u64 = 5; // approximate seconds per ledger

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
struct AuditLogPruned {
    pruned_count: u64,
    new_offset: u64,
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
struct AnchorDeactivated {
    anchor: Address,
    failure_count: u32,
    threshold: u32,
}

#[contracttype]
#[derive(Clone)]
struct SessionExpired {
    session_id: u64,
    expired_at: u64,
}

#[contracttype]
#[derive(Clone)]
struct AdminTransferProposed {
    current_admin: Address,
    new_admin: Address,
}

#[contracttype]
#[derive(Clone)]
struct AdminTransferred {
    old_admin: Address,
    new_admin: Address,
}

#[contracttype]
#[derive(Clone)]
struct AttestorRegistered(Address);

#[contracttype]
#[derive(Clone)]
struct AttestorRevoked(Address);

// ---------------------------------------------------------------------------
// TTLs (in ledgers)
//
// Stellar/Soroban ledgers close roughly every 5 seconds on mainnet and the
// public testnet, so the ledger counts below are sized as follows:
//   PERSISTENT_TTL = 1_555_200 ledgers ≈ 7_776_000 s ≈ 90 days
//   INSTANCE_TTL   =   518_400 ledgers ≈ 2_592_000 s ≈ 30 days
//   SPAN_TTL       =    17_280 ledgers ≈    86_400 s ≈ 24 hours
// If you deploy against a network with a different ledger close time, scale
// these constants accordingly (or override them per-network in a fork).
// ---------------------------------------------------------------------------
/// Persistent-storage TTL: ~90 days at 5 s/ledger.
const PERSISTENT_TTL: u32 = 1_555_200;
/// Temporary-storage TTL for tracing spans: ~24 hours at 5 s/ledger.
const SPAN_TTL: u32 = 17_280;
/// Instance-storage TTL: ~30 days at 5 s/ledger.
const INSTANCE_TTL: u32 = 518_400;
/// Session TTL in seconds (~24 hours).
const SESSION_TTL: u64 = 86_400;
/// Session storage TTL in ledgers (~24 hours at 5s/ledger).
const SESSION_LEDGER_TTL: u32 = 17_280;

fn pending_admin_key(env: &Env) -> soroban_sdk::Vec<soroban_sdk::Symbol> {
    soroban_sdk::vec![env, symbol_short!("PADMIN")]
}



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

    /// Initialise the contract.
    ///
    /// `replay_window_seconds` sets the tolerance window for timestamp-based
    /// replay attack detection.  Attestations whose timestamp falls outside
    /// `[now - window, now + window]` are rejected.
    ///
    /// Defaults to **300 seconds** (5 minutes) when `None` is supplied.
    pub fn initialize(env: Env, admin: Address, max_audit_log_size: u64, replay_window_seconds: Option<u64>) {
        admin.require_auth();
        if admin == env.current_contract_address() {
            panic_with_error!(&env, ErrorCode::ValidationError);
        }
        if max_audit_log_size == 0 {
            panic_with_error!(&env, ErrorCode::AuditLogMaxSizeInvalid);
        }
        let inst = env.storage().instance();
        if inst.has(&key_admin(&env)) {
            panic_with_error!(&env, ErrorCode::AlreadyInitialized);
        }
        inst.set(&key_admin(&env), &admin);
        inst.set(&StorageKey::AuditLogMaxSize, &max_audit_log_size);
        inst.set(&StorageKey::MaxPageSize, &50u32);
        // Default replay window: 300 seconds (5 minutes).
        let window = replay_window_seconds.unwrap_or(300u64);
        inst.set(&key_replay_window(&env), &window);
        inst.extend_ttl(INSTANCE_TTL, INSTANCE_TTL);
    }

    /// Propose new admin (current admin only). Sets pending_admin in instance storage.
    pub fn propose_admin(env: Env, new_admin: Address) {
        Self::require_admin(&env);
        let inst = env.storage().instance();
        if inst.has(&pending_admin_key(&env)) {
            panic_with_error!(&env, ErrorCode::PendingAdminAlreadyExists);
        }
        if new_admin == env.current_contract_address() {
            panic_with_error!(&env, ErrorCode::ValidationError);
        }
        inst.set(&pending_admin_key(&env), &new_admin);
        inst.extend_ttl(INSTANCE_TTL, INSTANCE_TTL);
        let current = Self::get_admin(env.clone());
        env.events().publish(
            (symbol_short!("admin"), symbol_short!("proposed")),
            AdminTransferProposed {
                current_admin: current,
                new_admin,
            },
        );
    }

    /// Accept admin transfer (pending admin only). Updates admin, clears pending.
    pub fn accept_admin(env: Env) {
        let inst = env.storage().instance();
        let pending: Address = inst
            .get(&pending_admin_key(&env))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::NoPendingAdmin));
        pending.require_auth();
        let old_admin = Self::get_admin(env.clone());
        inst.set(&key_admin(&env), &pending);
        inst.remove(&pending_admin_key(&env));
        inst.extend_ttl(INSTANCE_TTL, INSTANCE_TTL);
        env.events().publish(
            (symbol_short!("admin"), symbol_short!("transf")),
            AdminTransferred {
                old_admin,
                new_admin: pending,
            },
        );
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get::<_, Address>(&key_admin(&env))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::NotInitialized))
    }

    pub fn get_max_page_size(env: Env) -> u32 {
        env.storage()
            .instance()
            .get::<_, u32>(&StorageKey::MaxPageSize)
            .unwrap_or(50u32)
    }

    pub fn set_max_page_size(env: Env, max_page_size: u32) {
        Self::require_admin(&env);
        if max_page_size == 0 {
            panic_with_error!(&env, ErrorCode::ValidationError);
        }
        env.storage().instance().set(&StorageKey::MaxPageSize, &max_page_size);
        env.storage().instance().extend_ttl(INSTANCE_TTL, INSTANCE_TTL);
        env.events().publish(
            (symbol_short!("pagesize"), symbol_short!("updated")),
            max_page_size,
        );
    }

    /// Returns `true` if the contract has been initialized, `false` otherwise.
    /// Safe to call at any time — never panics.
    pub fn is_initialized(env: Env) -> bool {
        env.storage().instance().has(&key_admin(&env))
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
        let id = Bytes::from_array(&env, &hash.into());

        RequestId { id, created_at: ts }
    }

    // -----------------------------------------------------------------------
    // Attestor management
    // -----------------------------------------------------------------------

    pub fn upsert_sep10_verifying_key(env: Env, issuer: Address, public_key: Bytes) {
        Self::require_admin(&env);
        if public_key.len() != 32 {
            panic_with_error!(&env, ErrorCode::ValidationError);
        }
        let storage_key = StorageKey::Sep10Key(issuer.clone());
        let mut keys: Vec<Bytes> = env
            .storage()
            .persistent()
            .get(&storage_key)
            .unwrap_or_else(|| Vec::new(&env));
        // Replace in-place if the key already exists; otherwise append.
        let mut found = false;
        for i in 0..keys.len() {
            if keys.get(i).unwrap() == public_key {
                keys.set(i, public_key.clone());
                found = true;
                break;
            }
        }
        if !found {
            if keys.len() >= sep10_jwt::MAX_VERIFYING_KEYS {
                panic_with_error!(&env, ErrorCode::ValidationError);
            }
            keys.push_back(public_key);
        }
        env.storage().persistent().set(&storage_key, &keys);
        env.storage()
            .persistent()
            .extend_ttl(&storage_key, PERSISTENT_TTL, PERSISTENT_TTL);
    }

    /// Deprecated alias kept for backward compatibility. Use `upsert_sep10_verifying_key` instead.
    pub fn set_sep10_jwt_verifying_key(env: Env, issuer: Address, public_key: Bytes) {
        Self::upsert_sep10_verifying_key(env, issuer, public_key);
    }

    pub fn add_sep10_verifying_key(env: Env, issuer: Address, public_key: Bytes) {
        Self::require_admin(&env);
        if public_key.len() != 32 {
            panic_with_error!(&env, ErrorCode::ValidationError);
        }
        let storage_key = StorageKey::Sep10Key(issuer.clone());
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
        let storage_key = StorageKey::Sep10Key(issuer.clone());
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
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::MissingSigningKey));
        if keys.is_empty() {
            panic_with_error!(&env, ErrorCode::MissingSigningKey);
        }
        if sep10_jwt::verify_sep10_jwt(&env, &token, &keys, None, 0).is_err() {
            panic_with_error!(&env, ErrorCode::InvalidSep10Token);
        }
    }

    /// Verify a SEP-10 token and additionally confirm it is scoped for `service`.
    ///
    /// `service` must be one of the `SERVICE_*` constants (1 = Deposits, 2 = Withdrawals,
    /// 3 = Quotes, 4 = KYC). Panics with `InvalidSep10Token` if the signature is invalid,
    /// the token is expired, or the `scp` claim does not include the required service scope.
    pub fn verify_sep10_token_for_service(env: Env, token: String, issuer: Address, service: u32) {
        let keys: Vec<Bytes> = env
            .storage()
            .persistent()
            .get(&StorageKey::Sep10Key(issuer.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::MissingSigningKey));
        if keys.is_empty() {
            panic_with_error!(&env, ErrorCode::MissingSigningKey);
        }
        if sep10_jwt::verify_sep10_jwt(&env, &token, &keys, None, 0).is_err() {
            panic_with_error!(&env, ErrorCode::InvalidSep10Token);
        }
        if sep10_jwt::check_token_scope(&env, &token, service).is_err() {
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
            .unwrap_or_else(|| panic_with_error!(env, ErrorCode::MissingSigningKey));
        if keys.is_empty() {
            panic_with_error!(env, ErrorCode::MissingSigningKey);
        }
        let expected = attestor.to_string();
        if sep10_jwt::verify_sep10_jwt(env, token, &keys, Some(&expected), 0).is_err() {
            panic_with_error!(env, ErrorCode::InvalidSep10Token);
        }
    }

    fn add_to_cached_anchors(env: &Env, anchor: &Address) {
        let list_key = soroban_sdk::vec![env, symbol_short!("CANCHORS")];
        let mut list: Vec<Address> = env.storage().persistent()
            .get::<_, Vec<Address>>(&list_key)
            .unwrap_or_else(|| Vec::new(env));
        if !list.contains(anchor) {
            list.push_back(anchor.clone());
            env.storage().persistent().set(&list_key, &list);
            env.storage().persistent().extend_ttl(&list_key, PERSISTENT_TTL, PERSISTENT_TTL);
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
        // Mark the attestor as revoked so historical attestations surface issuer_revoked=true.
        let revoked_key = StorageKey::AttestorRevoked(attestor.clone());
        env.storage().persistent().set(&revoked_key, &true);
        env.storage().persistent().extend_ttl(&revoked_key, PERSISTENT_TTL, PERSISTENT_TTL);
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
        let endpoint: String = env
            .storage()
            .persistent()
            .get::<_, String>(&StorageKey::Endpoint(attestor))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::AttestorNotRegistered));

        // Re-validate on read: a storage entry written by an older contract version
        // with looser validation rules must not be returned as a trusted URL.
        let len = endpoint.len() as usize;
        if len > 128 {
            panic_with_error!(&env, ErrorCode::InvalidEndpointFormat);
        }
        let mut rust_buf = [0u8; 128];
        endpoint.copy_into_slice(&mut rust_buf[..len]);
        let endpoint_str = core::str::from_utf8(&rust_buf[..len]).unwrap_or("");
        if crate::validate_anchor_domain(endpoint_str).is_err() {
            panic_with_error!(&env, ErrorCode::InvalidEndpointFormat);
        }

        endpoint
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
            // Reject any value that is not one of the four known service constants.
            if s != SERVICE_DEPOSITS
                && s != SERVICE_WITHDRAWALS
                && s != SERVICE_QUOTES
                && s != SERVICE_KYC
            {
                panic_with_error!(&env, ErrorCode::InvalidServiceType);
            }
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
        if let Err(e) = crate::rate_limiter::RateLimiter::check_and_increment(&env, &issuer) {
            panic_with_error!(&env, e);
        }
        Self::check_timestamp(&env, timestamp);
        Self::verify_attestation_signature(&env, &issuer, &payload_hash, &signature);

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
        if let Err(e) = crate::rate_limiter::RateLimiter::check_and_increment(&env, &issuer) {
            panic_with_error!(&env, e);
        }
        Self::check_timestamp(&env, timestamp);
        Self::verify_attestation_signature(&env, &issuer, &payload_hash, &signature);

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

        let inst = env.storage().instance();
        let qcnt_key = key_quote_counter(&env);
        let next: u64 = inst.get(&qcnt_key).unwrap_or(0u64) + 1;
        inst.set(&qcnt_key, &next);
        inst.extend_ttl(INSTANCE_TTL, INSTANCE_TTL);

        let quote = Quote {
            quote_id: next,
            anchor: anchor.clone(),
            base_asset: from_asset,
            quote_asset: to_asset,
            rate: amount,
            fee_percentage: fee_bps,
            minimum_amount: min_amount,
            maximum_amount: max_amount,
            valid_until: expires_at,
        };
        let q_key = StorageKey::Quote(anchor.clone(), next);
        env.storage().persistent().set(&q_key, &quote);
        env.storage().persistent().extend_ttl(&q_key, PERSISTENT_TTL, PERSISTENT_TTL);

        let lq_key = StorageKey::LatestQuote(anchor.clone());
        env.storage().persistent().set(&lq_key, &next);
        env.storage().persistent().extend_ttl(&lq_key, PERSISTENT_TTL, PERSISTENT_TTL);

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
        let mut attestation = env.storage()
            .persistent()
            .get::<_, Attestation>(&StorageKey::Attest(id))?;
        // Reflect current revocation status without rewriting every stored attestation.
        if env.storage().persistent().has(&StorageKey::AttestorRevoked(attestation.issuer.clone())) {
            attestation.issuer_revoked = true;
        }
        Some(attestation)
    }

    pub fn list_attestations(env: Env, subject: Address, offset: u64, limit: u32) -> Vec<Attestation> {
        let max_page_size = Self::get_max_page_size(env.clone());
        let actual_limit = if limit > max_page_size { max_page_size } else { limit };
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
                if let Some(mut attestation) = env.storage().persistent().get::<_, Attestation>(&main_key) {
                    if env.storage().persistent().has(&StorageKey::AttestorRevoked(attestation.issuer.clone())) {
                        attestation.issuer_revoked = true;
                    }
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

    /// Compute the canonical payload hash via the contract method.
    ///
    /// Off-chain callers should prefer this method over calling
    /// `deterministic_hash::compute_payload_hash` directly. Going through the
    /// Soroban host environment ensures that `Address` XDR serialisation uses
    /// the same host-side encoding as on-chain attestation submission, avoiding
    /// divergence that can occur when the module function is called outside the
    /// host context.
    pub fn compute_payload_hash_public(env: Env, subject: Address, timestamp: u64, data: Bytes) -> BytesN<32> {
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
        let nonce: u64 = env.prng().gen_range(u64::MIN..=u64::MAX);
        let session = Session {
            session_id,
            initiator: initiator.clone(),
            created_at: now,
            nonce,
            operation_count: 0,
            expires_at: now + SESSION_TTL,
        };
        let sess_key = StorageKey::Session(session_id);
        env.storage().persistent().set(&sess_key, &session);
        env.storage().persistent().extend_ttl(&sess_key, SESSION_LEDGER_TTL, SESSION_LEDGER_TTL);

        env.events().publish(
            (symbol_short!("session"), symbol_short!("created"), session_id),
            SessionCreatedEvent { session_id, initiator, timestamp: now },
        );

        session_id
    }

    // get_session, get_audit_log, get_session_operation_count defined later in the session-aware section.

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

        // Validate quote parameters
        if rate == 0 {
            panic_with_error!(&env, ErrorCode::InvalidQuote);
        }
        if minimum_amount > maximum_amount {
            panic_with_error!(&env, ErrorCode::InvalidQuote);
        }
        let now = env.ledger().timestamp();
        if valid_until <= now {
            panic_with_error!(&env, ErrorCode::InvalidQuote);
        }

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
    // Audit log pruning helper
    // -----------------------------------------------------------------------

    /// Prune oldest audit log entries if the log has exceeded `max_audit_log_size`.
    ///
    /// Uses a monotonically increasing `log_id` counter (total entries ever written)
    /// and a separate `offset` (first live entry). The live window is
    /// `[offset, log_id)`. When `log_id - offset > max_size`, we advance `offset`
    /// and delete the stale entries, then emit `AuditLogPruned`.
    fn maybe_prune_audit_log(env: &Env, log_id: u64) {
        let inst = env.storage().instance();
        let max_size: u64 = inst
            .get(&StorageKey::AuditLogMaxSize)
            .unwrap_or(u64::MAX);
        let offset: u64 = inst.get(&key_audit_log_offset(env)).unwrap_or(0u64);
        let live_count = log_id.saturating_sub(offset); // entries [offset, log_id)
        if live_count < max_size {
            return;
        }
        // Number of entries to remove so live_count == max_size - 1 (leaving room for the new one)
        let to_prune = live_count - max_size + 1;
        for i in 0..to_prune {
            let old_key = StorageKey::AuditLog(offset + i);
            env.storage().persistent().remove(&old_key);
        }
        let new_offset = offset + to_prune;
        inst.set(&key_audit_log_offset(env), &new_offset);
        env.events().publish(
            (symbol_short!("audit"), symbol_short!("pruned")),
            AuditLogPruned { pruned_count: to_prune, new_offset },
        );
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
        Self::check_session_expiry(&env, session_id);
        let session = Self::get_session(env.clone(), session_id);
        if session.initiator != issuer {
            panic_with_error!(&env, ErrorCode::UnauthorizedAttestor);
        }
        issuer.require_auth();
        Self::check_attestor(&env, &issuer);
        Self::check_timestamp(&env, timestamp);
        Self::verify_attestation_signature(&env, &issuer, &payload_hash, &signature);

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
        Self::maybe_prune_audit_log(&env, log_id);

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
                result_summary: String::from_str(&env, &alloc::format!("attestation_id={}", id)),
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

    pub fn register_attestor_with_session(env: Env, session_id: u64, attestor: Address, sep10_token: String, sep10_issuer: Address) {
        Self::check_session_expiry(&env, session_id);
        Self::require_admin(&env);
        Self::verify_sep10_token_matches_attestor(&env, &sep10_token, &sep10_issuer, &attestor);
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
        Self::maybe_prune_audit_log(&env, log_id);

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
                result_summary: String::from_str(&env, "attestor_registered"),
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
        Self::check_session_expiry(&env, session_id);
        Self::require_admin(&env);
        let key = StorageKey::Attestor(attestor.clone());
        if !env.storage().persistent().has(&key) {
            panic_with_error!(&env, ErrorCode::AttestorNotRegistered);
        }
        env.storage().persistent().remove(&key);
        // Mark the attestor as revoked so historical attestations surface issuer_revoked=true.
        let revoked_key = StorageKey::AttestorRevoked(attestor.clone());
        env.storage().persistent().set(&revoked_key, &true);
        env.storage().persistent().extend_ttl(&revoked_key, PERSISTENT_TTL, PERSISTENT_TTL);

        let sopcnt_key = StorageKey::SessionOpCount(session_id);
        let op_index: u64 = env.storage().persistent().get(&sopcnt_key).unwrap_or(0u64);
        env.storage().persistent().set(&sopcnt_key, &(op_index + 1));
        env.storage().persistent().extend_ttl(&sopcnt_key, PERSISTENT_TTL, PERSISTENT_TTL);

        let inst = env.storage().instance();
        let acnt_key = key_audit_counter(&env);
        let log_id: u64 = inst.get(&acnt_key).unwrap_or(0u64);
        inst.set(&acnt_key, &(log_id + 1));
        inst.extend_ttl(INSTANCE_TTL, INSTANCE_TTL);
        Self::maybe_prune_audit_log(&env, log_id);

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
                result_summary: String::from_str(&env, "attestor_revoked"),
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

    pub fn get_session_operation_count(env: Env, session_id: u64) -> Option<u64> {
        let sess_key = StorageKey::Session(session_id);
        if !env.storage().persistent().has(&sess_key) {
            return None;
        }
        Self::check_session_expiry(&env, session_id);
        Some(
            env.storage()
                .persistent()
                .get::<_, u64>(&StorageKey::SessionOpCount(session_id))
                .unwrap_or(0),
        )
    }

    /// Returns the current audit log pruning offset — the ID of the first live
    /// audit log entry.  Any log ID below this value has been pruned and is no
    /// longer available in storage.
    ///
    /// Callers can use this to detect gaps in a session's audit trail:
    /// if `log_id < get_audit_log_offset()` the entry was intentionally pruned,
    /// not lost.  Returns `0` when no pruning has occurred yet.
    pub fn get_audit_log_offset(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&key_audit_log_offset(&env))
            .unwrap_or(0u64)
    }

    // -----------------------------------------------------------------------
    // Metadata cache
    // -----------------------------------------------------------------------

    pub fn cache_metadata(env: Env, anchor: Address, metadata: AnchorMetadata, ttl_seconds: u64) {
        Self::require_admin(&env);
        // Issue #259: skip write if metadata is unchanged.
        // ttl_seconds=0 entries live in persistent storage (never network-evicted);
        // all other entries live in temporary storage.
        let key = StorageKey::MetadataCache(anchor.clone());
        let existing: Option<MetadataCache> = if ttl_seconds == 0 {
            env.storage().persistent().get(&key)
        } else {
            env.storage().temporary().get(&key)
        };
        if let Some(existing) = existing {
            let m = &existing.metadata;
            if m.anchor == metadata.anchor
                && m.reputation_score == metadata.reputation_score
                && m.liquidity_score == metadata.liquidity_score
                && m.uptime_percentage == metadata.uptime_percentage
                && m.total_volume == metadata.total_volume
                && m.average_settlement_time == metadata.average_settlement_time
                && m.is_active == metadata.is_active
            {
                return;
            }
        }
        let now = env.ledger().timestamp();
        let entry = MetadataCache { metadata, cached_at: now, ttl_seconds };
        if ttl_seconds == 0 {
            // Persistent storage is not subject to network-level eviction within
            // its TTL window, so ttl_seconds=0 ("never expire at app level") is
            // backed by storage that won't disappear under the contract's feet.
            env.storage().persistent().set(&key, &entry);
            env.storage().persistent().extend_ttl(&key, PERSISTENT_TTL, PERSISTENT_TTL);
        } else {
            let ledger_ttl = if ttl_seconds as u32 > MIN_TEMP_TTL { ttl_seconds as u32 } else { MIN_TEMP_TTL };
            env.storage().temporary().set(&key, &entry);
            env.storage().temporary().extend_ttl(&key, ledger_ttl, ledger_ttl);
        }

        Self::add_to_cached_anchors(&env, &anchor);
    }

    pub fn get_cached_metadata(env: Env, anchor: Address) -> AnchorMetadata {
        let key = StorageKey::MetadataCache(anchor);
        // ttl_seconds=0 entries are stored persistently; all others are temporary.
        let entry: MetadataCache = env.storage().persistent().get(&key)
            .or_else(|| env.storage().temporary().get(&key))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::CacheNotFound));
        let now = env.ledger().timestamp();
        // ttl_seconds = 0 means "never expire at app level" — skip the expiry check
        if entry.ttl_seconds != 0 && entry.cached_at + entry.ttl_seconds <= now {
            panic_with_error!(&env, ErrorCode::CacheExpired);
        }
        entry.metadata
    }

    /// Issue #260: returns seconds elapsed since the metadata cache entry was written,
    /// or `Err(CacheNotFound)` if no cache entry exists for the anchor.
    /// `Ok(0)` means the entry was just cached at the current ledger timestamp.
    pub fn get_cache_age_seconds(env: Env, anchor: Address) -> Result<u64, ErrorCode> {
        let key = StorageKey::MetadataCache(anchor);
        let entry: MetadataCache = env.storage().persistent().get(&key)
            .or_else(|| env.storage().temporary().get(&key))?;
        let now = env.ledger().timestamp();
        Ok(now.saturating_sub(entry.cached_at))
    }

    // #272: return the cached data so callers avoid a second storage read.
    pub fn refresh_metadata_cache(env: Env, anchor: Address) -> AnchorMetadata {
        Self::require_admin(&env);
        let key = StorageKey::MetadataCache(anchor.clone());
        let entry: MetadataCache = env.storage().persistent().get(&key)
            .or_else(|| env.storage().temporary().get(&key))
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::CacheNotFound));
        let metadata = entry.metadata.clone();
        // Remove from whichever storage tier holds the entry.
        env.storage().persistent().remove(&key);
        env.storage().temporary().remove(&key);

        // Issue #276: remove from CACHED_ANCHORS set
        let list_key = key_anchor_list(&env);
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
        metadata
    }
    /// Computes a health score (0-100) for an anchor based on cached metadata.
    ///
    /// # Formula
    ///
    /// The health score is a weighted combination of three metrics:
    /// - **Uptime (40%)**: `uptime_percentage / 100` (0-10000 scale → 0-100)
    /// - **Reputation (35%)**: `reputation_score / 100` (0-10000 scale → 0-100)
    /// - **Settlement Speed (25%)**: Inverse of `average_settlement_time`, normalized
    ///
    /// Settlement speed scoring:
    /// - 0-300s: 100 points (excellent)
    /// - 301-600s: 80 points (good)
    /// - 601-1800s: 60 points (acceptable)
    /// - 1801-3600s: 40 points (slow)
    /// - >3600s: 20 points (very slow)
    ///
    /// Final score = (uptime_weight × uptime_score) + (reputation_weight × reputation_score) + (speed_weight × speed_score)
    ///
    /// # Errors
    ///
    /// - `CacheNotFound` (49): No metadata cached for this anchor
    /// - `CacheExpired` (48): Metadata cache has expired
    ///
    /// # Example
    ///
    /// ```ignore
    /// let score = contract.get_anchor_health_score(&env, &anchor_addr);
    /// // score is 0-100, where 100 is perfect health
    /// ```
    pub fn get_anchor_health_score(env: Env, anchor: Address) -> u32 {
        // Retrieve cached metadata (will panic with CacheNotFound or CacheExpired if unavailable)
        let metadata = Self::get_cached_metadata(env.clone(), anchor);

        // Weight constants (must sum to 100)
        const UPTIME_WEIGHT: u64 = 40;
        const REPUTATION_WEIGHT: u64 = 35;
        const SPEED_WEIGHT: u64 = 25;
        const SCALE_FACTOR: u64 = 100; // Fixed-point scaling to preserve precision

        // 1. Uptime score: scale from 0-10000 to 0-100 using fixed-point arithmetic
        // Multiply by SCALE_FACTOR before dividing to preserve decimal precision
        let uptime_score = (metadata.uptime_percentage as u64 * SCALE_FACTOR) / 100;

        // 2. Reputation score: scale from 0-10000 to 0-100 using fixed-point arithmetic
        let reputation_score = (metadata.reputation_score as u64 * SCALE_FACTOR) / 100;

        // 3. Settlement speed score: tiered scoring based on settlement time
        let speed_score = if metadata.average_settlement_time <= 300 {
            100 * SCALE_FACTOR // Excellent: ≤5 minutes
        } else if metadata.average_settlement_time <= 600 {
            80 * SCALE_FACTOR // Good: 5-10 minutes
        } else if metadata.average_settlement_time <= 1800 {
            60 * SCALE_FACTOR // Acceptable: 10-30 minutes
        } else if metadata.average_settlement_time <= 3600 {
            40 * SCALE_FACTOR // Slow: 30-60 minutes
        } else {
            20 * SCALE_FACTOR // Very slow: >1 hour
        };

        // Calculate weighted health score using fixed-point arithmetic
        // All intermediate values are scaled by SCALE_FACTOR, so divide once at the end
        let weighted_sum = UPTIME_WEIGHT * uptime_score
            + REPUTATION_WEIGHT * reputation_score
            + SPEED_WEIGHT * speed_score;
        
        // Divide by (100 * SCALE_FACTOR) to get final score: 100 for weights, SCALE_FACTOR for precision
        let health_score = weighted_sum / (100 * SCALE_FACTOR);

        // Ensure score is capped at 100
        let final_score = if health_score > 100 { 100 } else { health_score as u32 };

        // Issue #464: enforce configurable minimum acceptable health score.
        // When key_health_threshold is set (> 0), reject anchors whose computed
        // score falls below it so callers cannot route to unhealthy anchors.
        let threshold: u32 = env
            .storage()
            .instance()
            .get(&key_health_threshold(&env))
            .unwrap_or(0u32);
        if threshold > 0 && final_score < threshold {
            panic_with_error!(&env, ErrorCode::ValidationError);
        }

        final_score
    }


    /// Issue #276: list all anchors that currently have active metadata cache entries.
    ///
    /// Filters out any anchor whose temporary cache keys have all been evicted, and
    /// writes the pruned list back to persistent storage so CANCHORS does not grow
    /// unboundedly after natural TTL eviction.
    pub fn list_cached_anchors(env: Env) -> Vec<Address> {
        let list_key = soroban_sdk::vec![&env, symbol_short!("CANCHORS")];
        let list: Vec<Address> = env.storage().persistent()
            .get::<_, Vec<Address>>(&list_key)
            .unwrap_or_else(|| Vec::new(&env));

        let mut active = Vec::new(&env);
        let mut pruned = false;
        for anchor in list.iter() {
            let meta_key = StorageKey::MetadataCache(anchor.clone());
            let caps_key = StorageKey::CapabilitiesCache(anchor.clone());
            let toml_key = StorageKey::TomlCache(anchor.clone());
            if env.storage().temporary().has(&meta_key)
                || env.storage().temporary().has(&caps_key)
                || env.storage().temporary().has(&toml_key)
            {
                active.push_back(anchor);
            } else {
                // All temp entries for this anchor have been evicted; mark for pruning.
                pruned = true;
            }
        }

        // Write back the pruned list so stale entries don't accumulate in persistent storage.
        if pruned {
            env.storage().persistent().set(&list_key, &active);
            env.storage().persistent().extend_ttl(&list_key, PERSISTENT_TTL, PERSISTENT_TTL);
        }

        active
    }

    // -----------------------------------------------------------------------
    // Capabilities cache
    // -----------------------------------------------------------------------

    pub fn cache_capabilities(env: Env, anchor: Address, toml_url: String, capabilities: Vec<u32>, ttl_seconds: u64) {
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
        let key = StorageKey::CapabilitiesCache(anchor.clone());
        let ledger_ttl = if ttl_seconds as u32 > MIN_TEMP_TTL { ttl_seconds as u32 } else { MIN_TEMP_TTL };
        env.storage().temporary().set(&key, &entry);
        env.storage().temporary().extend_ttl(&key, ledger_ttl, ledger_ttl);

        Self::add_to_cached_anchors(&env, &anchor);
    }

    pub fn get_cached_capabilities(env: Env, anchor: Address) -> CapabilitiesCache {
        let key = StorageKey::CapabilitiesCache(anchor);
        let entry: CapabilitiesCache = env.storage().temporary().get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, ErrorCode::CacheNotFound));
        let now = env.ledger().timestamp();
        // ttl_seconds = 0 means "never expire" — skip the expiry check to prevent refresh loops
        if entry.ttl_seconds != 0 && entry.cached_at + entry.ttl_seconds <= now {
            panic_with_error!(&env, ErrorCode::CacheExpired);
        }
        entry
    }

    pub fn refresh_capabilities_cache(env: Env, anchor: Address) {
        Self::require_admin(&env);
        let key = StorageKey::CapabilitiesCache(anchor.clone());
        env.storage().temporary().remove(&key);

        // If no other cache entries remain for this anchor, remove it from CANCHORS.
        let meta_key = StorageKey::MetadataCache(anchor.clone());
        let toml_key = StorageKey::TomlCache(anchor.clone());
        if !env.storage().temporary().has(&meta_key) && !env.storage().temporary().has(&toml_key) {
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
    }

    /// Issue #258/#463: admin-only emergency flush of all MetadataCache,
    /// CapabilitiesCache, and TomlCache entries for every tracked anchor.
    /// Emits a `CacheInvalidated` event with the count of cleared entries.
    pub fn invalidate_all_caches(env: Env) {
        Self::require_admin(&env);
        let list_key = key_anchor_list(&env);
        let anchors: Vec<Address> = env.storage().persistent()
            .get::<_, Vec<Address>>(&list_key)
            .unwrap_or_else(|| Vec::new(&env));

        let mut count: u32 = 0;
        for anchor in anchors.iter() {
            let meta_key = StorageKey::MetadataCache(anchor.clone());
            if env.storage().temporary().has(&meta_key) {
                env.storage().temporary().remove(&meta_key);
                count += 1;
            }
            let caps_key = StorageKey::CapabilitiesCache(anchor.clone());
            if env.storage().temporary().has(&caps_key) {
                env.storage().temporary().remove(&caps_key);
                count += 1;
            }
            // Issue #463: also flush cached stellar.toml entries
            let toml_key = StorageKey::TomlCache(anchor.clone());
            if env.storage().temporary().has(&toml_key) {
                env.storage().temporary().remove(&toml_key);
                count += 1;
            }
        }

        // Clear the anchor list
        let empty: Vec<Address> = Vec::new(&env);
        env.storage().persistent().set(&list_key, &empty);
        env.storage().persistent().extend_ttl(&list_key, PERSISTENT_TTL, PERSISTENT_TTL);

        env.events().publish(
            (symbol_short!("cache"), symbol_short!("invall")),
            count,
        );
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
                .get::<_, AnchorMetadata>(&meta_key)
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

    pub fn get_quote(env: Env, anchor: Address, quote_id: u64) -> Option<Quote> {
        env.storage().persistent().get::<_, Quote>(&StorageKey::Quote(anchor, quote_id))
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
        let meta = AnchorMetadata {
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
            let meta: AnchorMetadata = match env.storage().persistent().get(&meta_key) {
                Some(m) => m,
                None => continue,
            };
            if !meta.is_active { continue; }
            if meta.reputation_score < options.min_reputation { continue; }

            // Check KYC requirement filter
            if options.require_kyc {
                let services_key = StorageKey::Services(anchor.clone());
                let services_record: AnchorServices = match env.storage().persistent().get(&services_key) {
                    Some(sr) => sr,
                    None => continue,
                };
                if !services_record.services.contains(SERVICE_KYC) {
                    continue;
                }
            }

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

            if quote.valid_until <= now {
                env.events().publish(
                    (symbol_short!("quote"),),
                    crate::events::QuoteExpiredEvent {
                        anchor: anchor.clone(),
                        quote_id,
                        valid_until: quote.valid_until,
                    },
                );
                continue;
            }
            if options.request.amount < quote.minimum_amount || options.request.amount > quote.maximum_amount {
                continue;
            }

            candidates.push_back(quote);

            // Stop adding candidates if we've reached max_anchors limit
            if options.max_anchors > 0 && candidates.len() >= options.max_anchors as usize {
                break;
            }
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

        // Validate that the strategy symbol is recognized
        if strategy_sym != lowest_fee_sym 
            && strategy_sym != fastest_sym 
            && strategy_sym != reputation_sym 
            && strategy_sym != balanced_sym {
            panic_with_error!(&env, ErrorCode::InvalidStrategy);
        }

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
                .get::<_, AnchorMetadata>(&meta_key)
                .map(|m| m.average_settlement_time)
                .unwrap_or(u64::MAX);
            for q in candidates.iter() {
                let mk = StorageKey::AnchorMeta(q.anchor.clone());
                let t = env.storage().persistent()
                    .get::<_, AnchorMetadata>(&mk)
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
                .get::<_, AnchorMetadata>(&meta_key)
                .map(|m| m.reputation_score)
                .unwrap_or(0);
            for q in candidates.iter() {
                let mk = StorageKey::AnchorMeta(q.anchor.clone());
                let rep = env.storage().persistent()
                    .get::<_, AnchorMetadata>(&mk)
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
                let meta: AnchorMetadata = env.storage().persistent()
                    .get(&mk)
                    .unwrap_or(AnchorMetadata {
                        anchor: q.anchor.clone(),
                        reputation_score: 0,
                        average_settlement_time: 0,
                        liquidity_score: 0,
                        uptime_percentage: 0,
                        total_volume: 0,
                        is_active: false,
                    });
                let fee_term = if q.fee_percentage > 0 { 40_000 / q.fee_percentage as u64 } else { 0 };
                let time_term = 30_000u64.checked_div(meta.average_settlement_time).unwrap_or(0);
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

        let strategy_str = if strategy_sym == lowest_fee_sym {
            String::from_str(&env, "LowestFee")
        } else if strategy_sym == fastest_sym {
            String::from_str(&env, "FastestSettlement")
        } else if strategy_sym == reputation_sym {
            String::from_str(&env, "HighestReputation")
        } else {
            String::from_str(&env, "Balanced")
        };

        env.events().publish(
            (symbol_short!("routing"),),
            crate::events::RoutingDecisionEvent {
                anchor: best.anchor.clone(),
                strategy: strategy_str,
                quote_id: best.quote_id,
                ledger_sequence: env.ledger().sequence(),
            },
        );

        best
    }

    // -----------------------------------------------------------------------
    // Anchor Info Discovery
    // -----------------------------------------------------------------------

    pub fn fetch_anchor_info(env: Env, anchor: Address, toml_data: StellarToml, network_passphrase: String, ttl_override: Option<u64>) {
        anchor.require_auth();

        // Validate network_passphrase matches a known network before caching.
        // Prevents caching a TOML from the wrong network, which would cause
        // SEP-10 verification to succeed against the wrong signing key.
        let np_len = network_passphrase.len() as usize;
        if np_len > 256 {
            panic_with_error!(&env, ErrorCode::ValidationError);
        }
        let mut np_buf = [0u8; 256];
        network_passphrase.copy_into_slice(&mut np_buf[..np_len]);
        let np_str = core::str::from_utf8(&np_buf[..np_len]).unwrap_or("");

        const MAINNET: &str = "Public Global Stellar Network ; September 2015";
        const TESTNET: &str = "Test SDF Network ; September 2015";
        if np_str != MAINNET && np_str != TESTNET {
            panic_with_error!(&env, ErrorCode::ValidationError);
        }

        // Validate that the TOML's own network_passphrase matches the supplied one.
        let toml_np_len = toml_data.network_passphrase.len() as usize;
        if toml_np_len > 256 {
            panic_with_error!(&env, ErrorCode::ValidationError);
        }
        let mut toml_np_buf = [0u8; 256];
        toml_data.network_passphrase.copy_into_slice(&mut toml_np_buf[..toml_np_len]);
        let toml_np_str = core::str::from_utf8(&toml_np_buf[..toml_np_len]).unwrap_or("");
        if toml_np_str != np_str {
            panic_with_error!(&env, ErrorCode::ValidationError);
        }

        // Reject non-HTTPS endpoints to prevent MITM exposure of anchor metadata.
        let ts_len = toml_data.transfer_server.len() as usize;
        if ts_len > 2048 {
            panic_with_error!(&env, ErrorCode::InvalidEndpointFormat);
        }
        let mut ts_buf = [0u8; 2048];
        toml_data.transfer_server.copy_into_slice(&mut ts_buf[..ts_len]);
        let transfer_server_str = core::str::from_utf8(&ts_buf[..ts_len]).unwrap_or("");
        if crate::validate_anchor_domain(transfer_server_str).is_err() {
            panic_with_error!(&env, ErrorCode::InvalidEndpointFormat);
        }

        // Validate decimals for each currency: must be in range 0..=18.
        for asset in toml_data.currencies.iter() {
            if asset.decimals > 18 {
                panic_with_error!(&env, ErrorCode::ValidationError);
            }
        }

        let now = env.ledger().timestamp();
        let ttl_seconds = ttl_override.unwrap_or(3600);
        let cached = CachedToml {
            toml: toml_data,
            cached_at: now,
            ttl_seconds,
        };
        let key = StorageKey::TomlCache(anchor.clone());
        let ledger_ttl = if ttl_seconds as u32 > MIN_TEMP_TTL { ttl_seconds as u32 } else { MIN_TEMP_TTL };
        env.storage().temporary().set(&key, &cached);
        env.storage().temporary().extend_ttl(&key, ledger_ttl, ledger_ttl);

        Self::add_to_cached_anchors(&env, &anchor);
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
        let key = StorageKey::TomlCache(anchor.clone());
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

    /// Return the fiat currencies supported by `anchor` from its cached stellar.toml.
    /// Returns `Err(ErrorCode::CacheNotFound)` when no TOML has been cached for this anchor.
    pub fn get_anchor_currencies(
        env: Env,
        anchor: Address,
    ) -> Result<Vec<FiatCurrency>, ErrorCode> {
        let key = StorageKey::TomlCache(anchor.clone());
        if !env.storage().temporary().has(&key) {
            return Err(ErrorCode::CacheNotFound);
        }
        let toml = Self::get_anchor_toml(env.clone(), anchor);
        Ok(toml.fiat_currencies)
    }

    pub fn get_anchor_asset_info(env: Env, anchor: Address, asset_code: String) -> AssetInfo {
        let toml = Self::get_anchor_toml(env.clone(), anchor);
        for asset in toml.currencies.iter() {
            if asset.code == asset_code {
                return asset;
            }
        }
        panic_with_error!(&env, ErrorCode::ValidationError);
    }

    pub fn get_anchor_deposit_limits(env: Env, anchor: Address, asset_code: String) -> Result<(u64, u64), ErrorCode> {
        let key = StorageKey::TomlCache(anchor.clone());
        if !env.storage().temporary().has(&key) {
            return Err(ErrorCode::CacheNotFound);
        }
        let asset = Self::get_anchor_asset_info(env, anchor, asset_code);
        Ok((asset.deposit_min_amount, asset.deposit_max_amount))
    }

    pub fn get_anchor_withdrawal_limits(env: Env, anchor: Address, asset_code: String) -> Result<(u64, u64), ErrorCode> {
        let key = StorageKey::TomlCache(anchor.clone());
        if !env.storage().temporary().has(&key) {
            return Err(ErrorCode::CacheNotFound);
        }
        let asset = Self::get_anchor_asset_info(env, anchor, asset_code);
        Ok((asset.withdrawal_min_amount, asset.withdrawal_max_amount))
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
        let now = env.ledger().timestamp();
        // Read the configured replay window (default 300 s if not set).
        let window: u64 = env
            .storage()
            .instance()
            .get(&key_replay_window(env))
            .unwrap_or(300u64);
        let lower = now.saturating_sub(window);
on this r        if timestamp < lower || timestamp > now {
            panic_with_error!(env, ErrorCode::InvalidTimestamp);
        }
    }

    fn next_attestation_id(env: &Env) -> u64 {
        let inst = env.storage().instance();
        let ck = key_counter(env);
        let id: u64 = inst.get(&ck).unwrap_or(0u64);
        let next = id.saturating_add(1);
        if next == u64::MAX {
            panic_with_error!(env, ErrorCode::AttestationLimitReached);
        }
        inst.set(&ck, &next);
        inst.extend_ttl(INSTANCE_TTL, INSTANCE_TTL);
        id
    }

    fn check_session_expiry(env: &Env, session_id: u64) {
        let sess_key = StorageKey::Session(session_id);
        let session: Session = env
            .storage()
            .persistent()
            .get::<_, Session>(&sess_key)
            .unwrap_or_else(|| panic_with_error!(env, ErrorCode::SessionNotFound));
        let now = env.ledger().timestamp();
        if now >= session.expires_at {
            env.events().publish(
                (symbol_short!("session"), symbol_short!("expired"), session_id),
                SessionExpired { session_id, expired_at: now },
            );
            panic_with_error!(env, ErrorCode::SessionExpired);
        }
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
            issuer_revoked: false,
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

    /// Verifies that the attestation signature is valid for the given payload hash
    /// using any of the public keys registered for the issuer.
    ///
    /// # Panics
    ///
    /// Panics with `ErrorCode::UnauthorizedAttestor` if no valid signature is found.
    // Verifies that the attestation signature is valid for the given payload hash
// using any of the public keys registered for the issuer.
//
// # Panics
//
// Panics with `ErrorCode::UnauthorizedAttestor` if no valid signature is found.
fn verify_attestation_signature(
    env: &Env,
    issuer: &Address,
    payload_hash: &Bytes,
    signature: &Bytes,
) {
    // Retrieve the list of registered public keys for the issuer.
    let keys: Vec<Bytes> = env
        .storage()
        .persistent()
        .get(&StorageKey::Sep10Key(issuer.clone()))
        .unwrap_or_else(|| panic_with_error!(env, ErrorCode::UnauthorizedAttestor));

    // Convert signature to the fixed-size BytesN<64> expected by env.crypto().ed25519_verify.
    let sig_n: BytesN<64> = signature.clone().try_into().unwrap_or_else(|_| {
        panic_with_error!(env, ErrorCode::UnauthorizedAttestor)
    });

    // Attempt verification with each stored public key.
    for key in keys.iter() {
        if key.len() != 32 {
            continue; // Skip malformed keys.
        }
        // Convert the key to BytesN<32>.
        let pk_n: BytesN<32> = key.clone().try_into().unwrap();
        // Use the host environment's crypto verification.
        if env.crypto().ed25519_verify(&pk_n, payload_hash, &sig_n) {
            // Successful verification; exit the function.
            return;
        }
    }

    // If we reach this point, no key verified the signature.
    panic_with_error!(env, ErrorCode::UnauthorizedAttestor);
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

pub fn get_attestation_count(env: Env) -> u64 {
    AnchorKitContract::get_attestation_count(env)
}
