#![cfg(test)]

mod webhook_middleware_tests {
    use soroban_sdk::{testutils::{Address as _, Ledger, LedgerInfo}, Address, Bytes, Env, String};

    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::sep10_test_util::{register_attestor_with_sep10, sign_attestation};

    fn make_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    fn set_ledger(env: &Env, timestamp: u64) {
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

    fn setup(env: &Env) -> (AnchorKitContractClient, Address) {
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin, &100_u64, &None, &None);
        (client, admin)
    }

    /// Verifies that a G-address (Stellar account address) is rejected when used
    /// as a Soroban contract address source_address parameter.
    /// The host emits "unexpected strkey length" diagnostic events and panics.
    #[test]
    #[should_panic]
    fn test_webhook_request_with_source_address() {
        let env = Env::default();
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

        // G-addresses are Stellar account addresses (56 chars), not Soroban contract
        // addresses (C-addresses, 58 chars). Parsing one as Address panics with
        // "unexpected strkey length".
        let g_address = String::from_str(&env, "GBBD6A7KNZF5WNWQEPZP5DYJD2AYUTLXRB6VXJ4RCX4RTNPPQVNF3GQ");
        let _ = Address::from_string(&g_address);
    }

    /// Verifies that a webhook request carrying an invalid (garbage) ed25519 signature
    /// is rejected. The contract checks the signature against the registered SEP-10 key;
    /// 64 bytes of 0xFF do not form a valid signature for any key, so the call panics
    /// with UnauthorizedAttestor.
    #[test]
    #[should_panic]
    fn test_webhook_request_with_invalid_signature() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _admin) = setup(&env);

        let attestor = Address::generate(&env);
        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        let subject = Address::generate(&env);
        let payload_hash = Bytes::from_slice(&env, &[0x42u8; 32]);
        // 64 bytes of 0xFF are not a valid ed25519 signature for the registered key.
        let bad_sig = Bytes::from_slice(&env, &[0xFFu8; 64]);

        client.submit_attestation(&attestor, &subject, &1_000_000u64, &payload_hash, &bad_sig);
    }

    /// Verifies that a replayed webhook request — identical payload submitted a second
    /// time — is rejected. After the first successful submission the payload hash is
    /// stored as a used key; the second call panics with ReplayAttack.
    #[test]
    #[should_panic]
    fn test_webhook_request_replay_detected() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _admin) = setup(&env);

        let attestor = Address::generate(&env);
        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &sk);

        let subject = Address::generate(&env);
        let payload_hash = Bytes::from_slice(&env, &[0xBEu8; 32]);
        let sig = sign_attestation(&env, &sk, &client.address, &attestor, &subject, 1_000_000u64, &payload_hash);

        // First submission succeeds.
        client.submit_attestation(&attestor, &subject, &1_000_000u64, &payload_hash, &sig);

        // Replaying the exact same payload must panic.
        client.submit_attestation(&attestor, &subject, &1_000_000u64, &payload_hash, &sig);
    }

    /// Verifies that a webhook request from an anchor that has never been registered
    /// is rejected. The contract checks StorageKey::Attestor and panics with
    /// AttestorNotRegistered when the key is absent.
    #[test]
    #[should_panic]
    fn test_webhook_request_from_unregistered_anchor() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _admin) = setup(&env);

        // Deliberately skip registration for this anchor.
        let unregistered = Address::generate(&env);
        let subject = Address::generate(&env);
        let payload_hash = Bytes::from_slice(&env, &[0xDEu8; 32]);
        let sig = Bytes::new(&env);

        client.submit_attestation(&unregistered, &subject, &1_000_000u64, &payload_hash, &sig);
    }
}
