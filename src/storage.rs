use soroban_sdk::{Address, BytesN, Env, IntoVal};

use crate::{
    types::{
        AnchorMetadata, AnchorServices, Attestation, AuditLog, Endpoint, InteractionSession,
        OperationContext, QuoteData,
    },
    Error,
};

#[derive(Clone)]
enum StorageKey {
    Admin,
    Attestor(Address),
    Counter,
    Attestation(u64),
    UsedHash(BytesN<32>),
    Endpoint(Address),
    AnchorServices(Address),
    Quote(Address, u64),
    QuoteCounter,
    IntentCounter,
    SessionCounter,
    Session(u64),
    SessionNonce(u64),
    AuditLogCounter,
    AuditLog(u64),
    SessionOperationCount(u64),
    AnchorMetadata(Address),
    AnchorList,
}

impl StorageKey {
    fn to_storage_key(&self, env: &Env) -> soroban_sdk::Val {
        match self {
            StorageKey::Admin => (soroban_sdk::symbol_short!("ADMIN"),).into_val(env),
            StorageKey::Attestor(addr) => (soroban_sdk::symbol_short!("ATTESTOR"), addr).into_val(env),
            StorageKey::Counter => (soroban_sdk::symbol_short!("COUNTER"),).into_val(env),
            StorageKey::Attestation(id) => (soroban_sdk::symbol_short!("ATTEST"), *id).into_val(env),
            StorageKey::UsedHash(hash) => {
                (soroban_sdk::symbol_short!("USED"), hash.clone()).into_val(env)
            }
            StorageKey::Endpoint(addr) => (soroban_sdk::symbol_short!("ENDPOINT"), addr).into_val(env),
            StorageKey::AnchorServices(addr) => {
                (soroban_sdk::symbol_short!("SERVICES"), addr).into_val(env)
            }
            StorageKey::Quote(addr, id) => (soroban_sdk::symbol_short!("QUOTE"), addr, *id).into_val(env),
            StorageKey::QuoteCounter => (soroban_sdk::symbol_short!("QCNT"),).into_val(env),
            StorageKey::IntentCounter => (soroban_sdk::symbol_short!("ICNT"),).into_val(env),
            StorageKey::SessionCounter => (soroban_sdk::symbol_short!("SCNT"),).into_val(env),
            StorageKey::Session(id) => (soroban_sdk::symbol_short!("SESS"), *id).into_val(env),
            StorageKey::SessionNonce(id) => (soroban_sdk::symbol_short!("SNONCE"), *id).into_val(env),
            StorageKey::AuditLogCounter => (soroban_sdk::symbol_short!("ACNT"),).into_val(env),
            StorageKey::AuditLog(id) => (soroban_sdk::symbol_short!("AUDIT"), *id).into_val(env),
            StorageKey::SessionOperationCount(id) => {
                (soroban_sdk::symbol_short!("SOPCNT"), *id).into_val(env)
            }
            StorageKey::AnchorMetadata(addr) => {
                (soroban_sdk::symbol_short!("ANCHMETA"), addr).into_val(env)
            }
            StorageKey::AnchorList => (soroban_sdk::symbol_short!("ANCHLIST"),).into_val(env),
        }
    }
}

pub struct Storage;

impl Storage {
    const DAY_IN_LEDGERS: u32 = 17280;
    const INSTANCE_LIFETIME: u32 = Self::DAY_IN_LEDGERS * 30;
    const PERSISTENT_LIFETIME: u32 = Self::DAY_IN_LEDGERS * 90;

    pub fn has_admin(env: &Env) -> bool {
        let key = StorageKey::Admin.to_storage_key(env);
        env.storage().instance().has(&key)
    }

    pub fn set_admin(env: &Env, admin: &Address) {
        let key = StorageKey::Admin.to_storage_key(env);
        env.storage().instance().set(&key, admin);
        env.storage()
            .instance()
            .extend_ttl(Self::INSTANCE_LIFETIME, Self::INSTANCE_LIFETIME);
    }

    pub fn get_admin(env: &Env) -> Result<Address, Error> {
        let key = StorageKey::Admin.to_storage_key(env);
        env.storage().instance().get(&key).ok_or(Error::NotInitialized)
    }

    pub fn set_attestor(env: &Env, attestor: &Address, is_registered: bool) {
        let key = StorageKey::Attestor(attestor.clone()).to_storage_key(env);
        env.storage().persistent().set(&key, &is_registered);
        env.storage().persistent().extend_ttl(
            &key,
            Self::PERSISTENT_LIFETIME,
            Self::PERSISTENT_LIFETIME,
        );
    }

    pub fn is_attestor(env: &Env, attestor: &Address) -> bool {
        let key = StorageKey::Attestor(attestor.clone()).to_storage_key(env);
        env.storage().persistent().get(&key).unwrap_or(false)
    }

