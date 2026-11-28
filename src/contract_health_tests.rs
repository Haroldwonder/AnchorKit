use super::*;
use crate::storage::{
    key_admin, key_attestor_count, key_session_counter, key_total_count, StorageKey,
};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn health_reports_defaults_before_initialization() {
    let env = Env::default();
    let health = AnchorKitContract::health(env.clone());
    assert!(!health.initialized);
    assert!(!health.paused);
    assert!(health.admin.is_none());
    assert_eq!(health.attestor_count, 0);
    assert_eq!(health.attestation_counter, 0);
    assert_eq!(health.session_counter, 0);
    assert_eq!(health.contract_version, String::from_str(&env, "0.1.0"));
}

#[test]
fn health_reports_current_state_and_counters() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let inst = env.storage().instance();
    inst.set(&key_admin(&env), &admin);
    inst.set(&StorageKey::IsPaused, &true);
    inst.set(&key_attestor_count(&env), &3u64);
    inst.set(&key_total_count(&env), &17u64);
    inst.set(&key_session_counter(&env), &5u64);

    let health = AnchorKitContract::health(env);
    assert!(health.initialized);
    assert!(health.paused);
    assert_eq!(health.admin, Some(admin));
    assert_eq!(health.attestor_count, 3);
    assert_eq!(health.attestation_counter, 17);
    assert_eq!(health.session_counter, 5);
}
