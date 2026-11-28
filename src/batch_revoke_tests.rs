#![cfg(test)]

//! Regression tests for #1512: batch revocation must mirror `revoke_attestation`
//! for every ID while bounding the number of IDs accepted in one call.

mod batch_revoke_tests {
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;
    use soroban_sdk::{
        testutils::{Address as _, Events as _, Ledger},
        Address, Bytes, Env, Vec as SorobanVec,
    };

    use crate::contract::{AnchorKitContract, AnchorKitContractClient, MAX_REVOKE_BATCH_SIZE};
    use crate::errors::ErrorCode;
    use crate::sep10_test_util::register_attestor_with_sep10;

    const TIMESTAMP: u64 = 1_000_000;

    fn make_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        // Keep the host's default ledger settings and only move the clock to
        // the timestamp used by the attestations below.
        env.ledger().with_mut(|info| info.timestamp = TIMESTAMP);
        env
    }

    fn setup(env: &Env) -> (AnchorKitContractClient<'_>, Address, SigningKey) {
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        // Admin is seeded at deployment time; write it directly so these tests
        // do not depend on the separate initialize call.
        env.as_contract(&contract_id, || {
            env.storage().instance().set(&crate::storage::key_admin(env), &admin);
        });
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let attestor = Address::generate(env);
        // Attestation keys are registered against the SEP-10 issuer address;
        // using the attestor itself keeps signature verification aligned with
        // `submit_attestation`.
        register_attestor_with_sep10(env, &client, &attestor, &attestor, &signing_key);
        (client, attestor, signing_key)
    }

    fn payload(env: &Env, seed: u8) -> Bytes {
        let mut b = Bytes::new(env);
        for i in 0..32u8 {
            b.push_back(seed.wrapping_add(i));
        }
        b
    }

    fn submit(env: &Env, client: &AnchorKitContractClient, attestor: &Address, signing_key: &SigningKey, seed: u8) -> u64 {
        let subject = Address::generate(env);
        let hash = payload(env, seed);
        let mut bytes = [0u8; 32];
        hash.copy_into_slice(&mut bytes);
        let signature = Bytes::from_slice(env, &signing_key.sign(&bytes).to_bytes());
        client.submit_attestation(attestor, &subject, &TIMESTAMP, &hash, &signature)
    }

    /// Number of contract events currently recorded by the test host.
    /// Revoking one attestation emits exactly one event (same topics/data as
    /// the single-attestation path), so a successful batch adds one event per ID.
    fn event_count(env: &Env) -> usize {
        env.events().all().events().len()
    }

    #[test]
    fn batch_revokes_every_id_and_emits_one_event_per_id() {
        let env = make_env();
        let (client, attestor, signing_key) = setup(&env);
        let id1 = submit(&env, &client, &attestor, &signing_key, 1);
        let id2 = submit(&env, &client, &attestor, &signing_key, 2);
        let id3 = submit(&env, &client, &attestor, &signing_key, 3);
        let ids = SorobanVec::from_array(&env, [id1, id2, id3]);

        client.revoke_attestations(&attestor, &ids);
        // Event inspection must happen before any further contract call: the
        // Soroban test host records events per top-level invocation.
        assert_eq!(event_count(&env), ids.len() as usize, "one event per ID");

        for id in [id1, id2, id3] {
            assert!(client.get_attestation(&id).is_none(), "attestation {id} must be revoked");
        }
    }

    #[test]
    fn batch_rejects_attestation_issued_by_another_attestor_without_partial_commit() {
        let env = make_env();
        let (client, attestor, signing_key) = setup(&env);
        let own_id = submit(&env, &client, &attestor, &signing_key, 10);

        let other_attestor = Address::generate(&env);
        let mut csprng = OsRng;
        let other_key = SigningKey::generate(&mut csprng);
        register_attestor_with_sep10(&env, &client, &other_attestor, &other_attestor, &other_key);
        let foreign_id = submit(&env, &client, &other_attestor, &other_key, 11);

        let ids = SorobanVec::from_array(&env, [own_id, foreign_id]);
        let err = client.try_revoke_attestations(&attestor, &ids).unwrap_err().unwrap();
        assert_eq!(
            err,
            soroban_sdk::Error::from_contract_error(ErrorCode::UnauthorizedAttestor as u32)
        );
        assert_eq!(event_count(&env), 0, "failed batch emits no events");
        assert!(client.get_attestation(&own_id).is_some(), "no partial revocation");
    }

    #[test]
    fn batch_rejects_unknown_id_without_partial_commit() {
        let env = make_env();
        let (client, attestor, signing_key) = setup(&env);
        let own_id = submit(&env, &client, &attestor, &signing_key, 20);

        let ids = SorobanVec::from_array(&env, [own_id, u64::MAX]);
        let err = client.try_revoke_attestations(&attestor, &ids).unwrap_err().unwrap();
        assert_eq!(
            err,
            soroban_sdk::Error::from_contract_error(ErrorCode::AttestationNotFound as u32)
        );
        assert!(client.get_attestation(&own_id).is_some(), "no partial revocation");
    }

    #[test]
    fn batch_over_max_size_is_rejected() {
        let env = make_env();
        let (client, attestor, _signing_key) = setup(&env);

        let mut ids = SorobanVec::new(&env);
        for id in 0..=MAX_REVOKE_BATCH_SIZE {
            ids.push_back(id as u64);
        }
        assert_eq!(ids.len(), MAX_REVOKE_BATCH_SIZE + 1);

        let err = client.try_revoke_attestations(&attestor, &ids).unwrap_err().unwrap();
        assert_eq!(
            err,
            soroban_sdk::Error::from_contract_error(ErrorCode::ValidationError as u32)
        );
    }

    #[test]
    fn batch_at_max_size_is_accepted() {
        let env = make_env();
        let (client, attestor, signing_key) = setup(&env);
        let id = submit(&env, &client, &attestor, &signing_key, 30);

        let mut ids = SorobanVec::new(&env);
        for _ in 0..MAX_REVOKE_BATCH_SIZE {
            ids.push_back(id);
        }

        client.revoke_attestations(&attestor, &ids);
        assert!(client.get_attestation(&id).is_none());
    }

    #[test]
    fn batch_rejects_when_contract_is_paused() {
        let env = make_env();
        let (client, attestor, signing_key) = setup(&env);
        let id = submit(&env, &client, &attestor, &signing_key, 40);
        client.pause_contract();

        let ids = SorobanVec::from_array(&env, [id]);
        let err = client.try_revoke_attestations(&attestor, &ids).unwrap_err().unwrap();
        assert_eq!(
            err,
            soroban_sdk::Error::from_contract_error(ErrorCode::ContractPaused as u32)
        );
        assert!(client.get_attestation(&id).is_some(), "no revocation while paused");
    }

    #[test]
    fn empty_batch_is_a_no_op() {
        let env = make_env();
        let (client, attestor, signing_key) = setup(&env);
        let id = submit(&env, &client, &attestor, &signing_key, 50);

        client.revoke_attestations(&attestor, &SorobanVec::<u64>::new(&env));

        assert!(client.get_attestation(&id).is_some());
        assert_eq!(event_count(&env), 0);
    }
}
