#![cfg(test)]

mod pause_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Bytes, Env,
    };
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::sep10_test_util::{register_attestor_with_sep10, sign_payload};

    fn setup() -> (Env, AnchorKitContractClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set(LedgerInfo {
            timestamp: 1_700_000_000,
            protocol_version: 21,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 100,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6_312_000,
        });
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None, &None);
        (env, client, admin)
    }

    // -----------------------------------------------------------------------
    // is_paused default
    // -----------------------------------------------------------------------

    #[test]
    fn is_paused_false_by_default() {
        let (_env, client, _admin) = setup();
        assert!(!client.is_paused());
    }

    // -----------------------------------------------------------------------
    // pause / unpause basic lifecycle
    // -----------------------------------------------------------------------

    #[test]
    fn pause_sets_paused_flag() {
        let (_env, client, _admin) = setup();
        client.pause_contract();
        assert!(client.is_paused());
    }

    #[test]
    fn unpause_clears_paused_flag() {
        let (_env, client, _admin) = setup();
        client.pause_contract();
        client.unpause_contract();
        assert!(!client.is_paused());
    }

    // -----------------------------------------------------------------------
    // submit_attestation blocked while paused
    // -----------------------------------------------------------------------

    #[test]
    #[should_panic(expected = "HostError: Error(Contract, #122)")]
    fn submit_attestation_rejected_when_paused() {
        let (env, client, _admin) = setup();
        let attestor = Address::generate(&env);
        let mut csprng = OsRng;
        let key = SigningKey::generate(&mut csprng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &key);

        client.pause_contract();

        let subject = Address::generate(&env);
        let ts = env.ledger().timestamp();
        let hash = {
            let mut b = Bytes::new(&env);
            for i in 0u8..32 { b.push_back(i); }
            b
        };
        let sig = sign_payload(&env, &key, &hash);
        client.submit_attestation(&attestor, &subject, &ts, &hash, &sig);
    }

    // -----------------------------------------------------------------------
    // register_attestor blocked while paused
    // -----------------------------------------------------------------------

    #[test]
    #[should_panic(expected = "HostError: Error(Contract, #122)")]
    fn register_attestor_rejected_when_paused() {
        let (env, client, _admin) = setup();
        let attestor = Address::generate(&env);
        let mut csprng = OsRng;
        let key = SigningKey::generate(&mut csprng);

        client.pause_contract();

        // attempt to register while paused — must panic with ContractPaused (#122)
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &key);
    }

    // -----------------------------------------------------------------------
    // Operations succeed again after unpause
    // -----------------------------------------------------------------------

    #[test]
    fn submit_attestation_succeeds_after_unpause() {
        let (env, client, _admin) = setup();
        let attestor = Address::generate(&env);
        let mut csprng = OsRng;
        let key = SigningKey::generate(&mut csprng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &key);

        client.pause_contract();
        client.unpause_contract();

        let subject = Address::generate(&env);
        let ts = env.ledger().timestamp();
        let hash = {
            let mut b = Bytes::new(&env);
            for i in 0u8..32 { b.push_back(i); }
            b
        };
        let sig = sign_payload(&env, &key, &hash);
        // must not panic
        client.submit_attestation(&attestor, &subject, &ts, &hash, &sig);
    }

    // -----------------------------------------------------------------------
    // submit_with_request_id blocked while paused
    // -----------------------------------------------------------------------

    #[test]
    #[should_panic(expected = "HostError: Error(Contract, #122)")]
    fn submit_with_request_id_rejected_when_paused() {
        let (env, client, _admin) = setup();
        let attestor = Address::generate(&env);
        let mut csprng = OsRng;
        let key = SigningKey::generate(&mut csprng);
        register_attestor_with_sep10(&env, &client, &attestor, &attestor, &key);

        client.pause_contract();

        let subject = Address::generate(&env);
        let ts = env.ledger().timestamp();
        let hash = {
            let mut b = Bytes::new(&env);
            for i in 0u8..32 { b.push_back(i); }
            b
        };
        let sig = sign_payload(&env, &key, &hash);
        let request_id = Bytes::from_slice(&env, &[0u8; 32]);
        // submit_with_request_id should respect pause like submit_attestation does
        client.submit_with_request_id(&request_id, &attestor, &subject, &ts, &hash, &sig, &None);
    }
}
