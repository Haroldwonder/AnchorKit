#![cfg(test)]

//! Tests for explicit session closure (#1428).

use soroban_sdk::{
    testutils::{Address as _, Events, Ledger, LedgerInfo},
    Address, Env,
};

use crate::contract::{AnchorKitContract, AnchorKitContractClient};
use crate::errors::ErrorCode;

fn setup() -> (Env, AnchorKitContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set(LedgerInfo {
        timestamp: 1_000_000,
        protocol_version: 21,
        sequence_number: 0,
        network_id: Default::default(),
        base_reserve: 0,
        min_persistent_entry_ttl: 4096,
        min_temp_entry_ttl: 16,
        max_entry_ttl: 6_312_000,
    });
    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &100_u64, &None, &None);
    (env, client, admin)
}

#[test]
fn initiator_can_close_session_and_further_ops_are_rejected() {
    let (env, client, admin) = setup();
    let sid = client.create_session(&admin);
    let before = env.events().all().len();
    client.close_session(&sid, &admin);
    assert!(env.events().all().len() > before, "SessionClosed event must be emitted");

    let session = client.get_session(&sid);
    assert_eq!(session.expires_at, env.ledger().timestamp());

    let attestor = Address::generate(&env);
    let res = client.try_revoke_attestor_with_session(&sid, &attestor);
    assert_eq!(res, Err(Ok(ErrorCode::SessionExpired)));
}

#[test]
fn admin_can_close_another_initiators_session() {
    let (env, client, admin) = setup();
    let user = Address::generate(&env);
    let sid = client.create_session(&user);
    client.close_session(&sid, &admin);
    assert_eq!(client.get_session(&sid).expires_at, env.ledger().timestamp());
}

#[test]
fn stranger_cannot_close_session() {
    let (env, client, _admin) = setup();
    let user = Address::generate(&env);
    let sid = client.create_session(&user);
    let stranger = Address::generate(&env);
    let res = client.try_close_session(&sid, &stranger);
    assert_eq!(res, Err(Ok(ErrorCode::UnauthorizedSession)));
}

#[test]
fn closing_twice_fails_with_session_expired() {
    let (_env, client, admin) = setup();
    let sid = client.create_session(&admin);
    client.close_session(&sid, &admin);
    let res = client.try_close_session(&sid, &admin);
    assert_eq!(res, Err(Ok(ErrorCode::SessionExpired)));
}

#[test]
fn closing_unknown_session_fails() {
    let (_env, client, admin) = setup();
    let res = client.try_close_session(&999_u64, &admin);
    assert_eq!(res, Err(Ok(ErrorCode::SessionNotFound)));
}
