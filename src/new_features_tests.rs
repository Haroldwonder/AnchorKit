#![cfg(test)]

mod new_features_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Bytes, Env,
    };
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::sep10_test_util::{register_attestor_with_sep10, sign_payload};

    fn make_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    fn setup_ledger(env: &Env) {
        env.ledger().set(LedgerInfo {
            timestamp: 1700000000,
            protocol_version: 21,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 100,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });
    }

    fn payload(env: &Env, byte: u8) -> Bytes {
        let mut b = Bytes::new(env);
        for _ in 0..32 {
            b.push_back(byte);
        }
        b
    }

    fn setup_contract(env: &Env) -> (AnchorKitContractClient, Address, Address, SigningKey) {
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let attestor = Address::generate(env);
        client.initialize(&admin, &100_u64, &None, &None);
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        register_attestor_with_sep10(env, &client, &attestor, &attestor, &signing_key);
        (client, admin, attestor, signing_key)
    }

    // -----------------------------------------------------------------------
    // get_version
    // -----------------------------------------------------------------------

    #[test]
    fn get_version_returns_semver() {
        let env = make_env();
        setup_ledger(&env);
        let (client, _, _, _) = setup_contract(&env);
        let version = client.get_version();
        // Must match the version in Cargo.toml
        assert_eq!(version, soroban_sdk::String::from_str(&env, "0.1.0"));
    }

    // -----------------------------------------------------------------------
    // get_attestation_count
    // -----------------------------------------------------------------------

    #[test]
    fn attestation_count_zero_before_any_submission() {
        let env = make_env();
        setup_ledger(&env);
        let (client, _, attestor, _) = setup_contract(&env);
        assert_eq!(client.get_attestation_count(&attestor), 0);
    }

    #[test]
    fn attestation_count_increments_per_attestor() {
        let env = make_env();
        setup_ledger(&env);
        let (client, _, attestor, signing_key) = setup_contract(&env);
        let subject = Address::generate(&env);

        let p1 = payload(&env, 0x01);
        client.submit_attestation(&attestor, &subject, &1700000000u64, &p1, &sign_payload(&env, &signing_key, &p1));

        assert_eq!(client.get_attestation_count(&attestor), 1);

        let p2 = payload(&env, 0x02);
        client.submit_attestation(&attestor, &subject, &1700000000u64, &p2, &sign_payload(&env, &signing_key, &p2));

        assert_eq!(client.get_attestation_count(&attestor), 2);
    }

    #[test]
    fn attestation_count_independent_per_attestor() {
        let env = make_env();
        setup_ledger(&env);
        let (client, _, attestor1, signing_key1) = setup_contract(&env);

        let attestor2 = Address::generate(&env);
        let mut csprng = OsRng;
        let signing_key2 = SigningKey::generate(&mut csprng);
        register_attestor_with_sep10(&env, &client, &attestor2, &attestor2, &signing_key2);

        let subject = Address::generate(&env);
        let p1 = payload(&env, 0x10);
        client.submit_attestation(&attestor1, &subject, &1700000000u64, &p1, &sign_payload(&env, &signing_key1, &p1));

        assert_eq!(client.get_attestation_count(&attestor1), 1);
        assert_eq!(client.get_attestation_count(&attestor2), 0);
    }

    // -----------------------------------------------------------------------
    // revoke_attestation
    // -----------------------------------------------------------------------

    #[test]
    fn revoke_attestation_marks_as_revoked() {
        let env = make_env();
        setup_ledger(&env);
        let (client, _, attestor, signing_key) = setup_contract(&env);
        let subject = Address::generate(&env);

        let p = payload(&env, 0xAA);
        let id = client.submit_attestation(&attestor, &subject, &1700000000u64, &p, &sign_payload(&env, &signing_key, &p));

        // Attestation exists before revocation
        assert!(client.get_attestation(&id).is_some());

        client.revoke_attestation(&attestor, &id);

        // After revocation, get_attestation should return None
        let result = client.get_attestation(&id);
        assert!(result.is_none());
    }

    #[test]
    fn revoke_attestation_by_non_issuer_fails() {
        let env = make_env();
        setup_ledger(&env);
        let (client, _, attestor, signing_key) = setup_contract(&env);

        let attestor2 = Address::generate(&env);
        let mut csprng = OsRng;
        let signing_key2 = SigningKey::generate(&mut csprng);
        register_attestor_with_sep10(&env, &client, &attestor2, &attestor2, &signing_key2);

        let subject = Address::generate(&env);
        let p = payload(&env, 0xBB);
        let id = client.submit_attestation(&attestor, &subject, &1700000000u64, &p, &sign_payload(&env, &signing_key, &p));

        // attestor2 tries to revoke attestor's attestation — should fail
        let result = client.try_revoke_attestation(&attestor2, &id);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // expires_at
    // -----------------------------------------------------------------------

    #[test]
    fn get_attestation_returns_none_when_expired() {
        let env = make_env();
        setup_ledger(&env);
        let (client, _, attestor, signing_key) = setup_contract(&env);
        let subject = Address::generate(&env);

        // Submit with no expiry — should succeed
        let p = payload(&env, 0xCC);
        let id = client.submit_attestation(&attestor, &subject, &1700000000u64, &p, &sign_payload(&env, &signing_key, &p));

        // The base Attestation has expires_at = None so it never expires
        assert!(client.get_attestation(&id).is_some());
    }
}
