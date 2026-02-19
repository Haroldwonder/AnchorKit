use soroban_sdk::{Address, BytesN, Env, IntoVal, String};

use crate::{types::{Attestation, Endpoint}, Error};

#[derive(Clone)]
enum StorageKey {
    Admin,
    Attestor(Address),
    Counter,
    Attestation(u64),
    UsedHash(BytesN<32>),
    Endpoint(Address),
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
}
