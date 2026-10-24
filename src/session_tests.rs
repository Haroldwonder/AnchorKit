#![cfg(test)]

// Issue #1105 — resolved: all `submit_attestation_with_session` call sites in this file
// previously passed a trailing `metadata: Option<Map<String, String>>` argument that was
// added in feat commit cc3eb4ed and removed from the contract in fix commit 7bc36508.
// Every call site now passes exactly the 6 arguments the current contract expects:
//   (session_id, issuer, subject, timestamp, payload_hash, signature).

mod session_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Bytes, Env, String,
    };

    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::sep10_test_util::{build_sep10_jwt, register_attestor_with_sep10, sign_attestation};

    fn make_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    fn setup_ledger(env: &Env) {
        env.ledger().set(LedgerInfo {
            timestamp: 1_000_000,
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

    /// Register an attestor via `register_attestor_with_session`, generating a valid SEP-10 token.
    fn register_with_session(
        env: &Env,
        client: &AnchorKitContractClient,
        session_id: u64,
        attestor: &Address,
        sk: &SigningKey,
    ) {
        let issuer = attestor.clone();
        let pk = soroban_sdk::Bytes::from_slice(env, sk.verifying_key().as_bytes());
        client.set_sep10_jwt_verifying_key(&issuer, &pk);
        client.add_attestation_key(&issuer, &pk);

        let sub = attestor.to_string();
        let mut buf = [0u8; 128];
        let len = sub.len() as usize;
        let final_len = if len > 128 { 128 } else { len };
        sub.copy_into_slice(&mut buf[..final_len]);
        let sub_str = core::str::from_utf8(&buf[..final_len]).unwrap_or("");
        let exp = env.ledger().timestamp().saturating_add(86_400);
        let jwt = build_sep10_jwt(sk, sub_str, exp);
        let token = String::from_str(env, jwt.as_str());
        client.register_attestor_with_session(&session_id, attestor, &token, &issuer);
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
        client.initialize(&admin, &100_u64, &None, &None);

        let id0 = client.create_session(&user);
        let id1 = client.create_session(&user);
        let id2 = client.create_session(&user);

        // IDs are still sequential
        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);

        let s0 = client.get_session(&id0);
        let s1 = client.get_session(&id1);
        let s2 = client.get_session(&id2);

        // Nonces must not equal the session ID (no longer sequential 0,1,2)
        assert_ne!(s0.nonce, id0, "nonce should not equal session_id");
        assert_ne!(s1.nonce, id1, "nonce should not equal session_id");
        assert_ne!(s2.nonce, id2, "nonce should not equal session_id");

        // Nonces must be unique across sessions
        assert_ne!(s0.nonce, s1.nonce, "nonces should be unique");
        assert_ne!(s1.nonce, s2.nonce, "nonces should be unique");
        assert_ne!(s0.nonce, s2.nonce, "nonces should be unique");
    }

    #[test]
    fn test_create_session_stores_initiator() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None, &None);

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
        client.initialize(&admin, &100_u64, &None, &None);

        let session_id = client.create_session(&user);
        assert_eq!(client.get_session_operation_count(&session_id).unwrap(), 0);
    }
    #[test]
    fn test_get_session_operation_count_returns_none_for_non_existent_session() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None, &None);

        assert!(client.get_session_operation_count(&999u64).is_none());
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
        client.initialize(&admin, &100_u64, &None, &None);

        let session_id = client.create_session(&admin);
        let sk = SigningKey::generate(&mut OsRng);
        register_with_session(&env, &client, session_id, &attestor, &sk);

        assert_eq!(client.get_session_operation_count(&session_id).unwrap(), 1);
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
        client.initialize(&admin, &100_u64, &None, &None);

        let session_id = client.create_session(&attestor);
        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        let p = payload(&env, 0x01);
        client.submit_attestation_with_session(
            &session_id,
            &attestor,
            &subject,
            &env.ledger().timestamp(),
            &p,
            &sign_payload(&env, &sk, &p), &None,
        );

        assert_eq!(client.get_session_operation_count(&session_id).unwrap(), 1);
    }

    #[test]
    #[should_panic(expected = "HostError: Error(Contract, #4)")]
    fn test_submit_attestation_with_session_fails_for_wrong_initiator() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user_a = Address::generate(&env);
        let user_b = Address::generate(&env);
        let subject = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None, &None);

        // Register user_a as attestor so they can call the function
        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &user_a, &user_a, &sk);

        // user_b creates a session
        let session_id = client.create_session(&user_b);

        // user_a tries to submit to user_b's session
        client.submit_attestation_with_session(
            &session_id,
            &user_a,
            &subject,
            &env.ledger().timestamp(),
            &payload(&env, 0x01),
            &sig(&env, &[0x0a]), &None,
        );
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
        client.initialize(&admin, &100_u64, &None, &None);

        let session_id = client.create_session(&admin);
        let sk = SigningKey::generate(&mut OsRng);
        register_with_session(&env, &client, session_id, &attestor, &sk);

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
        client.initialize(&admin, &100_u64, &None, &None);

        let session_id = client.create_session(&admin);
        let sk = SigningKey::generate(&mut OsRng);
        register_with_session(&env, &client, session_id, &attestor, &sk);

        let log = client.get_audit_log(&0u64);
        assert_eq!(log.log_id, 0);
        assert_eq!(log.session_id, session_id);
        assert_eq!(
            log.operation.operation_type,
            String::from_str(&env, "register")
        );
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
        client.initialize(&admin, &100_u64, &None, &None);

        let session_id = client.create_session(&admin);
        let sk = SigningKey::generate(&mut OsRng);
        register_with_session(&env, &client, session_id, &attestor, &sk);
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
        client.initialize(&admin, &100_u64, &None, &None);

        let session_id = client.create_session(&admin);
        let sk = SigningKey::generate(&mut OsRng);
        register_with_session(&env, &client, session_id, &attestor, &sk);
        client.revoke_attestor_with_session(&session_id, &attestor);

        // log_id 0 = register, log_id 1 = revoke
        let log = client.get_audit_log(&1u64);
        assert_eq!(log.log_id, 1);
        assert_eq!(log.session_id, session_id);
        assert_eq!(
            log.operation.operation_type,
            String::from_str(&env, "revoke")
        );
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
        client.initialize(&admin, &100_u64, &None, &None);

        let session_id = client.create_session(&attestor);
        let admin_session_id = client.create_session(&admin);
        // Set Sep10 key so signature verification works, then register via the admin session.
        let sk = SigningKey::generate(&mut OsRng);
        register_with_session(&env, &client, admin_session_id, &attestor, &sk);
        let p = payload(&env, 0x01);
        client.submit_attestation_with_session(
            &session_id,
            &attestor,
            &subject,
            &env.ledger().timestamp(),
            &p,
            &sign_payload(&env, &sk, &p), &None,
        );

        let log0 = client.get_audit_log(&0u64);
        let log1 = client.get_audit_log(&1u64);
        assert_eq!(log0.log_id, 0);
        assert_eq!(log1.log_id, 1);
        assert_eq!(
            log0.operation.operation_type,
            String::from_str(&env, "register")
        );
        assert_eq!(
            log1.operation.operation_type,
            String::from_str(&env, "attest")
        );
    }

    #[test]
    fn test_audit_log_ids_strictly_monotonic_when_interleaving_two_sessions() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);

        // Use large max_audit_log_size to avoid pruning during this test.
        client.initialize(&admin, &100_u64, &None, &None);

        // Two independent sessions.
        let initiator_a = Address::generate(&env);
        let initiator_b = Address::generate(&env);
        let session_a = client.create_session(&initiator_a);
        let session_b = client.create_session(&initiator_b);
        let admin_session_a = client.create_session(&admin);
        let admin_session_b = client.create_session(&admin);

        // For attestations: use one subject and two distinct attestations.
        let subject = Address::generate(&env);

        // Session A attestor
        let attestor_a = Address::generate(&env);
        let sk_a = SigningKey::generate(&mut OsRng);
        register_with_session(&env, &client, admin_session_a, &attestor_a, &sk_a);

        // Interleave: Session B attestor
        let attestor_b = Address::generate(&env);
        let sk_b = SigningKey::generate(&mut OsRng);
        register_with_session(&env, &client, admin_session_b, &attestor_b, &sk_b);

        // Interleave further: submit attestation in session A
        let p_a = payload(&env, 0x11);
        client.submit_attestation_with_session(
            &session_a,
            &attestor_a,
            &subject,
            &env.ledger().timestamp(),
            &p_a,
            &sign_payload(&env, &sk_a, &p_a), &None,
        );

        // ...and session B
        let p_b = payload(&env, 0x22);
        client.submit_attestation_with_session(
            &session_b,
            &attestor_b,
            &subject,
            &env.ledger().timestamp(),
            &p_b,
            &sign_payload(&env, &sk_b, &p_b), &None,
        );

        // Four operations total => audit log IDs should be strictly monotonically increasing.
        let log0 = client.get_audit_log(&0u64);
        let log1 = client.get_audit_log(&1u64);
        let log2 = client.get_audit_log(&2u64);
        let log3 = client.get_audit_log(&3u64);

        // Stronger check: exact order.
        assert_eq!(log0.log_id, 0);
        assert_eq!(log1.log_id, 1);
        assert_eq!(log2.log_id, 2);
        assert_eq!(log3.log_id, 3);

        // Extra monotonicity assertions.
        assert!(log0.log_id < log1.log_id);
        assert!(log1.log_id < log2.log_id);
        assert!(log2.log_id < log3.log_id);

        // Sanity-check operation types: register, register, attest, attest.
        assert_eq!(
            log0.operation.operation_type,
            String::from_str(&env, "register")
        );
        assert_eq!(
            log1.operation.operation_type,
            String::from_str(&env, "register")
        );
        assert_eq!(
            log2.operation.operation_type,
            String::from_str(&env, "attest")
        );
        assert_eq!(
            log3.operation.operation_type,
            String::from_str(&env, "attest")
        );
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
        client.initialize(&admin, &100_u64, &None, &None);

        // Step 1: create session
        let session_id = client.create_session(&attestor);
        assert_eq!(session_id, 0);

        let admin_session_id = client.create_session(&admin);
        // Step 2: set Sep10 key, then register via the admin session.
        let sk = SigningKey::generate(&mut OsRng);
        register_with_session(&env, &client, admin_session_id, &attestor, &sk);
        assert!(client.is_attestor(&attestor));

        // Step 3: two attestations
        let p0 = payload(&env, 0x01);
        let p1 = payload(&env, 0x02);
        let id0 = client.submit_attestation_with_session(
            &session_id,
            &attestor,
            &subject,
            &env.ledger().timestamp(),
            &p0,
            &sign_payload(&env, &sk, &p0), &None,
        );
        let id1 = client.submit_attestation_with_session(
            &session_id,
            &attestor,
            &subject,
            &env.ledger().timestamp(),
            &p1,
            &sign_payload(&env, &sk, &p1), &None,
        );
        assert_eq!(id0, 0);
        assert_eq!(id1, 1);

        // Step 4: the admin session records registration; the attestor session records attestations.
        assert_eq!(client.get_session_operation_count(&admin_session_id).unwrap(), 1);
        assert_eq!(client.get_session_operation_count(&session_id).unwrap(), 2);

        // Step 5: verify audit logs
        let log0 = client.get_audit_log(&0u64);
        assert_eq!(
            log0.operation.operation_type,
            String::from_str(&env, "register")
        );

        let log1 = client.get_audit_log(&1u64);
        assert_eq!(
            log1.operation.operation_type,
            String::from_str(&env, "attest")
        );
        assert_eq!(
            log1.operation.result_summary,
            String::from_str(&env, "attestation_id=0")
        );

        let log2 = client.get_audit_log(&2u64);
        assert_eq!(
            log2.operation.operation_type,
            String::from_str(&env, "attest")
        );
        assert_eq!(
            log2.operation.result_summary,
            String::from_str(&env, "attestation_id=1")
        );
    }

    // -----------------------------------------------------------------------
    // Audit log pruning
    // -----------------------------------------------------------------------

    #[test]
    fn test_audit_log_pruning_keeps_new_entry() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        // max_audit_log_size = 2: only 2 live entries at a time
        client.initialize(&admin, &2_u64, &None, &None);

        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let sk2 = SigningKey::generate(&mut csprng);
        let attestor = Address::generate(&env);
        let attestor2 = Address::generate(&env);
        let attestor3 = Address::generate(&env);

        // Write log_id=0 via register_attestor_with_session
        let session_id = client.create_session(&attestor);
        let admin_session_id = client.create_session(&admin);
        register_with_session(&env, &client, admin_session_id, &attestor, &signing_key);

        // Write log_id=1 via register_attestor_with_session for attestor2
        register_with_session(&env, &client, admin_session_id, &attestor2, &sk2);

        // log_id=0 still accessible (live=[0,1], count=2 == max_size, no prune yet).
        let log0 = client.get_audit_log(&0u64);
        assert_eq!(log0.log_id, 0);

        // Write log_id=2 → live=[0,1,2], count=3 > max_size=2 → prune log_id=0.
        let ph = payload(&env, 0xAB);
        let s = sign_attestation(&env, &signing_key, &client.address, &attestor, &attestor, env.ledger().timestamp(), &ph);
        client.submit_attestation_with_session(
            &session_id,
            &attestor,
            &attestor,
            &env.ledger().timestamp(),
            &ph,
            &s, &None,
        );

        // log_id=2 must be accessible.
        let log2 = client.get_audit_log(&2u64);
        assert_eq!(log2.log_id, 2);
    }

    #[test]
    #[should_panic]
    fn test_audit_log_pruned_entry_is_gone() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin, &2_u64, &None, &None);

        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let sk2 = SigningKey::generate(&mut csprng);
        let attestor = Address::generate(&env);
        let attestor2 = Address::generate(&env);

        // Write log_id=0 and log_id=1 via register_attestor_with_session
        let session_id = client.create_session(&attestor);
        let admin_session_id = client.create_session(&admin);
        register_with_session(&env, &client, admin_session_id, &attestor, &signing_key);
        register_with_session(&env, &client, admin_session_id, &attestor2, &sk2);

        // Write log_id=2 → prunes log_id=0.
        let ph = payload(&env, 0xCD);
        let s = sign_attestation(&env, &signing_key, &client.address, &attestor, &attestor, env.ledger().timestamp(), &ph);
        client.submit_attestation_with_session(
            &session_id,
            &attestor,
            &attestor,
            &env.ledger().timestamp(),
            &ph,
            &s, &None,
        );

        // Accessing pruned entry must panic.
        client.get_audit_log(&0u64);
    }

    #[test]
    #[should_panic]
    fn test_initialize_rejects_zero_max_audit_log_size() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &0_u64, &None, &None); // must panic with AuditLogMaxSizeInvalid
    }
}
