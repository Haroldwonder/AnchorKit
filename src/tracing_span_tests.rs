#![cfg(test)]

mod tracing_span_tests {
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
    fn test_span_propagates_across_operations() {
        let env = make_env();
        env.ledger().set(LedgerInfo {
            timestamp: 1_000,
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

        client.initialize(&admin, &100_u64, &None, &None);
        let req_id = client.generate_request_id();
        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        let span = client.get_tracing_span(&req_id.id);
        assert!(span.is_none());
    }

    #[test]
    fn test_span_emits_request_id() {
        let env = make_env();
        env.ledger().set(LedgerInfo {
            timestamp: 1_000,
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
        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        let req_id = client.generate_request_id();
        let p = payload(&env, 0x01);
        client.submit_with_request_id(
            &req_id,
            &attestor,
            &subject,
            &env.ledger().timestamp(),
            &p,
            &sign_payload(&env, &sk, &p),
            &None,
        );

        let span = client.get_tracing_span(&req_id.id).unwrap();
        assert_eq!(span.request_id.id, req_id.id);
        assert_eq!(span.request_id.created_at, req_id.created_at);
    }

    #[test]
    fn test_span_emits_operation_metadata() {
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
        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        let req_id = client.generate_request_id();
        let p = payload(&env, 0x01);
        client.submit_with_request_id(&req_id, &attestor, &subject, &1000u64, &p, &sign_payload(&env, &sk, &p), &None);

        let span = client.get_tracing_span(&req_id.id).unwrap();
        assert_eq!(span.operation, String::from_str(&env, "submit_attestation"));
        assert_eq!(span.actor, attestor);
        assert_eq!(span.started_at, 1000);
        assert_eq!(span.completed_at, 1000);
        assert_eq!(span.status, String::from_str(&env, "success"));
    }

    #[test]
    fn test_structured_log_format() {
        let env = make_env();
        env.ledger().set(LedgerInfo {
            timestamp: 1_000,
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
        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        let req_id = client.generate_request_id();
        let p = payload(&env, 0x01);
        client.submit_with_request_id(
            &req_id,
            &attestor,
            &subject,
            &env.ledger().timestamp(),
            &p,
            &sign_payload(&env, &sk, &p),
            &None,
        );

        let span = client.get_tracing_span(&req_id.id).unwrap();
        assert_eq!(span.request_id.id, req_id.id);
        assert_eq!(span.operation, String::from_str(&env, "submit_attestation"));
        assert_eq!(span.actor, attestor);
        assert_eq!(span.status, String::from_str(&env, "success"));
    }
}
