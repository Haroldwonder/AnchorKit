#![cfg(test)]

mod attestation_pagination_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Bytes, Env, Vec,
    };
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::sep10_test_util::register_attestor_with_sep10;

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

    fn sig(env: &Env, byte: u8) -> Bytes {
        let mut b = Bytes::new(env);
        for _ in 0..64 {
            b.push_back(byte);
        }
        b
    }

    #[test]
    fn test_list_attestations_empty() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let subject = Address::generate(&env);
        let results = client.list_attestations(&subject, &0, &10);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_list_attestations_single_subject() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let subject = Address::generate(&env);
        client.initialize(&admin);

        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        // Submit 5 attestations
        for i in 0..5 {
            client.submit_attestation(&attestor, &subject, &(1700000000 + i as u64), &payload(&env, i), &sig(&env, i));
        }

        let results = client.list_attestations(&subject, &0, &10);
        assert_eq!(results.len(), 5);
        assert_eq!(results.get(0).unwrap().id, 0);
        assert_eq!(results.get(4).unwrap().id, 4);
    }

    #[test]
    fn test_list_attestations_pagination() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let subject = Address::generate(&env);
        client.initialize(&admin);

        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        // Submit 10 attestations
        for i in 0..10 {
            client.submit_attestation(&attestor, &subject, &(1700000000 + i as u64), &payload(&env, i), &sig(&env, i));
        }

        // Page 1: offset 0, limit 3
        let page1 = client.list_attestations(&subject, &0, &3);
        assert_eq!(page1.len(), 3);
        assert_eq!(page1.get(0).unwrap().id, 0);
        assert_eq!(page1.get(2).unwrap().id, 2);

        // Page 2: offset 3, limit 3
        let page2 = client.list_attestations(&subject, &3, &3);
        assert_eq!(page2.len(), 3);
        assert_eq!(page2.get(0).unwrap().id, 3);
        assert_eq!(page2.get(2).unwrap().id, 5);

        // Page 4: offset 9, limit 3 (only 1 left)
        let page4 = client.list_attestations(&subject, &9, &3);
        assert_eq!(page4.len(), 1);
        assert_eq!(page4.get(0).unwrap().id, 9);
    }

    #[test]
    fn test_list_attestations_multiple_subjects() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let subj1 = Address::generate(&env);
        let subj2 = Address::generate(&env);
        client.initialize(&admin);

        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        // Subj1: 2 attestations
        client.submit_attestation(&attestor, &subj1, &1700000001, &payload(&env, 1), &sig(&env, 1));
        client.submit_attestation(&attestor, &subj1, &1700000002, &payload(&env, 2), &sig(&env, 2));

        // Subj2: 1 attestation
        client.submit_attestation(&attestor, &subj2, &1700000003, &payload(&env, 3), &sig(&env, 3));

        let res1 = client.list_attestations(&subj1, &0, &10);
        assert_eq!(res1.len(), 2);
        assert_eq!(res1.get(0).unwrap().id, 0);
        assert_eq!(res1.get(1).unwrap().id, 1);

        let res2 = client.list_attestations(&subj2, &0, &10);
        assert_eq!(res2.len(), 1);
        assert_eq!(res2.get(0).unwrap().id, 2);
    }

    #[test]
    fn test_list_attestations_limit_capping() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let subject = Address::generate(&env);
        client.initialize(&admin);

        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        // Submit 60 attestations
        for i in 0..60 {
            client.submit_attestation(&attestor, &subject, &(1700000000 + i as u64), &payload(&env, i as u8), &sig(&env, i as u8));
        }

        // Request 100, should get only 50
        let results = client.list_attestations(&subject, &0, &100);
        assert_eq!(results.len(), 50);
    }

    #[test]
    #[should_panic]
    fn test_attestation_id_overflow_panics() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let subject = Address::generate(&env);
        client.initialize(&admin);

        // Seed the counter to u64::MAX so the next increment overflows
        env.as_contract(&contract_id, &|| {
            let ck = soroban_sdk::vec![&env, soroban_sdk::symbol_short!("COUNTER")];
            env.storage().instance().set(&ck, &u64::MAX);
        });

        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        // This should panic due to overflow guard
        client.submit_attestation(&attestor, &subject, &1700000001, &payload(&env, 1), &sig(&env, 1));
    }

    #[test]
    fn test_list_attestations_offset_out_of_bounds() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let subject = Address::generate(&env);
        client.initialize(&admin);

        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        client.submit_attestation(&attestor, &subject, &1700000001, &payload(&env, 1), &sig(&env, 1));

        let results = client.list_attestations(&subject, &5, &10);
        assert_eq!(results.len(), 0);
    }
}
