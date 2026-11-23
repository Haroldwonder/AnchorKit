#![cfg(test)]

mod credential_storage_tests {
    use soroban_sdk::{
        symbol_short,
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Env, Error, String,
    };

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::errors::ErrorCode;

    fn set_ledger(env: &Env, sequence_number: u32) {
        env.ledger().set(LedgerInfo {
            timestamp: 1_000_000,
            protocol_version: 21,
            sequence_number,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });
    }

    fn register(env: &Env) -> AnchorKitContractClient {
        env.mock_all_auths();
        set_ledger(env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        AnchorKitContractClient::new(env, &contract_id)
    }

    #[test]
    fn credential_functions_fail_with_not_initialized_before_init() {
        let env = Env::default();
        let client = register(&env);
        let attestor = Address::generate(&env);
        let value = String::from_str(&env, "enc");
        let expected = Error::from_contract_error(ErrorCode::NotInitialized as u32);

        assert_eq!(
            client
                .try_set_credential_policy(&attestor, &3600u64, &true)
                .unwrap_err()
                .unwrap(),
            expected
        );
        assert_eq!(
            client
                .try_store_encrypted_credential(&attestor, &symbol_short!("api"), &value, &0u64)
                .unwrap_err()
                .unwrap(),
            expected
        );
        assert_eq!(
            client
                .try_rotate_credential(&attestor, &symbol_short!("api"), &value, &0u64)
                .unwrap_err()
                .unwrap(),
            expected
        );
    }

    #[test]
    fn credential_entries_survive_past_minimum_persistent_ttl() {
        let env = Env::default();
        let client = register(&env);
        let admin = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None, &None);

        let attestor = Address::generate(&env);
        let value = String::from_str(&env, "enc");
        client.set_credential_policy(&attestor, &3600u64, &true);
        client.store_encrypted_credential(&attestor, &symbol_short!("api"), &value, &2_000_000u64);

        // Well past the 4096-ledger minimum persistent TTL.
        set_ledger(&env, 10_000);

        assert!(client.get_credential_policy(&attestor).is_some());
        assert!(!client.check_credential_rotation(&attestor));

        client.rotate_credential(&attestor, &symbol_short!("api"), &value, &2_000_000u64);
        set_ledger(&env, 20_000);
        assert!(client.get_credential_policy(&attestor).is_some());
    }
}
