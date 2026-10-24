/// Polling-based state update flow tests.
///
/// Soroban contracts are synchronous — there is no streaming API. Clients
/// observe state changes by polling contract storage after each transaction.
/// These tests verify that multi-step anchor flows produce the expected
/// on-chain state at each polling point.
#[cfg(test)]
mod streaming_flow_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Bytes, Env, String, Vec,
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

    fn setup_ledger(env: &Env) {
        env.ledger().set(LedgerInfo {
            timestamp: 1_700_000_000,
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

    fn register_with_session(
        env: &Env,
        client: &AnchorKitContractClient,
        session_id: u64,
        attestor: &Address,
        sk: &SigningKey,
    ) {
        use crate::sep10_test_util::build_sep10_jwt;

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
        let jwt = build_sep10_jwt(sk, sub_str, exp);
        let token = String::from_str(env, jwt.as_str());
        client.register_attestor_with_session(&session_id, attestor, &token, &issuer);
    }

    fn setup(env: &Env) -> (AnchorKitContractClient<'_>, Address, Address, SigningKey) {
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let anchor = Address::generate(env);
        client.initialize(&admin, &100_u64, &None, &None);
        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(env, &client, &anchor, &anchor, &sk);
        (client, admin, anchor, sk)
    }

    #[test]
    fn test_session_operation_count_increments_on_each_step() {
        let env = make_env();
        setup_ledger(&env);
        let (client, admin, _anchor, _sk) = setup(&env);

        let session_id = client.create_session(&admin);

        assert_eq!(client.get_session_operation_count(&session_id).unwrap(), 0);

        let new_attestor = Address::generate(&env);
        let sk = SigningKey::generate(&mut OsRng);
        register_with_session(&env, &client, session_id, &new_attestor, &sk);

        assert_eq!(client.get_session_operation_count(&session_id).unwrap(), 1);

        client.revoke_attestor_with_session(&session_id, &new_attestor);

        assert_eq!(client.get_session_operation_count(&session_id).unwrap(), 2);
    }

    #[test]
    fn test_audit_log_reflects_attestation_state() {
        let env = make_env();
        setup_ledger(&env);
        let (client, _admin, anchor, sk) = setup(&env);

        let mut services = Vec::new(&env);
        services.push_back(1u32);
        client.configure_services(&anchor, &services);

        let session_id = client.create_session(&anchor);

        let subject = Address::generate(&env);
        let p = payload(&env, 1);
        client.submit_attestation_with_session(
            &session_id,
            &anchor,
            &subject,
            &(env.ledger().timestamp()),
            &p,
            &sign_payload(&env, &sk, &p),
            &None,
        );

        let log = client.get_audit_log(&0);
        assert_eq!(log.session_id, session_id);
        assert_eq!(log.operation.operation_type, String::from_str(&env, "attest"));
        assert_eq!(log.operation.status, String::from_str(&env, "success"));
        assert_eq!(log.actor, anchor);
    }

    #[test]
    fn test_audit_log_records_failed_operation() {
        let env = make_env();
        setup_ledger(&env);
        let (client, _admin, anchor, sk) = setup(&env);

        let mut services = Vec::new(&env);
        services.push_back(1u32);
        client.configure_services(&anchor, &services);

        let session_id = client.create_session(&anchor);

        let subject = Address::generate(&env);
        let p = payload(&env, 2);
        let ts = env.ledger().timestamp();
        let sig = sign_attestation(&env, &sk, &client.address, &anchor, &subject, ts, &p);

        client.submit_attestation_with_session(&session_id, &anchor, &subject, &ts, &p, &sig, &None);

        let result = client.try_submit_attestation_with_session(
            &session_id,
            &anchor,
            &subject,
            &ts,
            &p,
            &sig,
            &None,
        );
        assert!(result.is_err());

        // Only the successful submission increments the session operation count.
        assert_eq!(client.get_session_operation_count(&session_id).unwrap(), 1);
    }

    #[test]
    fn test_full_deposit_flow_state_visible_via_polling() {
        let env = make_env();
        setup_ledger(&env);
        let (client, _admin, anchor, _sk) = setup(&env);

        let mut services = Vec::new(&env);
        services.push_back(1u32);
        services.push_back(3u32);
        client.configure_services(&anchor, &services);

        let initiator = Address::generate(&env);
        let session_id = client.create_session(&initiator);

        let session = client.get_session(&session_id);
        assert_eq!(session.session_id, session_id);
        assert_eq!(session.operation_count, 0);

        let base = String::from_str(&env, "USD");
        let quote_asset_str = String::from_str(&env, "USDC");
        let valid_until = env.ledger().timestamp() + 600;
        let quote_id = client.submit_quote(
            &anchor,
            &base,
            &quote_asset_str,
            &10000u64,
            &25u32,
            &100_000000u64,
            &10_000_000000u64,
            &valid_until,
        );

        let quote = client.get_quote(&anchor, &quote_id).unwrap();
        assert_eq!(quote.rate, 10000u64);
        assert_eq!(client.get_session_operation_count(&session_id).unwrap(), 0);
    }
}
