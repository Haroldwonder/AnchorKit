use soroban_sdk::{Address, BytesN, Env, IntoVal};

use crate::{types::{Attestation, Endpoint, InteractionSession, OperationContext, AuditLog}, Error};

#[derive(Clone)]
enum StorageKey {
    Admin,
    Attestor(Address),
    Counter,
    Attestation(u64),
    UsedHash(BytesN<32>),
    Endpoint(Address),
    SessionCounter,
    Session(u64),
    SessionNonce(u64),
    AuditLogCounter,
    AuditLog(u64),
    SessionOperationCount(u64),
}

impl StorageKey {
    fn to_storage_key(&self, env: &Env) -> soroban_sdk::Val {
        match self {
            StorageKey::Admin => (soroban_sdk::symbol_short!("ADMIN"),).into_val(env),
            StorageKey::Attestor(addr) => {
                (soroban_sdk::symbol_short!("ATTESTOR"), addr).into_val(env)
            }
            StorageKey::Counter => (soroban_sdk::symbol_short!("COUNTER"),).into_val(env),
            StorageKey::Attestation(id) => {
                (soroban_sdk::symbol_short!("ATTEST"), *id).into_val(env)
            }
            StorageKey::UsedHash(hash) => {
                (soroban_sdk::symbol_short!("USED"), hash.clone()).into_val(env)
            }
            StorageKey::Endpoint(addr) => {
                (soroban_sdk::symbol_short!("ENDPOINT"), addr).into_val(env)
            }
            StorageKey::SessionCounter => (soroban_sdk::symbol_short!("SCNT"),).into_val(env),
            StorageKey::Session(id) => {
                (soroban_sdk::symbol_short!("SESS"), *id).into_val(env)
            }
            StorageKey::SessionNonce(id) => {
                (soroban_sdk::symbol_short!("SNONCE"), *id).into_val(env)
            }
            StorageKey::AuditLogCounter => (soroban_sdk::symbol_short!("ACNT"),).into_val(env),
            StorageKey::AuditLog(id) => {
                (soroban_sdk::symbol_short!("AUDIT"), *id).into_val(env)
            }
            StorageKey::SessionOperationCount(id) => {
                (soroban_sdk::symbol_short!("SOPCNT"), *id).into_val(env)
            }
        }
    }
}

pub struct Storage;

impl Storage {
    const DAY_IN_LEDGERS: u32 = 17280;
    const INSTANCE_LIFETIME: u32 = Self::DAY_IN_LEDGERS * 30; // 30 days
    const PERSISTENT_LIFETIME: u32 = Self::DAY_IN_LEDGERS * 90; // 90 days

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
        env.storage()
            .instance()
            .get(&key)
            .ok_or(Error::NotInitialized)
    }

    pub fn set_attestor(env: &Env, attestor: &Address, is_registered: bool) {
        let key = StorageKey::Attestor(attestor.clone()).to_storage_key(env);
        env.storage().persistent().set(&key, &is_registered);
        env.storage()
            .persistent()
            .extend_ttl(&key, Self::PERSISTENT_LIFETIME, Self::PERSISTENT_LIFETIME);
    }

    pub fn is_attestor(env: &Env, attestor: &Address) -> bool {
        let key = StorageKey::Attestor(attestor.clone()).to_storage_key(env);
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(false)
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
        env.storage()
            .persistent()
            .extend_ttl(&key, Self::PERSISTENT_LIFETIME, Self::PERSISTENT_LIFETIME);
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
        env.storage()
            .persistent()
            .extend_ttl(&key, Self::PERSISTENT_LIFETIME, Self::PERSISTENT_LIFETIME);
    }

    pub fn is_hash_used(env: &Env, hash: &BytesN<32>) -> bool {
        let key = StorageKey::UsedHash(hash.clone()).to_storage_key(env);
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(false)
    }

    pub fn set_endpoint(env: &Env, endpoint: &Endpoint) {
        let key = StorageKey::Endpoint(endpoint.attestor.clone()).to_storage_key(env);
        env.storage().persistent().set(&key, endpoint);
        env.storage()
            .persistent()
            .extend_ttl(&key, Self::PERSISTENT_LIFETIME, Self::PERSISTENT_LIFETIME);
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

    // ============ Session Management ============

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
        env.storage()
            .persistent()
            .extend_ttl(&key, Self::PERSISTENT_LIFETIME, Self::PERSISTENT_LIFETIME);

        // Store nonce for replay protection
        let nonce_key = StorageKey::SessionNonce(session_id).to_storage_key(env);
        env.storage().persistent().set(&nonce_key, &nonce);
        env.storage()
            .persistent()
            .extend_ttl(&nonce_key, Self::PERSISTENT_LIFETIME, Self::PERSISTENT_LIFETIME);

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
        env.storage()
            .persistent()
            .extend_ttl(&key, Self::PERSISTENT_LIFETIME, Self::PERSISTENT_LIFETIME);
        count
    }

    pub fn get_session_operation_count(env: &Env, session_id: u64) -> u64 {
        let key = StorageKey::SessionOperationCount(session_id).to_storage_key(env);
        env.storage().persistent().get(&key).unwrap_or(0)
    }

    pub fn verify_session_nonce(env: &Env, session_id: u64, nonce: u64) -> Result<(), Error> {
        let key = StorageKey::SessionNonce(session_id).to_storage_key(env);
        let stored_nonce: u64 = env.storage()
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

    // ============ Audit Logging ============

    pub fn log_operation(env: &Env, session_id: u64, actor: &Address, operation: &OperationContext) -> u64 {
        let log_id = Self::get_and_increment_audit_counter(env);
        
        let audit_log = AuditLog {
            log_id,
            session_id,
            operation: operation.clone(),
            actor: actor.clone(),
        };

        let key = StorageKey::AuditLog(log_id).to_storage_key(env);
        env.storage().persistent().set(&key, &audit_log);
        env.storage()
            .persistent()
            .extend_ttl(&key, Self::PERSISTENT_LIFETIME, Self::PERSISTENT_LIFETIME);

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
}
