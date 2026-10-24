#![cfg(test)]

mod session_expiry_error_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Env,
    };

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::errors::ErrorCode;
    use crate::sep10_test_util::{build_sep10_jwt, sign_attestation};
    use crate::deterministic_hash::compute_payload_hash;
    use soroban_sdk::Bytes;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn make_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    fn setup_ledger(env: &Env, timestamp: u64) {
        env.ledger().set(LedgerInfo {
            timestamp,
            protocol_version: 21,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });
    }

    fn setup_contract(env: &Env) -> AnchorKitContractClient {
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin, &100_u64, &None, &None);
        client
    }

    fn register_attestor(env: &Env, client: &AnchorKitContractClient) -> (Address, SigningKey) {
        let sk = SigningKey::generate(&mut OsRng);
        let attestor = Address::generate(env);
        let pk = Bytes::from_slice(env, sk.verifying_key().as_bytes());
        client.set_sep10_jwt_verifying_key(&attestor, &pk);
        client.add_attestation_key(&attestor, &pk);
        let sub = attestor.to_string();
        let mut buf = [0u8; 128];
        let len = sub.len() as usize;
        let final_len = if len > 128 { 128 } else { len };
        sub.copy_into_slice(&mut buf[..final_len]);
        let sub_str = core::str::from_utf8(&buf[..final_len]).unwrap_or("");
        let exp = env.ledger().timestamp().saturating_add(86_400);
        let jwt = build_sep10_jwt(&sk, sub_str, exp);
        let token = soroban_sdk::String::from_str(env, jwt.as_str());
        client.register_attestor(&attestor, &token, &attestor);
        (attestor, sk)
    }

    // -----------------------------------------------------------------------
    // Case 1: Missing session → SessionNotFound
    // -----------------------------------------------------------------------

    #[test]
    fn test_missing_session_returns_session_not_found() {
        let env = make_env();
        setup_ledger(&env, 1_000_000);
        let client = setup_contract(&env);
        let (attestor, sk) = register_attestor(&env, &client);

        let nonexistent_session_id = 9999u64;
        let subject = Address::generate(&env);
        let ts = env.ledger().timestamp();
        let mut data = Bytes::new(&env);
        data.push_back(0x01);
        let hash = compute_payload_hash(&env, &subject, ts, &data);
        let hash_bytes: Bytes = hash.into();
        let sig = sign_attestation(&env, &sk, &client.address, &attestor, &subject, ts, &hash_bytes);

        let result = client.try_submit_attestation_with_session(&nonexistent_session_id, &attestor, &subject, &ts, &hash_bytes, &sig, &None);
        assert!(result.is_err());
        let err = result.unwrap_err().unwrap();
        assert_eq!(err, soroban_sdk::Error::from_contract_error(ErrorCode::SessionNotFound as u32));
    }

    // -----------------------------------------------------------------------
    // Case 2: Expired session → SessionExpired
    // -----------------------------------------------------------------------

    #[test]
    fn test_expired_session_returns_session_expired() {
        let env = make_env();
        // Start at t=0 so session is created with expires_at = SESSION_TTL (86400)
        setup_ledger(&env, 0);
        let client = setup_contract(&env);
        let (attestor, sk) = register_attestor(&env, &client);
        let session_id = client.create_session(&attestor);

        // Advance ledger past SESSION_TTL (86400s)
        setup_ledger(&env, 86_401);

        let subject = Address::generate(&env);
        let ts = env.ledger().timestamp();
        let mut data = Bytes::new(&env);
        data.push_back(0x02);
        let hash = compute_payload_hash(&env, &subject, ts, &data);
        let hash_bytes: Bytes = hash.into();
        let sig = sign_attestation(&env, &sk, &client.address, &attestor, &subject, ts, &hash_bytes);

        let result = client.try_submit_attestation_with_session(&session_id, &attestor, &subject, &ts, &hash_bytes, &sig, &None);
        assert!(result.is_err());
        let err = result.unwrap_err().unwrap();
        assert_eq!(err, soroban_sdk::Error::from_contract_error(ErrorCode::SessionExpired as u32));
    }

    // -----------------------------------------------------------------------
    // Case 3: Valid session → executes cleanly
    // -----------------------------------------------------------------------

    #[test]
    fn test_valid_session_executes_without_error() {
        let env = make_env();
        setup_ledger(&env, 1_000_000);
        let client = setup_contract(&env);
        let (attestor, sk) = register_attestor(&env, &client);
        let session_id = client.create_session(&attestor);

        let subject = Address::generate(&env);
        let ts = env.ledger().timestamp();
        let mut data = Bytes::new(&env);
        data.push_back(0x03);
        let hash = compute_payload_hash(&env, &subject, ts, &data);
        let hash_bytes: Bytes = hash.into();
        let sig = sign_attestation(&env, &sk, &client.address, &attestor, &subject, ts, &hash_bytes);

        // Should succeed without panic
        let result = client.try_submit_attestation_with_session(&session_id, &attestor, &subject, &ts, &hash_bytes, &sig, &None);
        assert!(result.is_ok());
    }
}
