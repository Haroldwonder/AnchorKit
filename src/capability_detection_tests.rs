#![cfg(test)]

mod capability_detection_tests {
    use soroban_sdk::{testutils::Address as _, Address, Env, Vec};
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    use crate::contract::{
        AnchorKitContract, AnchorKitContractClient, ServiceType,
        SERVICE_DEPOSITS, SERVICE_WITHDRAWALS, SERVICE_QUOTES, SERVICE_KYC,
    };
    use crate::sep10_test_util::register_attestor_with_sep10;

    fn make_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    fn setup(env: &Env) -> (AnchorKitContractClient, Address) {
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin);
        (client, admin)
    }

    fn register(env: &Env, client: &AnchorKitContractClient, admin: &Address) -> Address {
        let anchor = Address::generate(env);
        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(env, client, &anchor, admin, &sk);
        anchor
    }

    fn services(env: &Env, vals: &[u32]) -> Vec<u32> {
        let mut v = Vec::new(env);
        for &s in vals {
            v.push_back(s);
        }
        v
    }

    #[test]
    fn test_service_type_values() {
        assert_eq!(ServiceType::Deposits.as_u32(), SERVICE_DEPOSITS);
        assert_eq!(ServiceType::Withdrawals.as_u32(), SERVICE_WITHDRAWALS);
        assert_eq!(ServiceType::Quotes.as_u32(), SERVICE_QUOTES);
        assert_eq!(ServiceType::KYC.as_u32(), SERVICE_KYC);
    }

    #[test]
    fn test_detect_deposit_only_anchor() {
        let env = make_env();
        let (client, admin) = setup(&env);
        let anchor = register(&env, &client, &admin);

        client.configure_services(&anchor, &services(&env, &[SERVICE_DEPOSITS]));

        assert!(client.supports_service(&anchor, &SERVICE_DEPOSITS));
        assert!(!client.supports_service(&anchor, &SERVICE_WITHDRAWALS));
        assert!(!client.supports_service(&anchor, &SERVICE_QUOTES));
        assert!(!client.supports_service(&anchor, &SERVICE_KYC));
    }

    #[test]
    fn test_detect_withdrawal_only_anchor() {
        let env = make_env();
        let (client, admin) = setup(&env);
        let anchor = register(&env, &client, &admin);

        client.configure_services(&anchor, &services(&env, &[SERVICE_WITHDRAWALS]));

        assert!(!client.supports_service(&anchor, &SERVICE_DEPOSITS));
        assert!(client.supports_service(&anchor, &SERVICE_WITHDRAWALS));
    }

    #[test]
    fn test_detect_full_service_anchor() {
        let env = make_env();
        let (client, admin) = setup(&env);
        let anchor = register(&env, &client, &admin);

        client.configure_services(
            &anchor,
            &services(&env, &[SERVICE_DEPOSITS, SERVICE_WITHDRAWALS, SERVICE_QUOTES, SERVICE_KYC]),
        );

        assert!(client.supports_service(&anchor, &SERVICE_DEPOSITS));
        assert!(client.supports_service(&anchor, &SERVICE_WITHDRAWALS));
        assert!(client.supports_service(&anchor, &SERVICE_QUOTES));
        assert!(client.supports_service(&anchor, &SERVICE_KYC));
    }

    #[test]
    fn test_update_anchor_capabilities() {
        let env = make_env();
        let (client, admin) = setup(&env);
        let anchor = register(&env, &client, &admin);

        client.configure_services(&anchor, &services(&env, &[SERVICE_DEPOSITS]));
        assert!(client.supports_service(&anchor, &SERVICE_DEPOSITS));
        assert!(!client.supports_service(&anchor, &SERVICE_WITHDRAWALS));

        client.configure_services(&anchor, &services(&env, &[SERVICE_DEPOSITS, SERVICE_WITHDRAWALS]));
        assert!(client.supports_service(&anchor, &SERVICE_DEPOSITS));
        assert!(client.supports_service(&anchor, &SERVICE_WITHDRAWALS));
    }

    #[test]
    #[should_panic]
    fn test_reject_empty_services() {
        let env = make_env();
        let (client, admin) = setup(&env);
        let anchor = register(&env, &client, &admin);
        client.configure_services(&anchor, &services(&env, &[]));
    }

    #[test]
    #[should_panic]
    fn test_reject_duplicate_services() {
        let env = make_env();
        let (client, admin) = setup(&env);
        let anchor = register(&env, &client, &admin);
        client.configure_services(&anchor, &services(&env, &[SERVICE_DEPOSITS, SERVICE_DEPOSITS]));
    }

    #[test]
    #[should_panic]
    fn test_reject_unregistered_anchor_services() {
        let env = make_env();
        let (client, _) = setup(&env);
        let anchor = Address::generate(&env);
        client.configure_services(&anchor, &services(&env, &[SERVICE_DEPOSITS]));
    }

    #[test]
    #[should_panic]
    fn test_get_services_for_non_configured_anchor() {
        let env = make_env();
        let (client, admin) = setup(&env);
        let anchor = register(&env, &client, &admin);
        client.get_supported_services(&anchor);
    }
}
