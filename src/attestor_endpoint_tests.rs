#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

use crate::contract::{AnchorKitContract, AnchorKitContractClient};
use crate::sep10_test_util::register_attestor_with_sep10;

fn make_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn setup(env: &Env) -> (AnchorKitContractClient, Address, Address, SigningKey) {
    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let attestor = Address::generate(env);
    client.initialize(&admin);
    let sk = SigningKey::generate(&mut OsRng);
    register_attestor_with_sep10(env, &client, &attestor, &admin, &sk);
    (client, admin, attestor, sk)
}

#[test]
fn test_set_get_endpoint_happy_path() {
    let env = make_env();
    let (client, _, attestor, _) = setup(&env);
    let endpoint = String::from_str(&env, "https://example.com/api");
    client.set_endpoint(&attestor, &endpoint);
    assert_eq!(client.get_endpoint(&attestor), endpoint);
}

#[test]
#[should_panic]
fn test_get_endpoint_not_registered() {
    let env = make_env();
    let (client, _, _, _) = setup(&env);
    let unknown = Address::generate(&env);
    client.get_endpoint(&unknown);
}

#[test]
#[should_panic]
fn test_set_endpoint_not_attestor() {
    let env = make_env();
    let (client, _, _, _) = setup(&env);
    let unknown = Address::generate(&env);
    let endpoint = String::from_str(&env, "https://example.com");
    client.set_endpoint(&unknown, &endpoint);
}

#[test]
#[should_panic]
fn test_set_endpoint_invalid_url() {
    let env = make_env();
    let (client, _, attestor, _) = setup(&env);
    let invalid = String::from_str(&env, "http://invalid.com");
    client.set_endpoint(&attestor, &invalid);
}
