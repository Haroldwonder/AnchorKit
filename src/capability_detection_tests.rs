#![cfg(test)]

mod capability_detection_tests {
    use soroban_sdk::{testutils::Address as _, Address, Env, Vec};
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    use crate::contract::{
        AnchorKitContract, AnchorKitContractClient, ServiceType,
        SERVICE_DEPOSITS, SERVICE_WITHDRAWALS, SERVICE_QUOTES, SERVICE_KYC, SERVICE_EXCHANGE_QUOTES,
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
        client.initialize(&admin, &100_u64, &None, &None);
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
        assert_eq!(ServiceType::Deposits.as_u32(), 1u32);
        assert_eq!(ServiceType::Withdrawals.as_u32(), 2u32);
        assert_eq!(ServiceType::Quotes.as_u32(), 3u32);
        assert_eq!(ServiceType::KYC.as_u32(), 4u32);
        assert_eq!(ServiceType::ExchangeQuotes.as_u32(), 5u32);
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

    #[test]
    fn test_cache_capabilities_accepts_known_codes() {
        let env = make_env();
        let (client, admin) = setup(&env);
        let anchor = register(&env, &client, &admin);

        let caps = services(&env, &[SERVICE_DEPOSITS, SERVICE_WITHDRAWALS, SERVICE_QUOTES]);
        let res = client.try_cache_capabilities(&anchor, &caps);
        assert!(res.is_ok());

        let stored = client.get_cached_capabilities(&anchor);
        assert_eq!(stored.len(), 3);
        assert_eq!(stored.get(0).unwrap(), SERVICE_DEPOSITS);
        assert_eq!(stored.get(1).unwrap(), SERVICE_WITHDRAWALS);
        assert_eq!(stored.get(2).unwrap(), SERVICE_QUOTES);
    }

    #[test]
    fn test_cache_capabilities_rejects_unknown_code() {
        let env = make_env();
        let (client, admin) = setup(&env);
        let anchor = register(&env, &client, &admin);

        let caps = services(&env, &[SERVICE_DEPOSITS, 99u32]);
        let res = client.try_cache_capabilities(&anchor, &caps);
        assert_eq!(res, Err(Ok(Error::InvalidServiceType)));
    }

    #[test]
    fn test_cache_capabilities_rejects_duplicate_codes() {
        let env = make_env();
        let (client, admin) = setup(&env);
        let anchor = register(&env, &client, &admin);

        let caps = services(&env, &[SERVICE_DEPOSITS, SERVICE_DEPOSITS]);
        let res = client.try_cache_capabilities(&anchor, &caps);
        assert_eq!(res, Err(Ok(Error::InvalidServiceType)));
    }

    #[test]
    fn test_cache_capabilities_rejects_empty_list() {
        let env = make_env();
        let (client, admin) = setup(&env);
        let anchor = register(&env, &client, &admin);

        let caps = services(&env, &[]);
        let res = client.try_cache_capabilities(&anchor, &caps);
        assert_eq!(res, Err(Ok(Error::InvalidServiceType)));
    }
}
