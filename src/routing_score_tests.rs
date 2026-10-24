#![cfg(test)]

mod routing_score_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Env, String, Symbol, Vec,
    };

    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::sep10_test_util::register_attestor_with_sep10;
    use crate::types::{RoutingOptions, RoutingRequest};

    fn setup(env: &Env) -> AnchorKitContractClient {
        env.mock_all_auths();
        env.ledger().set(LedgerInfo {
            timestamp: 1_000_000,
            protocol_version: 21,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin, &100_u64, &None, &None);
        client
    }

    fn add_anchor(
        env: &Env,
        client: &AnchorKitContractClient,
        fee: u32,
        settlement_time: u64,
    ) -> Address {
        let anchor = Address::generate(env);
        let signing_key = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(env, client, &anchor, &anchor, &signing_key);
        let mut services = Vec::new(env);
        services.push_back(1u32);
        services.push_back(3u32);
        client.configure_services(&anchor, &services);
        client.set_anchor_metadata(
            &anchor,
            &5000u32,
            &settlement_time,
            &7500u32,
            &9900u32,
            &1_000_000u64,
            &None::<String>,
        );
        client.submit_quote(
            &anchor,
            &String::from_str(env, "USD"),
            &String::from_str(env, "USDC"),
            &10000u64,
            &fee,
            &100u64,
            &100000u64,
            &1_003_600u64,
        );
        anchor
    }

    fn options(env: &Env, strategy_name: &str) -> RoutingOptions {
        let mut strategy = Vec::new(env);
        strategy.push_back(Symbol::new(env, strategy_name));
        RoutingOptions {
            request: RoutingRequest {
                base_asset: String::from_str(env, "USD"),
                quote_asset: String::from_str(env, "USDC"),
                amount: 5000,
                operation_type: 1,
            },
            strategy,
            min_reputation: 0,
            max_anchors: 5,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(env),
        }
    }

    #[test]
    fn balanced_zero_fee_quote_beats_paid_quote() {
        let env = Env::default();
        let client = setup(&env);
        let paid = add_anchor(&env, &client, 1, 300);
        let free = add_anchor(&env, &client, 0, 300);

        let best = client.route_transaction(&options(&env, "Balanced"));
        assert_eq!(best.anchor, free);
        assert_ne!(best.anchor, paid);
    }

    #[test]
    fn balanced_instant_settlement_beats_timed_settlement() {
        let env = Env::default();
        let client = setup(&env);
        let timed = add_anchor(&env, &client, 10, 1);
        let instant = add_anchor(&env, &client, 10, 0);

        let best = client.route_transaction(&options(&env, "Balanced"));
        assert_eq!(best.anchor, instant);
        assert_ne!(best.anchor, timed);
    }

    #[test]
    fn weighted_never_selects_zero_weight_anchor() {
        let env = Env::default();
        let client = setup(&env);
        // The zero-weight anchor is registered first so it sits at index 0.
        let unhealthy = add_anchor(&env, &client, 10, 300);
        let healthy = add_anchor(&env, &client, 10, 300);
        client.update_health_status(&unhealthy, &100u64, &0u32, &0u32);
        client.update_health_status(&healthy, &100u64, &0u32, &100u32);

        for _ in 0..50 {
            let best = client.route_transaction(&options(&env, "Weighted"));
            assert_eq!(best.anchor, healthy);
        }
    }
}
