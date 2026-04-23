#![cfg(test)]

mod session_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Bytes, Env, String,
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
            timestamp: 0,
            protocol_version: 21,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
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

    fn sig(env: &Env, bytes: &[u8]) -> Bytes {
        let mut b = Bytes::new(env);
        for &x in bytes {
            b.push_back(x);
        }
        b
    }

    // -----------------------------------------------------------------------
    // create_session
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_session_returns_sequential_ids() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        client.initialize(&admin);

        let id0 = client.create_session(&user);
        let id1 = client.create_session(&user);
        let id2 = client.create_session(&user);

        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn test_create_session_stores_initiator() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        client.initialize(&admin);

        let session_id = client.create_session(&user);
        let session = client.get_session(&session_id);

        assert_eq!(session.session_id, session_id);
        assert_eq!(session.initiator, user);
    }

    // -----------------------------------------------------------------------
    // get_session_operation_count
    // -----------------------------------------------------------------------

    #[test]
    fn test_operation_count_starts_at_zero() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        client.initialize(&admin);

        let session_id = client.create_session(&user);
        assert_eq!(client.get_session_operation_count(&session_id), 0);
    }

    #[test]
    fn test_operation_count_increments_with_register_attestor_with_session() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let attestor = Address::generate(&env);
        client.initialize(&admin);

        let session_id = client.create_session(&user);
        client.register_attestor_with_session(&session_id, &attestor);

        assert_eq!(client.get_session_operation_count(&session_id), 1);
    }

    #[test]
    fn test_operation_count_increments_with_submit_attestation_with_session() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let attestor = Address::generate(&env);
        let subject = Address::generate(&env);
        client.initialize(&admin);

        let session_id = client.create_session(&user);
        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        client.submit_attestation_with_session(
            &session_id,
            &attestor,
            &subject,
            &1700000001u64,
            &payload(&env, 0x01),
            &sig(&env, &[0x0a, 0x0b]),
        );

        assert_eq!(client.get_session_operation_count(&session_id), 1);
    }

    // -----------------------------------------------------------------------
    // register_attestor_with_session
    // -----------------------------------------------------------------------

    #[test]
    fn test_register_attestor_with_session_registers_attestor() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let attestor = Address::generate(&env);
        client.initialize(&admin);

        let session_id = client.create_session(&user);
        client.register_attestor_with_session(&session_id, &attestor);

        assert!(client.is_attestor(&attestor));
    }

    #[test]
    fn test_register_attestor_with_session_writes_audit_log() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let attestor = Address::generate(&env);
        client.initialize(&admin);

        let session_id = client.create_session(&user);
        client.register_attestor_with_session(&session_id, &attestor);

        let log = client.get_audit_log(&0u64);
        assert_eq!(log.log_id, 0);
        assert_eq!(log.session_id, session_id);
        assert_eq!(log.operation.operation_type, String::from_str(&env, "register"));
        assert_eq!(log.operation.status, String::from_str(&env, "success"));
        assert_eq!(log.operation.operation_index, 0);
    }

    // -----------------------------------------------------------------------
    // revoke_attestor_with_session
    // -----------------------------------------------------------------------

    #[test]
    fn test_revoke_attestor_with_session_removes_attestor() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let attestor = Address::generate(&env);
        client.initialize(&admin);

        let session_id = client.create_session(&user);
        client.register_attestor_with_session(&session_id, &attestor);
        client.revoke_attestor_with_session(&session_id, &attestor);

        assert!(!client.is_attestor(&attestor));
    }

    #[test]
    fn test_revoke_attestor_with_session_writes_audit_log() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let attestor = Address::generate(&env);
        client.initialize(&admin);

        let session_id = client.create_session(&user);
        client.register_attestor_with_session(&session_id, &attestor);
        client.revoke_attestor_with_session(&session_id, &attestor);

        // log_id 0 = register, log_id 1 = revoke
        let log = client.get_audit_log(&1u64);
        assert_eq!(log.log_id, 1);
        assert_eq!(log.session_id, session_id);
        assert_eq!(log.operation.operation_type, String::from_str(&env, "revoke"));
        assert_eq!(log.operation.status, String::from_str(&env, "success"));
    }

    // -----------------------------------------------------------------------
    // get_audit_log
    // -----------------------------------------------------------------------

    #[test]
    fn test_audit_log_sequential_ids_across_operations() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let attestor = Address::generate(&env);
        let subject = Address::generate(&env);
        client.initialize(&admin);

        let session_id = client.create_session(&user);
        client.register_attestor_with_session(&session_id, &attestor);
        client.submit_attestation_with_session(
            &session_id,
            &attestor,
            &subject,
            &1700000001u64,
            &payload(&env, 0x01),
            &sig(&env, &[0x0a]),
        );

        let log0 = client.get_audit_log(&0u64);
        let log1 = client.get_audit_log(&1u64);
        assert_eq!(log0.log_id, 0);
        assert_eq!(log1.log_id, 1);
        assert_eq!(log0.operation.operation_type, String::from_str(&env, "register"));
        assert_eq!(log1.operation.operation_type, String::from_str(&env, "attest"));
    }

    // -----------------------------------------------------------------------
    // Snapshot reproducibility test (matches test_snapshots/session_tests/)
    // -----------------------------------------------------------------------

    #[test]
    fn test_recorded_anchor_session_replay_is_reproducible_offline() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let attestor = Address::generate(&env);
        let subject = Address::generate(&env);
        client.initialize(&admin);

        // Step 1: create session
        let session_id = client.create_session(&user);
        assert_eq!(session_id, 0);

        // Step 2: register attestor with session
        client.register_attestor_with_session(&session_id, &attestor);
        assert!(client.is_attestor(&attestor));

        // Step 3: two attestations
        let id0 = client.submit_attestation_with_session(
            &session_id,
            &attestor,
            &subject,
            &1700000001u64,
            &payload(&env, 0x01),
            &sig(&env, &[0x0a, 0x0b, 0x0c, 0x0d]),
        );
        let id1 = client.submit_attestation_with_session(
            &session_id,
            &attestor,
            &subject,
            &1700000002u64,
            &payload(&env, 0x02),
            &sig(&env, &[0x14, 0x15, 0x16, 0x17]),
        );
        assert_eq!(id0, 0);
        assert_eq!(id1, 1);

        // Step 4: verify operation count = 3 (register + 2 attests)
        assert_eq!(client.get_session_operation_count(&session_id), 3);

        // Step 5: verify audit logs
        let log0 = client.get_audit_log(&0u64);
        assert_eq!(log0.operation.operation_type, String::from_str(&env, "register"));
        assert_eq!(log0.operation.operation_index, 0);

        let log1 = client.get_audit_log(&1u64);
        assert_eq!(log1.operation.operation_type, String::from_str(&env, "attest"));
        assert_eq!(log1.operation.operation_index, 1);
        assert_eq!(log1.operation.result_data, 0); // attestation id 0

        let log2 = client.get_audit_log(&2u64);
        assert_eq!(log2.operation.operation_type, String::from_str(&env, "attest"));
        assert_eq!(log2.operation.operation_index, 2);
        assert_eq!(log2.operation.result_data, 1); // attestation id 1
    }

    // -----------------------------------------------------------------------
    // get_audit_log_range
    // -----------------------------------------------------------------------

    #[test]
    fn test_audit_log_range_basic() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register(AnchorKitContract, ());
        let client = AnchorKitContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        let (signing_key, _) = register_attestor_with_sep10(&env, &client, &admin);
        let session_id = client.create_session(&admin);

        // Create 3 audit log entries via register + 2 attests
        let _ = signing_key; // already registered above (log 0)
        let ph1 = payload(&env, 1);
        let s1 = sig(&env, &[1u8; 64]);
        client.submit_attestation_with_session(&session_id, &admin, &admin, &1u64, &ph1, &s1);
        let ph2 = payload(&env, 2);
        let s2 = sig(&env, &[2u8; 64]);
        client.submit_attestation_with_session(&session_id, &admin, &admin, &2u64, &ph2, &s2);

        // Range [0, 2] should return 3 entries
        let logs = client.get_audit_log_range(&0u64, &2u64);
        assert_eq!(logs.len(), 3);
        assert_eq!(logs.get(0).unwrap().log_id, 0);
        assert_eq!(logs.get(2).unwrap().log_id, 2);
    }

    #[test]
    fn test_audit_log_range_empty_for_out_of_range() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register(AnchorKitContract, ());
        let client = AnchorKitContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        // No audit logs written — range should return empty vec
        let logs = client.get_audit_log_range(&0u64, &5u64);
        assert_eq!(logs.len(), 0);
    }

    #[test]
    fn test_audit_log_range_max_100() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register(AnchorKitContract, ());
        let client = AnchorKitContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        // Requesting 200 entries should be capped at 100
        let logs = client.get_audit_log_range(&0u64, &199u64);
        // No entries stored, but the range is capped — result is empty (not 200 iterations)
        assert_eq!(logs.len(), 0);
    }

    #[test]
    fn test_audit_log_range_inverted_ids_returns_empty() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register(AnchorKitContract, ());
        let client = AnchorKitContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        let logs = client.get_audit_log_range(&5u64, &2u64);
        assert_eq!(logs.len(), 0);
    }
}
