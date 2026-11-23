use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Bytes, Env,
};
use ed25519_dalek::SigningKey;
use crate::contract::{AnchorKitContract, AnchorKitContractClient};
use crate::sep10_test_util::{register_attestor_with_sep10, sign_attestation};

#[test]
fn test_submit_attestation_happy_path() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set(LedgerInfo {
        timestamp: 1_700_000_000,
        protocol_version: 21,
        sequence_number: 100,
        network_id: Default::default(),
        base_reserve: 100,
        min_persistent_entry_ttl: 4096,
        min_temp_entry_ttl: 16,
        max_entry_ttl: 6_312_000,
    });

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    client.initialize(&admin, &100_u64, &None, &None);

    // Deterministic key material so the happy path registers a real SEP-10
    // verifying key and submits a valid attestation signature.
    let secret_bytes: [u8; 32] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
        17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
    ];
    let keypair = SigningKey::from_bytes(&secret_bytes);
    register_attestor_with_sep10(&env, &client, &issuer, &issuer, &keypair);

    let mut hash_data = [0u8; 32];
    hash_data[0] = 1;
    let payload_hash = Bytes::from_slice(&env, &hash_data);
    let timestamp = env.ledger().timestamp();
    let signature = sign_attestation(&env, &keypair, &client.address, &issuer, &subject, timestamp, &payload_hash);

    let id = client.submit_attestation(
        &issuer,
        &subject,
        &timestamp,
        &payload_hash,
        &signature,
    );

    assert_eq!(id, 0);
}

#[test]
#[should_panic(expected = "ValidationError")]
fn test_submit_attestation_empty_hash() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    // Empty hash
    let payload_hash = Bytes::new(&env);
    let signature = Bytes::from_slice(&env, &[0u8; 64]);
    let timestamp = 123456789;

    // Register issuer
    client.register_attestor(&issuer, &soroban_sdk::String::from_str(&env, "mock"), &Address::generate(&env));

    client.submit_attestation(&issuer, &subject, &timestamp, &payload_hash, &signature);
}

#[test]
#[should_panic(expected = "ValidationError")]
fn test_submit_attestation_short_hash() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    // 31-byte hash (one byte short)
    let payload_hash = Bytes::from_slice(&env, &[0u8; 31]);
    let signature = Bytes::from_slice(&env, &[0u8; 64]);
    let timestamp = 123456789;

    // Register issuer
    client.register_attestor(&issuer, &soroban_sdk::String::from_str(&env, "mock"), &Address::generate(&env));

    client.submit_attestation(&issuer, &subject, &timestamp, &payload_hash, &signature);
}

#[test]
fn test_submit_attestation_valid_signature() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    // Using a predictable hardcoded seed for deterministic tests
    let secret_bytes: [u8; 32] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
        17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
    ];
    let keypair = ed25519_dalek::SigningKey::from_bytes(&secret_bytes);
    let public_key = keypair.verifying_key().to_bytes();

    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);

    // Setup keys
    let pk_bytes = Bytes::from_slice(&env, &public_key);
    client.set_sep10_jwt_verifying_key(&issuer, &pk_bytes);
    client.add_attestation_key(&issuer, &pk_bytes);

    let mut hash_data = [0u8; 32];
    hash_data[0] = 42;
    let payload_hash = Bytes::from_slice(&env, &hash_data);
    
    // Sign the hash
    let timestamp = 123456789;
    let signature = crate::sep10_test_util::sign_attestation(
        &env, &keypair, &contract_id, &issuer, &subject, timestamp, &payload_hash,
    );

    // We do NOT need register_attestor here because the method checks authorization directly.
    // Wait, submit_attestation checks: verify_attestor_registered(&env, issuer);
    // So we DO need to register attestor!
    // But registering attestor checks the sep10 token.
    // In mock_all_auths(), we can bypass auth checks, but register_attestor validates the token.
    // If token validation fails, we can't register.
    // Alternatively, since mock_all_auths is on, can we skip registration?
    // The contract explicitly queries storage:
    // env.storage().persistent().get(&StorageKey::Attestor(attestor.clone())).unwrap_or(false)
    // So we must register! 
    // To register, we need a valid JWT token signed by `issuer`!
    // Actually, `verify_sep10_token_matches_attestor` checks the token.
    // Let's just create a valid JWT!
    // Or we can just call `env.storage().persistent().set(&StorageKey::Attestor(issuer.clone()), &true);` directly via `env.as_contract`.
    
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&crate::storage::StorageKey::Attestor(issuer.clone()), &true);
    });

    let id = client.submit_attestation(&issuer, &subject, &timestamp, &payload_hash, &signature);

    assert_eq!(id, 0);
}

