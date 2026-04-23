#![cfg(test)]

mod is_initialized_tests {
    use soroban_sdk::{testutils::Address as _, Address, Env};

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};

    fn make_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    #[test]
    fn returns_false_before_initialize() {
        let env = make_env();
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        assert!(!client.is_initialized());
    }

    #[test]
    fn returns_true_after_initialize() {
        let env = make_env();
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin);

        assert!(client.is_initialized());
    }
}