    pub fn get_and_increment_counter(env: &Env) -> u64 {
        let key = StorageKey::Counter.to_storage_key(env);
        let counter: u64 = env.storage().instance().get(&key).unwrap_or(0);
        env.storage().instance().set(&key, &(counter + 1));
        env.storage()
            .instance()
            .extend_ttl(Self::INSTANCE_LIFETIME, Self::INSTANCE_LIFETIME);
        counter
    }

    pub fn set_attestation(env: &Env, id: u64, attestation: &Attestation) {
        let key = StorageKey::Attestation(id).to_storage_key(env);
        env.storage().persistent().set(&key, attestation);
        env.storage().persistent().extend_ttl(
            &key,
            Self::PERSISTENT_LIFETIME,
            Self::PERSISTENT_LIFETIME,
        );
    }

    pub fn get_attestation(env: &Env, id: u64) -> Result<Attestation, Error> {
        let key = StorageKey::Attestation(id).to_storage_key(env);
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::AttestationNotFound)
    }

    pub fn mark_hash_used(env: &Env, hash: &BytesN<32>) {
        let key = StorageKey::UsedHash(hash.clone()).to_storage_key(env);
        env.storage().persistent().set(&key, &true);
        env.storage().persistent().extend_ttl(
            &key,
            Self::PERSISTENT_LIFETIME,
            Self::PERSISTENT_LIFETIME,
        );
    }

    pub fn is_hash_used(env: &Env, hash: &BytesN<32>) -> bool {
        let key = StorageKey::UsedHash(hash.clone()).to_storage_key(env);
        env.storage().persistent().get(&key).unwrap_or(false)
    }

    pub fn set_endpoint(env: &Env, endpoint: &Endpoint) {
        let key = StorageKey::Endpoint(endpoint.attestor.clone()).to_storage_key(env);
        env.storage().persistent().set(&key, endpoint);
        env.storage().persistent().extend_ttl(
            &key,
            Self::PERSISTENT_LIFETIME,
            Self::PERSISTENT_LIFETIME,
        );
    }

    pub fn get_endpoint(env: &Env, attestor: &Address) -> Result<Endpoint, Error> {
        let key = StorageKey::Endpoint(attestor.clone()).to_storage_key(env);
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::EndpointNotFound)
    }

    pub fn has_endpoint(env: &Env, attestor: &Address) -> bool {
        let key = StorageKey::Endpoint(attestor.clone()).to_storage_key(env);
        env.storage().persistent().has(&key)
    }

    pub fn remove_endpoint(env: &Env, attestor: &Address) {
        let key = StorageKey::Endpoint(attestor.clone()).to_storage_key(env);
        env.storage().persistent().remove(&key);
    }

    pub fn set_anchor_services(env: &Env, services: &AnchorServices) {
        let key = StorageKey::AnchorServices(services.anchor.clone()).to_storage_key(env);
        env.storage().persistent().set(&key, services);
        env.storage().persistent().extend_ttl(
            &key,
            Self::PERSISTENT_LIFETIME,
            Self::PERSISTENT_LIFETIME,
        );
    }

    pub fn get_anchor_services(env: &Env, anchor: &Address) -> Result<AnchorServices, Error> {
        let key = StorageKey::AnchorServices(anchor.clone()).to_storage_key(env);
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::ServicesNotConfigured)
    }

    pub fn set_quote(env: &Env, quote: &QuoteData) {
        let key = StorageKey::Quote(quote.anchor.clone(), quote.quote_id).to_storage_key(env);
        env.storage().persistent().set(&key, quote);
        env.storage().persistent().extend_ttl(
            &key,
            Self::PERSISTENT_LIFETIME,
            Self::PERSISTENT_LIFETIME,
        );
    }

    pub fn get_quote(env: &Env, anchor: &Address, quote_id: u64) -> Option<QuoteData> {
        let key = StorageKey::Quote(anchor.clone(), quote_id).to_storage_key(env);
        env.storage().persistent().get(&key)
    }

    pub fn get_next_quote_id(env: &Env) -> u64 {
        let key = StorageKey::QuoteCounter.to_storage_key(env);
        let current: u64 = env.storage().instance().get(&key).unwrap_or(0);
        let next = current + 1;
        env.storage().instance().set(&key, &next);
        env.storage()
            .instance()
            .extend_ttl(Self::INSTANCE_LIFETIME, Self::INSTANCE_LIFETIME);
        next
    }

    pub fn get_next_intent_id(env: &Env) -> u64 {
        let key = StorageKey::IntentCounter.to_storage_key(env);
        let current: u64 = env.storage().instance().get(&key).unwrap_or(0);
        let next = current + 1;
        env.storage().instance().set(&key, &next);
        env.storage()
            .instance()
            .extend_ttl(Self::INSTANCE_LIFETIME, Self::INSTANCE_LIFETIME);
        next
    }

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

        let key = StorageKey::Session(session_id).to_storage_key(env);
        env.storage().persistent().set(&key, &session);
        env.storage().persistent().extend_ttl(
            &key,
            Self::PERSISTENT_LIFETIME,
            Self::PERSISTENT_LIFETIME,
        );

        let nonce_key = StorageKey::SessionNonce(session_id).to_storage_key(env);
        env.storage().persistent().set(&nonce_key, &nonce);
        env.storage().persistent().extend_ttl(
            &nonce_key,
            Self::PERSISTENT_LIFETIME,
            Self::PERSISTENT_LIFETIME,
        );

        session_id
    }

    pub fn get_session(env: &Env, session_id: u64) -> Result<InteractionSession, Error> {
        let key = StorageKey::Session(session_id).to_storage_key(env);
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::SessionNotFound)
    }

    pub fn increment_session_operation_count(env: &Env, session_id: u64) -> u64 {
        let key = StorageKey::SessionOperationCount(session_id).to_storage_key(env);
        let count: u64 = env.storage().persistent().get(&key).unwrap_or(0);
        env.storage().persistent().set(&key, &(count + 1));
        env.storage().persistent().extend_ttl(
            &key,
            Self::PERSISTENT_LIFETIME,
            Self::PERSISTENT_LIFETIME,
        );
        count
    }

    pub fn get_session_operation_count(env: &Env, session_id: u64) -> u64 {
        let key = StorageKey::SessionOperationCount(session_id).to_storage_key(env);
        env.storage().persistent().get(&key).unwrap_or(0)
    }

    pub fn verify_session_nonce(env: &Env, session_id: u64, nonce: u64) -> Result<(), Error> {
        let key = StorageKey::SessionNonce(session_id).to_storage_key(env);
        let stored_nonce: u64 = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::SessionNotFound)?;

        if stored_nonce != nonce {
            return Err(Error::SessionReplayAttack);
        }
        Ok(())
    }

    fn get_and_increment_session_counter(env: &Env) -> u64 {
        let key = StorageKey::SessionCounter.to_storage_key(env);
        let counter: u64 = env.storage().instance().get(&key).unwrap_or(0);
        env.storage().instance().set(&key, &(counter + 1));
        env.storage()
            .instance()
            .extend_ttl(Self::INSTANCE_LIFETIME, Self::INSTANCE_LIFETIME);
        counter
    }

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

        let key = StorageKey::AuditLog(log_id).to_storage_key(env);
        env.storage().persistent().set(&key, &audit_log);
        env.storage().persistent().extend_ttl(
            &key,
            Self::PERSISTENT_LIFETIME,
            Self::PERSISTENT_LIFETIME,
        );

        log_id
    }

    pub fn get_audit_log(env: &Env, log_id: u64) -> Result<AuditLog, Error> {
        let key = StorageKey::AuditLog(log_id).to_storage_key(env);
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::SessionNotFound)
    }

    fn get_and_increment_audit_counter(env: &Env) -> u64 {
        let key = StorageKey::AuditLogCounter.to_storage_key(env);
        let counter: u64 = env.storage().instance().get(&key).unwrap_or(0);
        env.storage().instance().set(&key, &(counter + 1));
        env.storage()
            .instance()
            .extend_ttl(Self::INSTANCE_LIFETIME, Self::INSTANCE_LIFETIME);
        counter
    }

    // ============ Multi-Anchor Routing ============

    pub fn set_anchor_metadata(env: &Env, metadata: &AnchorMetadata) {
        let key = StorageKey::AnchorMetadata(metadata.anchor.clone()).to_storage_key(env);
        env.storage().persistent().set(&key, metadata);
        env.storage().persistent().extend_ttl(
            &key,
            Self::PERSISTENT_LIFETIME,
            Self::PERSISTENT_LIFETIME,
        );
    }

    pub fn get_anchor_metadata(env: &Env, anchor: &Address) -> Option<AnchorMetadata> {
        let key = StorageKey::AnchorMetadata(anchor.clone()).to_storage_key(env);
        env.storage().persistent().get(&key)
    }

    pub fn set_anchor_list(env: &Env, anchors: &soroban_sdk::Vec<Address>) {
        let key = StorageKey::AnchorList.to_storage_key(env);
        env.storage().persistent().set(&key, anchors);
        env.storage().persistent().extend_ttl(
            &key,
            Self::PERSISTENT_LIFETIME,
            Self::PERSISTENT_LIFETIME,
        );
    }

    pub fn get_anchor_list(env: &Env) -> soroban_sdk::Vec<Address> {
        let key = StorageKey::AnchorList.to_storage_key(env);
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(soroban_sdk::Vec::new(env))
    }

    pub fn add_to_anchor_list(env: &Env, anchor: &Address) {
        let mut anchors = Self::get_anchor_list(env);
        if !anchors.contains(anchor) {
            anchors.push_back(anchor.clone());
            Self::set_anchor_list(env, &anchors);
        }
    }

    pub fn remove_from_anchor_list(env: &Env, anchor: &Address) {
        let anchors = Self::get_anchor_list(env);
        let mut new_anchors = soroban_sdk::Vec::new(env);
        
        for i in 0..anchors.len() {
            let a = anchors.get(i).unwrap();
            if a != *anchor {
                new_anchors.push_back(a);
            }
        }
        
        Self::set_anchor_list(env, &new_anchors);
    }
}
