#![cfg(test)]

mod is_attestation_valid_tests {
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

    #[test]
    fn returns_false_for_nonexistent_id() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        assert!(!client.is_attestation_valid(&999));
    }

    #[test]
    fn returns_true_for_valid_attestation() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let subject = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &signing_key);

        let p = payload(&env, 0xAB);
        let id = client.submit_attestation(
            &attestor,
            &subject,
            &1700000000u64,
            &p,
            &sign_payload(&env, &signing_key, &p),
        );

        assert!(client.is_attestation_valid(&id));
    }

    #[test]
    fn returns_false_after_attestor_revoked() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let subject = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &signing_key);

        let p = payload(&env, 0xCD);
        let id = client.submit_attestation(
            &attestor,
            &subject,
            &1700000000u64,
            &p,
            &sign_payload(&env, &signing_key, &p),
        );

        assert!(client.is_attestation_valid(&id));

        client.revoke_attestor(&attestor);

        assert!(!client.is_attestation_valid(&id));
    }
}
