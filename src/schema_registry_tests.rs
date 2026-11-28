#![cfg(test)]

//! Tests for the attestation payload schema registry (#1514).

mod schema_registry_tests {
    use ed25519_dalek::{Signer, SigningKey};
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Bytes, Env, String, Symbol,
    };

    use crate::contract::{
        AnchorKitContract, AnchorKitContractClient, AttestationInput, RequestId,
    };
    use crate::sep10_test_util::{register_attestor_with_sep10, sign_attestation};
    use crate::storage::key_admin;

    fn signing_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn schema_id(env: &Env) -> Symbol {
        Symbol::new(env, "kyc_result_v1")
    }

    fn payload(env: &Env, byte: u8) -> Bytes {
        Bytes::from_slice(env, &[byte; 32])
    }

    macro_rules! setup {
        ($env:ident, $client:ident, $admin:ident, $issuer:ident, $subject:ident) => {
            let $env = Env::default();
            $env.mock_all_auths();
            $env.ledger().set(LedgerInfo {
                timestamp: 1_700_000_000,
                protocol_version: 21,
                sequence_number: 100,
                network_id: Default::default(),
                base_reserve: 100,
                min_persistent_entry_ttl: 4096,
                min_temp_entry_ttl: 16,
                max_entry_ttl: 6_312_000,
            });
            let contract_id = $env.register_contract(None, AnchorKitContract);
            let $client = AnchorKitContractClient::new(&$env, &contract_id);
            let $admin = Address::generate(&$env);
            let $issuer = Address::generate(&$env);
            let $subject = Address::generate(&$env);
            // The admin is written to instance storage at deploy time in
            // production; seed it directly so this test is self-contained.
            $env.as_contract(&contract_id, || {
                $env.storage()
                    .instance()
                    .set(&key_admin(&$env), &$admin);
            });
            $client.initialize(&100_u64, &None, &None);
            register_attestor_with_sep10(&$env, &$client, &$issuer, &$issuer, &signing_key());
        };
    }

    #[test]
    fn admin_can_register_read_and_remove_a_schema() {
        setup!(env, client, _admin, _issuer, _subject);
        let id = schema_id(&env);
        assert!(client.get_attestation_schema(&id).is_none());

        client.register_attestation_schema(
            &id,
            &String::from_str(&env, "KYC result"),
            &String::from_str(&env, "1.2.0"),
            &String::from_str(&env, "https://example.org/schemas/kyc-v1.json"),
        );

        let record = client.get_attestation_schema(&id).unwrap();
        assert_eq!(record.name, String::from_str(&env, "KYC result"));
        assert_eq!(record.version, String::from_str(&env, "1.2.0"));
        assert_eq!(
            record.uri,
            String::from_str(&env, "https://example.org/schemas/kyc-v1.json")
        );

        client.remove_attestation_schema(&id);
        assert!(client.get_attestation_schema(&id).is_none());
    }

    #[test]
    fn schema_registration_rejects_invalid_metadata() {
        setup!(env, client, _admin, _issuer, _subject);
        let id = schema_id(&env);
        let name = String::from_str(&env, "Name");
        let version = String::from_str(&env, "1.0.0");
        let uri = String::from_str(&env, "https://example.org/schema.json");

        assert!(client
            .try_register_attestation_schema(&id, &String::from_str(&env, ""), &version, &uri,)
            .is_err());
        assert!(client
            .try_register_attestation_schema(&id, &name, &String::from_str(&env, ""), &uri,)
            .is_err());
        assert!(client
            .try_register_attestation_schema(&id, &name, &version, &String::from_str(&env, ""),)
            .is_err());
        let long_uri = String::from_str(&env, &"x".repeat(513));
        assert!(client
            .try_register_attestation_schema(&id, &name, &version, &long_uri)
            .is_err());

        assert!(client.get_attestation_schema(&id).is_none());
    }

    #[test]
    fn plain_submission_stores_registered_schema_id() {
        setup!(env, client, _admin, issuer, subject);
        let id = schema_id(&env);
        client.register_attestation_schema(
            &id,
            &String::from_str(&env, "KYC result"),
            &String::from_str(&env, "1.0.0"),
            &String::from_str(&env, "https://example.org/kyc.json"),
        );

        let ts = env.ledger().timestamp();
        let hash = payload(&env, 0x11);
        let signature = sign_attestation(
            &env,
            &signing_key(),
            &client.address,
            &issuer,
            &subject,
            ts,
            &hash,
        );
        let attestation_id = client.submit_attestation_with_schema(
            &issuer,
            &subject,
            &ts,
            &hash,
            &signature,
            &Some(id.clone()),
        );

        let stored = client.get_attestation(&attestation_id).unwrap();
        assert_eq!(stored.schema_id, Some(id));
    }

    #[test]
    fn legacy_submission_has_no_schema_id() {
        setup!(env, client, _admin, issuer, subject);
        let ts = env.ledger().timestamp();
        let hash = payload(&env, 0x12);
        let signature = sign_attestation(
            &env,
            &signing_key(),
            &client.address,
            &issuer,
            &subject,
            ts,
            &hash,
        );
        let attestation_id = client.submit_attestation(&issuer, &subject, &ts, &hash, &signature);

        assert!(client
            .get_attestation(&attestation_id)
            .unwrap()
            .schema_id
            .is_none());
    }

    #[test]
    fn submission_with_unregistered_schema_is_rejected() {
        setup!(env, client, _admin, issuer, subject);
        let unknown = Symbol::new(&env, "unknown_schema");
        let ts = env.ledger().timestamp();

        assert!(client
            .try_submit_attestation_with_schema(
                &issuer,
                &subject,
                &ts,
                &payload(&env, 0x13),
                &Bytes::new(&env),
                &Some(unknown),
            )
            .is_err());
    }

    #[test]
    fn removed_schema_cannot_be_used_by_new_submissions() {
        setup!(env, client, _admin, issuer, subject);
        let id = schema_id(&env);
        client.register_attestation_schema(
            &id,
            &String::from_str(&env, "KYC result"),
            &String::from_str(&env, "1.0.0"),
            &String::from_str(&env, "https://example.org/kyc.json"),
        );
        client.remove_attestation_schema(&id);

        assert!(client
            .try_submit_attestation_with_schema(
                &issuer,
                &subject,
                &env.ledger().timestamp(),
                &payload(&env, 0x14),
                &Bytes::new(&env),
                &Some(id),
            )
            .is_err());
    }

    #[test]
    fn batch_submission_stores_schema_id() {
        setup!(env, client, _admin, issuer, subject);
        let id = schema_id(&env);
        client.register_attestation_schema(
            &id,
            &String::from_str(&env, "Deposit receipt"),
            &String::from_str(&env, "1.0.0"),
            &String::from_str(&env, "https://example.org/deposit.json"),
        );

        let hash = payload(&env, 0x15);
        let signature = Bytes::from_slice(&env, &signing_key().sign(&[0x15u8; 32]).to_bytes());
        let mut inputs = soroban_sdk::Vec::new(&env);
        inputs.push_back(AttestationInput {
            subject: subject.clone(),
            timestamp: env.ledger().timestamp(),
            payload_hash: hash,
            signature,
            schema_id: Some(id.clone()),
        });

        let ids = client.submit_attestation_batch(&issuer, &inputs);
        assert_eq!(
            client
                .get_attestation(&ids.get(0).unwrap())
                .unwrap()
                .schema_id,
            Some(id)
        );
    }

    #[test]
    fn request_id_and_session_submissions_store_schema_id() {
        setup!(env, client, _admin, issuer, subject);
        let id = schema_id(&env);
        client.register_attestation_schema(
            &id,
            &String::from_str(&env, "Quote acceptance"),
            &String::from_str(&env, "2.0.0"),
            &String::from_str(&env, "https://example.org/quote.json"),
        );

        let ts = env.ledger().timestamp();

        // Request-ID path.
        let request_hash = payload(&env, 0x16);
        let request_sig = sign_attestation(
            &env,
            &signing_key(),
            &client.address,
            &issuer,
            &subject,
            ts,
            &request_hash,
        );
        let request_id = RequestId {
            id: Bytes::from_slice(&env, &[9u8; 32]),
            created_at: ts,
        };
        let request_attestation_id = client.submit_request_with_schema(
            &request_id,
            &issuer,
            &subject,
            &ts,
            &request_hash,
            &request_sig,
            &None,
            &Some(id.clone()),
        );
        assert_eq!(
            client
                .get_attestation(&request_attestation_id)
                .unwrap()
                .schema_id,
            Some(id.clone())
        );

        // Session path.
        let session_id = client.create_session(&issuer);
        let session_hash = payload(&env, 0x17);
        let session_sig = sign_attestation(
            &env,
            &signing_key(),
            &client.address,
            &issuer,
            &subject,
            ts,
            &session_hash,
        );
        let session_attestation_id = client.submit_session_with_schema(
            &session_id,
            &issuer,
            &subject,
            &ts,
            &session_hash,
            &session_sig,
            &None,
            &Some(id.clone()),
        );
        assert_eq!(
            client
                .get_attestation(&session_attestation_id)
                .unwrap()
                .schema_id,
            Some(id)
        );
    }
}
