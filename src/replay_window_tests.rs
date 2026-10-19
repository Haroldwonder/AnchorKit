#![cfg(test)]

mod replay_window_tests {
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Bytes, Env,
    };

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::sep10_test_util::{register_attestor_with_sep10, sign_payload};

    fn make_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    fn set_ts(env: &Env, ts: u64) {
        env.ledger().set(LedgerInfo {
            timestamp: ts,
            protocol_version: 20,
            sequence_number: 1,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 16,
            min_persistent_entry_ttl: 4096,
            max_entry_ttl: 6_312_000,
        });
    }

    fn setup(env: &Env) -> (AnchorKitContractClient, Address, Address, SigningKey) {
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let attestor = Address::generate(env);
        let sk = SigningKey::generate(&mut OsRng);
        (client, admin, attestor, sk)
    }

    fn dummy_hash(env: &Env, seed: u8) -> Bytes {
        Bytes::from_slice(env, &[seed; 32])
    }

    // -----------------------------------------------------------------------
    // Default window (300 s)
    // -----------------------------------------------------------------------

    #[test]
    fn default_window_accepts_timestamp_within_300s() {
        let env = make_env();
        set_ts(&env, 1_000_000);
        let (client, admin, attestor, sk) = setup(&env);

        // Initialize with no custom window → defaults to 300 s
        client.initialize(&admin, &100_u64, &None, &None);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        // timestamp = now - 299 s → inside window
        let ts: u64 = 1_000_000 - 299;
        let hash = dummy_hash(&env, 1);
        let sig = sign_payload(&env, &sk, &hash);
        let id = client.submit_attestation(&attestor, &Address::generate(&env), &ts, &hash, &sig);
        assert_eq!(id, 0);
    }

    #[test]
    #[should_panic]
    fn default_window_rejects_timestamp_outside_300s() {
        let env = make_env();
        set_ts(&env, 1_000_000);
        let (client, admin, attestor, sk) = setup(&env);

        client.initialize(&admin, &100_u64, &None, &None);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        // timestamp = now - 301 s → outside default 300 s window
        let ts: u64 = 1_000_000 - 301;
        let hash = dummy_hash(&env, 2);
        let sig = sign_payload(&env, &sk, &hash);
        client.submit_attestation(&attestor, &Address::generate(&env), &ts, &hash, &sig);
    }

    // -----------------------------------------------------------------------
    // Custom window
    // -----------------------------------------------------------------------

    #[test]
    fn custom_window_accepts_timestamp_within_custom_range() {
        let env = make_env();
        set_ts(&env, 1_000_000);
        let (client, admin, attestor, sk) = setup(&env);

        // Custom window: 3600 s (1 hour)
        client.initialize(&admin, &100_u64, &Some(3600u64), &None);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        // timestamp = now - 3599 s → inside custom window
        let ts: u64 = 1_000_000 - 3599;
        let hash = dummy_hash(&env, 3);
        let sig = sign_payload(&env, &sk, &hash);
        let id = client.submit_attestation(&attestor, &Address::generate(&env), &ts, &hash, &sig);
        assert_eq!(id, 0);
    }

    #[test]
    #[should_panic]
    fn custom_window_rejects_timestamp_outside_custom_range() {
        let env = make_env();
        set_ts(&env, 1_000_000);
        let (client, admin, attestor, sk) = setup(&env);

        // Custom window: 60 s
        client.initialize(&admin, &100_u64, &Some(60u64), &None);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        // timestamp = now - 61 s → outside 60 s window
        let ts: u64 = 1_000_000 - 61;
        let hash = dummy_hash(&env, 4);
        let sig = sign_payload(&env, &sk, &hash);
        client.submit_attestation(&attestor, &Address::generate(&env), &ts, &hash, &sig);
    }

    // -----------------------------------------------------------------------
    // Future timestamp rejection (issue #436)
    // -----------------------------------------------------------------------

    #[test]
    #[should_panic]
    fn rejects_future_timestamp_within_window() {
        let env = make_env();
        set_ts(&env, 1_000_000);
        let (client, admin, attestor, sk) = setup(&env);

        client.initialize(&admin, &100_u64, &None, &None);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        // timestamp = now + 1 s → future, must be rejected even though it's within the old symmetric window
        let ts: u64 = 1_000_000 + 1;
        let hash = dummy_hash(&env, 6);
        let sig = sign_payload(&env, &sk, &hash);
        client.submit_attestation(&attestor, &Address::generate(&env), &ts, &hash, &sig);
    }

    #[test]
    fn custom_window_zero_only_accepts_exact_timestamp() {
        let env = make_env();
        set_ts(&env, 1_000_000);
        let (client, admin, attestor, sk) = setup(&env);

        // Window = 0 → only exact ledger timestamp is valid
        client.initialize(&admin, &100_u64, &Some(0u64), &None);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        let ts: u64 = 1_000_000; // exact match
        let hash = dummy_hash(&env, 5);
        let sig = sign_payload(&env, &sk, &hash);
        let id = client.submit_attestation(&attestor, &Address::generate(&env), &ts, &hash, &sig);
        assert_eq!(id, 0);
    }
}
