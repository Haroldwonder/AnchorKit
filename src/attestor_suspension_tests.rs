use super::*;
use crate::storage::{key_admin, StorageKey};
use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger, LedgerInfo},
    Address, Bytes, Env, String, Vec,
};

const NOW: u64 = 1_700_000_000;

fn setup(env: &Env) -> (Address, Address) {
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
    let admin = Address::generate(env);
    let attestor = Address::generate(env);
    env.storage().instance().set(&key_admin(env), &admin);
    env.storage()
        .persistent()
        .set(&StorageKey::Attestor(attestor.clone()), &true);
    (admin, attestor)
}

#[test]
fn suspend_preserves_registration_and_emits_event() {
    let env = Env::default();
    let (_admin, attestor) = setup(&env);
    let before = env.events().all().events().len();

    AnchorKitContract::suspend_attestor(env.clone(), attestor.clone());

    assert!(env.events().all().events().len() > before);

    assert!(AnchorKitContract::is_attestor(env.clone(), attestor.clone()));
    assert!(AnchorKitContract::is_suspended_attestor(env, attestor));
}

#[test]
fn unsuspend_clears_flag_and_emits_event() {
    let env = Env::default();
    let (_admin, attestor) = setup(&env);
    AnchorKitContract::suspend_attestor(env.clone(), attestor.clone());
    let before = env.events().all().events().len();

    AnchorKitContract::unsuspend_attestor(env.clone(), attestor.clone());

    assert!(env.events().all().events().len() > before);

    assert!(!AnchorKitContract::is_suspended_attestor(env, attestor));
}

#[test]
#[should_panic(expected = "AttestorSuspended")]
fn suspended_attestor_cannot_set_endpoint() {
    let env = Env::default();
    let (_, attestor) = setup(&env);
    AnchorKitContract::suspend_attestor(env.clone(), attestor.clone());
    let endpoint = String::from_str(&env, "https://example.com");
    AnchorKitContract::set_endpoint(env, attestor, endpoint);
}

#[test]
#[should_panic(expected = "AttestorSuspended")]
fn suspended_attestor_cannot_configure_services() {
    let env = Env::default();
    let (_, attestor) = setup(&env);
    AnchorKitContract::suspend_attestor(env.clone(), attestor.clone());
    let services = Vec::new(&env);
    AnchorKitContract::configure_services(env, attestor, services);
}

#[test]
#[should_panic(expected = "AttestorSuspended")]
fn suspended_attestor_cannot_submit() {
    let env = Env::default();
    let (_, attestor) = setup(&env);
    AnchorKitContract::suspend_attestor(env.clone(), attestor.clone());
    let subject = Address::generate(&env);
    let payload = Bytes::from_array(&env, &[1u8; 32]);
    let signature = Bytes::from_array(&env, &[0u8; 64]);
    AnchorKitContract::submit_attestation(env, attestor, subject, NOW, payload, signature);
}

#[test]
#[should_panic(expected = "AttestorSuspended")]
fn suspended_attestor_cannot_rotate_key() {
    let env = Env::default();
    let (_, attestor) = setup(&env);
    AnchorKitContract::suspend_attestor(env.clone(), attestor.clone());
    let new_key = Bytes::from_array(&env, &[2u8; 32]);
    AnchorKitContract::rotate_attestation_key(env, attestor, new_key, 10);
}
