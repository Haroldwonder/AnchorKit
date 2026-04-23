#![cfg(test)]

mod sep10_contract_tests {
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
    use soroban_sdk::{Address, Bytes, Env, String};

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::sep10_test_util::{build_sep10_jwt, register_attestor_with_sep10};

    fn make_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    fn ledger(env: &Env, ts: u64) {
        env.ledger().set(LedgerInfo {
            timestamp: ts,
            protocol_version: 21,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });
    }

    #[test]
    fn contract_verify_sep10_token_succeeds() {
        let env = make_env();
        ledger(&env, 1000);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        client.initialize(&admin);

        let sk = SigningKey::generate(&mut OsRng);
        let pk = Bytes::from_slice(&env, sk.verifying_key().as_bytes());
        client.set_sep10_jwt_verifying_key(&issuer, &pk);

        let attestor = Address::generate(&env);
        let sub = attestor.to_string();
        let mut buf = [0u8; 128];
        let len = sub.len() as usize;
        let final_len = if len > 128 { 128 } else { len };
        sub.copy_into_slice(&mut buf[..final_len]);
        let sub_std = core::str::from_utf8(&buf[..final_len]).unwrap_or("");
        let jwt = build_sep10_jwt(&sk, sub_std, 2000);
        let token = String::from_str(&env, jwt.as_str());
        client.verify_sep10_token(&token, &issuer);
    }

    #[test]
    fn contract_register_attestor_with_sep10_roundtrip() {
        let env = make_env();
        ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let issuer = Address::generate(&env);
        client.initialize(&admin);

        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &issuer, &sk);
        assert!(client.is_attestor(&attestor));
    }

    #[test]
    fn key_rotation_old_key_still_works_during_window() {
        let env = make_env();
        ledger(&env, 1000);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        client.initialize(&admin);

        // Set initial key
        let old_sk = SigningKey::generate(&mut OsRng);
        let old_pk = Bytes::from_slice(&env, old_sk.verifying_key().as_bytes());
        client.set_sep10_jwt_verifying_key(&issuer, &old_pk);

        // Add new key (rotation begins)
        let new_sk = SigningKey::generate(&mut OsRng);
        let new_pk = Bytes::from_slice(&env, new_sk.verifying_key().as_bytes());
        client.add_sep10_verifying_key(&issuer, &new_pk);

        // Token signed with old key still verifies
        let jwt_old = build_sep10_jwt(&old_sk, "any", 2000);
        let token_old = String::from_str(&env, jwt_old.as_str());
        client.verify_sep10_token(&token_old, &issuer);

        // Token signed with new key also verifies
        let jwt_new = build_sep10_jwt(&new_sk, "any", 2000);
        let token_new = String::from_str(&env, jwt_new.as_str());
        client.verify_sep10_token(&token_new, &issuer);
    }

    #[test]
    fn key_rotation_old_key_rejected_after_removal() {
        let env = make_env();
        ledger(&env, 1000);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        client.initialize(&admin);

        let old_sk = SigningKey::generate(&mut OsRng);
        let old_pk = Bytes::from_slice(&env, old_sk.verifying_key().as_bytes());
        client.set_sep10_jwt_verifying_key(&issuer, &old_pk);

        let new_sk = SigningKey::generate(&mut OsRng);
        let new_pk = Bytes::from_slice(&env, new_sk.verifying_key().as_bytes());
        client.add_sep10_verifying_key(&issuer, &new_pk);

        // Remove old key after rotation window
        client.remove_sep10_verifying_key(&issuer, &old_pk);

        // Old key token is now rejected
        let jwt_old = build_sep10_jwt(&old_sk, "any", 2000);
        let token_old = String::from_str(&env, jwt_old.as_str());
        let result = std::panic::catch_unwind(|| {
            client.verify_sep10_token(&token_old, &issuer);
        });
        assert!(result.is_err(), "old key should be rejected after removal");

        // New key token still works
        let jwt_new = build_sep10_jwt(&new_sk, "any", 2000);
        let token_new = String::from_str(&env, jwt_new.as_str());
        client.verify_sep10_token(&token_new, &issuer);
    }

    #[test]
    fn max_key_count_enforced() {
        let env = make_env();
        ledger(&env, 1000);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        client.initialize(&admin);

        // Fill up to max (3)
        for _ in 0..3 {
            let sk = SigningKey::generate(&mut OsRng);
            let pk = Bytes::from_slice(&env, sk.verifying_key().as_bytes());
            client.add_sep10_verifying_key(&issuer, &pk);
        }

        // 4th key should panic
        let sk = SigningKey::generate(&mut OsRng);
        let pk = Bytes::from_slice(&env, sk.verifying_key().as_bytes());
        let result = std::panic::catch_unwind(|| {
            client.add_sep10_verifying_key(&issuer, &pk);
        });
        assert!(result.is_err(), "should reject exceeding max key count");
    }
}