#[test]
#[should_panic(expected = "UnauthorizedAttestor")]
fn test_submit_attestation_tampered_signature() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let secret_bytes: [u8; 32] = [2u8; 32];
    let keypair = ed25519_dalek::SigningKey::from_bytes(&secret_bytes);
    let public_key = keypair.verifying_key().to_bytes();

    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);

    let pk_bytes = Bytes::from_slice(&env, &public_key);
    client.set_sep10_jwt_verifying_key(&issuer, &pk_bytes);
    client.add_attestation_key(&issuer, &pk_bytes);

    let mut hash_data = [0u8; 32];
    hash_data[0] = 42;
    let payload_hash = Bytes::from_slice(&env, &hash_data);
    
    // Sign the hash
    use ed25519_dalek::Signer;
    let mut dalek_sig = keypair.sign(&hash_data).to_bytes();
    
    // TAMPER THE SIGNATURE
    dalek_sig[0] ^= 1; 
    
    let signature = Bytes::from_slice(&env, &dalek_sig);
    let timestamp = 123456789;

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&crate::storage::StorageKey::Attestor(issuer.clone()), &true);
    });

    client.submit_attestation(&issuer, &subject, &timestamp, &payload_hash, &signature);
}

// ---------------------------------------------------------------------------
// #1376: the signature covers issuer, subject, timestamp, contract and network
// ---------------------------------------------------------------------------

/// A registered issuer with a signing key on a fresh contract, ledger at t=1000.
fn signed_setup(
    env: &Env,
    seed: u8,
) -> (Address, AnchorKitContractClient<'_>, ed25519_dalek::SigningKey, Address, Bytes) {
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000);

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(env, &contract_id);
    let keypair = ed25519_dalek::SigningKey::from_bytes(&[seed; 32]);
    let issuer = Address::generate(env);
    client.set_sep10_jwt_verifying_key(&issuer, &Bytes::from_slice(env, &keypair.verifying_key().to_bytes()));
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&crate::storage::StorageKey::Attestor(issuer.clone()), &true);
    });
    (contract_id, client, keypair, issuer, Bytes::from_slice(env, &[7u8; 32]))
}

#[test]
fn test_attestation_signature_binds_subject_and_timestamp() {
    let env = Env::default();
    let (contract_id, client, keypair, issuer, payload_hash) = signed_setup(&env, 3);
    let subject = Address::generate(&env);
    let sig = crate::sep10_test_util::sign_attestation(&env, &keypair, &contract_id, &issuer, &subject, 900, &payload_hash);

    // Changing the subject or the timestamp invalidates the signature.
    let other_subject = Address::generate(&env);
    assert_eq!(
        client.try_submit_attestation(&issuer, &other_subject, &900, &payload_hash, &sig).unwrap_err().unwrap(),
        crate::errors::ErrorCode::UnauthorizedAttestor,
    );
    assert_eq!(
        client.try_submit_attestation(&issuer, &subject, &901, &payload_hash, &sig).unwrap_err().unwrap(),
        crate::errors::ErrorCode::UnauthorizedAttestor,
    );

    // The untouched submission verifies.
    assert_eq!(client.submit_attestation(&issuer, &subject, &900, &payload_hash, &sig), 0);
}

#[test]
fn test_attestation_signature_is_bound_to_the_contract() {
    let env = Env::default();
    let (contract_a, _client_a, keypair, issuer, payload_hash) = signed_setup(&env, 4);
    let subject = Address::generate(&env);
    let sig_for_a = crate::sep10_test_util::sign_attestation(&env, &keypair, &contract_a, &issuer, &subject, 900, &payload_hash);

    // A second deployment registering the same issuer key must not accept it.
    let contract_b = env.register_contract(None, AnchorKitContract);
    let client_b = AnchorKitContractClient::new(&env, &contract_b);
    client_b.set_sep10_jwt_verifying_key(&issuer, &Bytes::from_slice(&env, &keypair.verifying_key().to_bytes()));
    env.as_contract(&contract_b, || {
        env.storage().persistent().set(&crate::storage::StorageKey::Attestor(issuer.clone()), &true);
    });

    assert_eq!(
        client_b.try_submit_attestation(&issuer, &subject, &900, &payload_hash, &sig_for_a).unwrap_err().unwrap(),
        crate::errors::ErrorCode::UnauthorizedAttestor,
    );
}
