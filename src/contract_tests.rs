#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};
use crate::contract::{AnchorKitContract, AnchorKitContractClient};

fn setup() -> (Env, AnchorKitContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);
    (env, client)
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

#[test]
fn test_initialize_first_call_succeeds() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &100_u64, &None, &None);
    assert_eq!(client.get_admin(), admin);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_initialize_second_call_returns_already_initialized() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &100_u64, &None, &None);
    client.initialize(&admin, &100_u64, &None, &None);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_initialize_different_admin_fails() {
    let (env, client) = setup();
    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    client.initialize(&admin1, &100_u64, &None, &None);
    client.initialize(&admin2, &100_u64, &None, &None);
}

// ---------------------------------------------------------------------------
// Two-step admin transfer (#619)
// ---------------------------------------------------------------------------

#[test]
fn test_admin_transfer_full_flow() {
    let (env, client) = setup();
    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    client.initialize(&admin1, &100_u64, &None, &None);

    // propose: admin1 still in charge
    client.propose_admin(&admin2);
    assert_eq!(client.get_admin(), admin1);

    // accept: admin2 takes over
    client.propose_admin(&admin2);
    assert_eq!(client.get_admin(), admin1);
    client.accept_admin();
    assert_eq!(client.get_admin(), admin2);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #52)")]
fn test_pending_admin_already_exists() {
    let (env, client) = setup();
    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);
    client.initialize(&admin1, &100_u64, &None, &None);
    client.propose_admin(&admin2);
    // second propose while one is in flight must panic with UnauthorizedProposeAdmin (#52)
    client.propose_admin(&admin3);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #53)")]
fn test_accept_admin_with_no_pending() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &100_u64, &None, &None);
    // no propose — must panic with NoPendingAdmin
    client.accept_admin();
}

#[test]
fn test_admin_unchanged_after_propose_before_accept() {
    let (env, client) = setup();
    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    client.initialize(&admin1, &100_u64, &None, &None);
    client.propose_admin(&admin2);
    // admin must not change until accept
    assert_eq!(client.get_admin(), admin1);
}

#[test]
fn test_accept_no_pending() {
    let (env, client) = setup();
    let admin1 = Address::generate(&env);
    client.initialize(&admin1, &100_u64, &None, &None);
    client.accept_admin();
}
