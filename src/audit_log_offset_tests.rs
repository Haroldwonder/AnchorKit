#![cfg(test)]

mod audit_log_offset_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Env,
    };

    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::sep10_test_util::{build_sep10_jwt, sign_attestation};
    use crate::deterministic_hash::compute_payload_hash;
    use soroban_sdk::Bytes;

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

    /// Register an attestor and return its signing key.
    fn setup_attestor(
        env: &Env,
        client: &AnchorKitContractClient,
    ) -> (Address, SigningKey) {
        let sk = SigningKey::generate(&mut OsRng);
        let attestor = Address::generate(env);
        let issuer = attestor.clone();
        let pk = Bytes::from_slice(env, sk.verifying_key().as_bytes());
        client.set_sep10_jwt_verifying_key(&issuer, &pk);
        client.add_attestation_key(&issuer, &pk);

        let sub = attestor.to_string();
        let mut buf = [0u8; 128];
        let len = sub.len() as usize;
        let final_len = if len > 128 { 128 } else { len };
        sub.copy_into_slice(&mut buf[..final_len]);
        let sub_str = core::str::from_utf8(&buf[..final_len]).unwrap_or("");
        let exp = env.ledger().timestamp().saturating_add(86_400);
        let jwt = build_sep10_jwt(&sk, sub_str, exp);
        let token = soroban_sdk::String::from_str(env, jwt.as_str());
        client.register_attestor(&attestor, &token, &issuer);
        (attestor, sk)
    }

    /// Submit one attestation within a session.
    fn submit_one(
        env: &Env,
        client: &AnchorKitContractClient,
        session_id: u64,
        attestor: &Address,
        sk: &SigningKey,
        nonce: u8,
    ) {
        let subject = Address::generate(env);
        let ts = env.ledger().timestamp();
        let mut data = Bytes::new(env);
        data.push_back(nonce);
        let hash = compute_payload_hash(env, &subject, ts, &data);
        let hash_bytes: Bytes = hash.into();
        let sig = sign_payload(env, sk, &hash_bytes);
        client.submit_attestation_with_session(&session_id, attestor, &subject, &ts, &hash_bytes, &sig, &None);
    }

    // -----------------------------------------------------------------------
    // Test 1: offset is 0 before any pruning
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_audit_log_offset_default_zero() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        // Large max size — pruning will never trigger.
        client.initialize(&admin, &100_u64, &None, &None);

        assert_eq!(client.get_audit_log_offset(), 0u64);
    }

    // -----------------------------------------------------------------------
    // Test 2: offset stays 0 while log is below max_audit_log_size
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_audit_log_offset_no_pruning_below_max() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin, &5_u64, &None, &None);

        let (attestor, sk) = setup_attestor(&env, &client);
        let session_id = client.create_session(&attestor);

        // Submit 4 entries — one below the max of 5, no pruning expected.
        for nonce in 0..4u8 {
            submit_one(&env, &client, session_id, &attestor, &sk, nonce);
        }

        assert_eq!(client.get_audit_log_offset(), 0u64);
    }

    // -----------------------------------------------------------------------
    // Test 3: offset advances after pruning is triggered
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_audit_log_offset_updates_after_pruning() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        // max_audit_log_size = 3: pruning fires when live_count >= 3.
        client.initialize(&admin, &3_u64, &None, &None);

        let (attestor, sk) = setup_attestor(&env, &client);
        let session_id = client.create_session(&attestor);

        // First 3 submissions fill the log without pruning.
        for nonce in 0..3u8 {
            submit_one(&env, &client, session_id, &attestor, &sk, nonce);
        }
        assert_eq!(client.get_audit_log_offset(), 0u64);

        // 4th submission triggers pruning: live_count (3) == max_size (3),
        // so to_prune = 1 and new_offset = 1.
        submit_one(&env, &client, session_id, &attestor, &sk, 3);
        assert_eq!(client.get_audit_log_offset(), 1u64);

        // 5th submission: live_count is again 3 (ids 1,2,3), triggers another prune.
        submit_one(&env, &client, session_id, &attestor, &sk, 4);
        assert_eq!(client.get_audit_log_offset(), 2u64);
    }

    // -----------------------------------------------------------------------
    // Test 4: offset matches the new_offset from AuditLogPruned event semantics
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_audit_log_offset_matches_pruned_count() {
        let env = make_env();
        setup_ledger(&env);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        // max = 2: every submission beyond the 2nd triggers a prune.
        client.initialize(&admin, &2_u64, &None, &None);

        let (attestor, sk) = setup_attestor(&env, &client);
        let session_id = client.create_session(&attestor);

        // Submit 5 entries and track expected offset.
        // Pruning fires once the projected live window would exceed max_size.
        //
        // N=0: projected=1, offset=0
        // N=1: projected=2, offset=0 (at capacity, not over)
        // N=2: projected=3 > max=2, prune 1, offset=1
        // N=3: projected=4 > max=2, prune 1, offset=2
        // N=4: projected=5 > max=2, prune 1, offset=3
        let expected_offsets = [0u64, 0, 1, 2, 3];
        for (nonce, &expected) in (0u8..5).zip(expected_offsets.iter()) {
            submit_one(&env, &client, session_id, &attestor, &sk, nonce);
            assert_eq!(
                client.get_audit_log_offset(),
                expected,
                "offset mismatch after submission {}",
                nonce
            );
        }
    }
}
