use super::*;
use crate::storage::StorageKey;
use ed25519_dalek::{Signer, SigningKey};
use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger, LedgerInfo},
    Address, Bytes, Env,
};

const NOW: u64 = 1_700_000_000;

fn register_key(env: &Env, attestor: &Address, key: &[u8; 32]) -> Bytes {
    let bytes = Bytes::from_array(env, key);
    let keys = soroban_sdk::vec![env, bytes.clone()];
    env.storage()
        .persistent()
        .set(&StorageKey::AttestationKey(attestor.clone()), &keys);
    bytes
}

fn setup(env: &Env) -> Address {
    env.mock_all_auths();
    env.ledger().set(LedgerInfo {
        timestamp: NOW,
        protocol_version: 21,
        sequence_number: 100,
        network_id: Default::default(),
        base_reserve: 100,
        min_persistent_entry_ttl: 4096,
        min_temp_entry_ttl: 16,
        max_entry_ttl: 6_312_000,
    });
    let attestor = Address::generate(env);
    env.storage()
        .persistent()
        .set(&StorageKey::Attestor(attestor.clone()), &true);
    attestor
}

fn signature(env: &Env, key: &SigningKey, payload: &Bytes) -> Bytes {
    let mut data = [0u8; 32];
    payload.copy_into_slice(&mut data);
    let sig = key.sign(&data);
    Bytes::from_slice(env, &sig.to_bytes())
}

#[test]
fn rotation_accepts_old_and_new_keys_during_overlap() {
    let env = Env::default();
    let attestor = setup(&env);
    let old_signer = SigningKey::from_bytes(&[7u8; 32]);
    let new_signer = SigningKey::from_bytes(&[8u8; 32]);
    let old_key = register_key(&env, &attestor, old_signer.as_bytes());
    let new_key = Bytes::from_array(&env, new_signer.as_bytes());

    AnchorKitContract::rotate_attestation_key(env.clone(), attestor.clone(), new_key, 60);

    let payload = Bytes::from_array(&env, &[42u8; 32]);
    AnchorKitContract::verify_attestation_signature(
        &env,
        &attestor,
        &payload,
        &signature(&env, &old_signer, &payload),
    );
    AnchorKitContract::verify_attestation_signature(
        &env,
        &attestor,
        &payload,
        &signature(&env, &new_signer, &payload),
    );
    let keys: Vec<Bytes> = AnchorKitContract::get_attestation_keys(env, attestor);
    assert_eq!(keys.len(), 2);
    assert_eq!(keys.get(0).unwrap(), old_key);
}

#[test]
#[should_panic(expected = "UnauthorizedAttestor")]
fn old_key_rejected_when_overlap_expires() {
    let env = Env::default();
    let attestor = setup(&env);
    let old_signer = SigningKey::from_bytes(&[7u8; 32]);
    register_key(&env, &attestor, old_signer.as_bytes());

    let new_signer = SigningKey::from_bytes(&[9u8; 32]);
    AnchorKitContract::rotate_attestation_key(
        env.clone(),
        attestor.clone(),
        Bytes::from_array(&env, new_signer.as_bytes()),
        30,
    );
    env.ledger().set(LedgerInfo {
        timestamp: NOW + 31,
        ..env.ledger().get()
    });

    let payload = Bytes::from_array(&env, &[42u8; 32]);
    // New key remains valid after the overlap; only the old key must fail.
    AnchorKitContract::verify_attestation_signature(
        &env,
        &attestor,
        &payload,
        &signature(&env, &new_signer, &payload),
    );
    AnchorKitContract::verify_attestation_signature(
        &env,
        &attestor,
        &payload,
        &signature(&env, &old_signer, &payload),
    );
}

#[test]
#[should_panic(expected = "AttestorNotRegistered")]
fn rotation_requires_registered_attestor() {
    let env = Env::default();
    setup(&env);
    let other = Address::generate(&env);
    AnchorKitContract::rotate_attestation_key(
        env.clone(),
        other,
        Bytes::from_array(&env, &[9u8; 32]),
        10,
    );
}

#[test]
fn rotation_emits_event() {
    let env = Env::default();
    let attestor = setup(&env);
    let old_signer = SigningKey::from_bytes(&[7u8; 32]);
    let old_key = register_key(&env, &attestor, old_signer.as_bytes());
    let new_key = Bytes::from_array(&env, &[8u8; 32]);

    let keys: Vec<Bytes> = AnchorKitContract::get_attestation_keys(env.clone(), attestor.clone());
    assert_eq!(keys.get(0).unwrap(), old_key);
    let before = env.events().all().events().len();

    AnchorKitContract::rotate_attestation_key(env.clone(), attestor, new_key, 45);

    assert!(env.events().all().events().len() > before);
}
