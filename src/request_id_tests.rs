#![cfg(test)]

mod request_id_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Bytes, Env, String,
    };

    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::sep10_test_util::{register_attestor_with_sep10, sign_attestation};

    fn make_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    fn payload(env: &Env, byte: u8) -> Bytes {
        let mut b = Bytes::new(env);
        for _ in 0..32 {
            b.push_back(byte);
        }
        b
    }

    #[test]
    fn test_generate_request_id() {
        let env = make_env();
        env.ledger().set(LedgerInfo {
            timestamp: 1000,
            protocol_version: 21,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let req_id = client.generate_request_id();
        assert_eq!(req_id.created_at, 1000);
        assert_eq!(req_id.id.len(), 32);
    }

    #[test]
    fn test_unique_request_ids() {
        let env = make_env();
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        env.ledger().set(LedgerInfo {
            timestamp: 0,
            protocol_version: 21,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });
        let id1 = client.generate_request_id();

        env.ledger().set(LedgerInfo {
            timestamp: 0,
            protocol_version: 21,
            sequence_number: 1,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });
        let id2 = client.generate_request_id();

        assert_ne!(id1.id, id2.id);
    }

    #[test]
    fn test_submit_attestation_with_request_id() {
        let env = make_env();
        env.ledger().set(LedgerInfo {
            timestamp: 1000,
            protocol_version: 21,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let subject = Address::generate(&env);

        client.initialize(&admin, &100_u64, &None, &None);
        let signing_key = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &signing_key);

        let req_id = client.generate_request_id();
        let p = payload(&env, 0x01);
        let attest_id = client.submit_with_request_id(&req_id, &attestor, &subject, &1000u64, &p, &sign_payload(&env, &signing_key, &p), &None);

        assert_eq!(attest_id, 0);

        let span = client.get_tracing_span(&req_id.id).unwrap();
        assert_eq!(span.operation, String::from_str(&env, "submit_attestation"));
        assert_eq!(span.status, String::from_str(&env, "success"));
        assert_eq!(span.actor, attestor);
    }

    #[test]
    fn test_tracing_span_timing() {
        let env = make_env();
        env.ledger().set(LedgerInfo {
            timestamp: 1000,
            protocol_version: 21,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let subject = Address::generate(&env);

        client.initialize(&admin, &100_u64, &None, &None);
        let signing_key = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &signing_key);

        let req_id = client.generate_request_id();
        let p = payload(&env, 0x01);
        client.submit_with_request_id(&req_id, &attestor, &subject, &1000u64, &p, &sign_payload(&env, &signing_key, &p), &None);

        let span = client.get_tracing_span(&req_id.id).unwrap();
        assert_eq!(span.started_at, 1000);
        assert_eq!(span.completed_at, 1000);
    }

    #[test]
    fn test_tracing_span_records_failure() {
        let env = make_env();
        env.ledger().set(LedgerInfo {
            timestamp: 1000,
            protocol_version: 21,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let unregistered = Address::generate(&env);
        let subject = Address::generate(&env);

        client.initialize(&admin, &100_u64, &None, &None);

        let req_id = client.generate_request_id();

        let result = client.try_submit_with_request_id(&req_id, &unregistered, &subject, &1000u64, &payload(&env, 0x01), &Bytes::new(&env), &None);
        assert!(result.is_err());

        let span = client.get_tracing_span(&req_id.id);
        assert!(span.is_none());
    }

    #[test]
    fn test_submit_quote_with_request_id() {
        let env = make_env();
        env.ledger().set(LedgerInfo {
            timestamp: 1000,
            protocol_version: 21,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);

        client.initialize(&admin, &100_u64, &None, &None);
        let signing_key = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &anchor, &anchor, &signing_key);

        let mut services = soroban_sdk::Vec::new(&env);
        services.push_back(3u32);
        client.configure_services(&anchor, &services);

        let req_id = client.generate_request_id();
        client.quote_with_request_id(
            &req_id,
            &anchor,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64,
            &10_000u32,
            &100u64,
            &10000u64,
            &4600u64,
        );

        let span = client.get_tracing_span(&req_id.id).unwrap();
        assert_eq!(span.operation, String::from_str(&env, "submit_quote"));
        assert_eq!(span.status, String::from_str(&env, "success"));

        let over_limit_req_id = client.generate_request_id();
        assert!(client.try_quote_with_request_id(
            &over_limit_req_id,
            &anchor,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64,
            &10_001u32,
            &100u64,
            &10000u64,
            &4600u64,
        ).is_err());
    }
}
