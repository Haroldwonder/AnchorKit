#![cfg(test)]

mod issue_1355_1362_1364_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Env, String,
    };

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set(LedgerInfo {
            timestamp: 1_000_000,
            protocol_version: 21,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6_312_000,
        });
        let contract_id = env.register_contract(None, AnchorKitContract);
        let admin = Address::generate(&env);
        let client = AnchorKitContractClient::new(&env, &contract_id);
        client.initialize(&admin, &100_u64, &None, &None);
        (env, contract_id, admin)
    }

    #[test]
    fn set_anchor_metadata_rejects_scores_above_10000() {
        let (env, contract_id, _) = setup();
        let client = AnchorKitContractClient::new(&env, &contract_id);
        let anchor = Address::generate(&env);
        assert!(client
            .try_set_anchor_metadata(
                &anchor,
                &10001_u32,
                &300_u64,
                &7500_u32,
                &9900_u32,
                &1_000_000_u64,
                &None::<String>,
            )
            .is_err());
        assert!(client
            .try_set_anchor_metadata(
                &anchor,
                &9000_u32,
                &300_u64,
                &7500_u32,
                &10001_u32,
                &1_000_000_u64,
                &None::<String>,
            )
            .is_err());
    }

    #[test]
    fn update_health_status_rejects_availability_above_100() {
        let (env, contract_id, _) = setup();
        let client = AnchorKitContractClient::new(&env, &contract_id);
        let anchor = Address::generate(&env);
        assert!(client
            .try_update_health_status(&anchor, &100_u64, &0_u32, &101_u32)
            .is_err());
        assert!(client.try_get_health_status(&anchor).is_err());
    }

    #[test]
    fn admin_operations_reject_a_session_created_by_another_user() {
        let (env, contract_id, admin) = setup();
        let client = AnchorKitContractClient::new(&env, &contract_id);
        let unrelated_user = Address::generate(&env);
        let session_id = client.create_session(&unrelated_user);
        let attestor = Address::generate(&env);
        let empty = String::from_str(&env, "");

        assert!(client
            .try_register_attestor_with_session(
                &session_id,
                &attestor,
                &empty,
                &attestor,
            )
            .is_err());
        assert!(client
            .try_revoke_attestor_with_session(&session_id, &attestor)
            .is_err());
        assert_eq!(client.get_session_operation_count(&session_id), Some(0));
        assert_eq!(client.get_admin(), admin);
    }
}
