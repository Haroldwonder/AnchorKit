use super::*;
use soroban_sdk::{testutils::{Address as _}, Address, Env, Bytes};
use crate::errors::ErrorCode;
use crate::contract::{AnchorKitContract, AnchorKitContractClient};

#[test]
fn test_submit_attestation_happy_path() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    // Valid 32-byte hash
    let mut hash_data = [0u8; 32];
    hash_data[0] = 1;
    let payload_hash = Bytes::from_slice(&env, &hash_data);
    let signature = Bytes::from_slice(&env, &[0u8; 64]);
    let timestamp = 123456789;

    // Register issuer
    client.register_attestor(&issuer, &soroban_sdk::String::from_str(&env, "mock"), &Address::generate(&env));

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

    client.submit_attestation(
        &issuer,
        &subject,
        &timestamp,
        &payload_hash,
        &signature,
    );
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

    client.submit_attestation(
        &issuer,
        &subject,
        &timestamp,
        &payload_hash,
        &signature,
    );
}
