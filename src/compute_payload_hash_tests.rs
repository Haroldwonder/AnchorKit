use crate::contract::AnchorKitContract;
use crate::deterministic_hash;
use soroban_sdk::{testutils::Address as _, Address, Env, Bytes};

#[test]
fn test_compute_payload_hash_public_matches_internal() {
    let env = Env::default();
    let subject = Address::generate(&env);
    let data = Bytes::from_slice(&env, b"kyc_approved");
    let timestamp: u64 = 1_700_000_000u64;

    let public_hash =
        AnchorKitContract::compute_payload_hash_public(env.clone(), subject.clone(), timestamp, data.clone());
    let internal_hash = deterministic_hash::compute_payload_hash(&env, &subject, timestamp, &data);

    assert_eq!(public_hash, internal_hash);
}

#[test]
fn test_compute_payload_hash_public_backward_compat() {
    // There is now only one contract entry point (compute_payload_hash_public).
    // Verify it remains deterministic across identical calls so that any
    // off-chain tooling that previously relied on the old compute_payload_hash
    // name produces the same digest after migrating to compute_payload_hash_public.
    let env = Env::default();
    let subject = Address::generate(&env);
    let data = Bytes::from_slice(&env, b"payment_confirmed");
    let timestamp: u64 = 1_700_000_001u64;

    let hash1 =
        AnchorKitContract::compute_payload_hash_public(env.clone(), subject.clone(), timestamp, data.clone());
    let hash2 =
        AnchorKitContract::compute_payload_hash_public(env, subject, timestamp, data);

    assert_eq!(hash1, hash2);
}

#[test]
fn test_deterministic_same_inputs() {
    let env = Env::default();
    let subject = Address::generate(&env);
    let data = Bytes::from_slice(&env, b"test_data");
    let timestamp: u64 = 1_700_000_000u64;

    let h1 = AnchorKitContract::compute_payload_hash_public(env.clone(), subject.clone(), timestamp, data.clone());
    let h2 = AnchorKitContract::compute_payload_hash_public(env, subject, timestamp, data);

    assert_eq!(h1, h2);
}
