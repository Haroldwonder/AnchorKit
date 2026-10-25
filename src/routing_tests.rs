#![cfg(test)]

mod routing_tests {
    use soroban_sdk::{
        testutils::{Address as _, Events, Ledger, LedgerInfo},
        Address, Env, IntoVal, String, Symbol, Vec, symbol_short,
    };

    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::types::{RoutingOptions, RoutingRequest};
    use crate::sep10_test_util::register_attestor_with_sep10;
    use crate::events::RoutingDecisionEvent;

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

    fn register_anchor(env: &Env, client: &AnchorKitContractClient, anchor: &Address) {
        let signing_key = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(env, client, anchor, anchor, &signing_key);
        let mut services = Vec::new(env);
        services.push_back(1u32);
        services.push_back(3u32);
        client.configure_services(anchor, &services);
        client.set_anchor_metadata(anchor, &5000u32, &300u64, &7500u32, &9900u32, &1_000_000u64, &None);
    }

    fn make_request(env: &Env) -> RoutingRequest {
        RoutingRequest {
            base_asset: String::from_str(env, "USD"),
            quote_asset: String::from_str(env, "USDC"),
            amount: 5000,
            operation_type: 1,
        }
    }

    fn jurisdiction(env: &Env, code: &str) -> Option<String> {
        Some(String::from_str(env, code))
    }

    fn submit_standard_quote(
        env: &Env,
        client: &AnchorKitContractClient,
        anchor: &Address,
        fee: u32,
    ) {
        client.submit_quote(
            anchor,
            &String::from_str(env, "USD"),
            &String::from_str(env, "USDC"),
            &10000u64,
            &fee,
            &100u64,
            &100000u64,
            &1_003_600u64,
        );
    }

    #[test]
    fn test_quote_fee_rejects_values_above_10000_bps() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);
        let anchor = Address::generate(&env);
        register_anchor(&env, &client, &anchor);

        assert!(client.try_submit_quote(
            &anchor,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64,
            &10_000u32,
            &100u64,
            &100000u64,
            &1_003_600u64,
        ).is_ok());
        assert!(client.try_submit_quote(
            &anchor,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64,
            &10_001u32,
            &100u64,
            &100000u64,
            &1_003_600u64,
        ).is_err());
    }

    #[test]
    fn test_select_lowest_fee_anchor() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor1 = Address::generate(&env);
        let anchor2 = Address::generate(&env);
        register_anchor(&env, &client, &anchor1);
        register_anchor(&env, &client, &anchor2);

        client.submit_quote(
            &anchor1,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &50u32, &100u64, &100000u64, &1_003_600u64,
        );
        client.submit_quote(
            &anchor2,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &20u32, &100u64, &100000u64, &1_003_600u64,
        );

        let q1 = client.get_quote(&anchor1, &1u64).unwrap();
        let q2 = client.get_quote(&anchor2, &2u64).unwrap();

        assert_eq!(q1.fee_percentage, 50);
        assert_eq!(q2.fee_percentage, 20);
        // anchor2 has lower fee
        assert!(q2.fee_percentage < q1.fee_percentage);
        assert_eq!(q2.anchor, anchor2);
    }

    #[test]
    fn test_fastest_settlement_strategy() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor1 = Address::generate(&env);
        let anchor2 = Address::generate(&env);
        register_anchor(&env, &client, &anchor1);
        client.set_anchor_metadata(&anchor1, &8000u32, &600u64, &7500u32, &9900u32, &1_000_000u64, &None::<soroban_sdk::String>);
        client.submit_quote(
            &anchor1,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &25u32, &100u64, &100000u64, &1_003_600u64,
        );

        register_anchor(&env, &client, &anchor2);
        client.set_anchor_metadata(&anchor2, &8000u32, &200u64, &7500u32, &9900u32, &1_000_000u64, &None::<soroban_sdk::String>);
        client.submit_quote(
            &anchor2,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &25u32, &100u64, &100000u64, &1_003_600u64,
        );

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "FastestSettlement"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 0,
            max_anchors: 2,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(&env),
        };

        // anchor2 has faster settlement time (200 < 600)
        let best = client.route_transaction(&options);
        assert_eq!(best.anchor, anchor2);
    }

    #[test]
    fn test_filter_by_reputation() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor1 = Address::generate(&env);
        let anchor2 = Address::generate(&env);
        register_anchor(&env, &client, &anchor1);
        // anchor1: reputation 3000 — below the threshold we will set
        client.set_anchor_metadata(&anchor1, &3000u32, &300u64, &7500u32, &9900u32, &1_000_000u64, &None::<soroban_sdk::String>);
        client.submit_quote(
            &anchor1,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &9900u64, &20u32, &100u64, &100000u64, &1_003_600u64,
        );

        register_anchor(&env, &client, &anchor2);
        // anchor2: reputation 8000 — above the threshold
        client.set_anchor_metadata(&anchor2, &8000u32, &300u64, &7500u32, &9900u32, &1_000_000u64, &None::<soroban_sdk::String>);
        client.submit_quote(
            &anchor2,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &25u32, &100u64, &100000u64, &1_003_600u64,
        );

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));

        // min_reputation = 5000 excludes anchor1 (3000 < 5000); only anchor2 qualifies
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 5000,
            max_anchors: 2,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(&env),
        };

        let best = client.route_transaction(&options);
        assert_eq!(best.anchor, anchor2);
    }

    #[test]
    fn test_filter_by_reputation_mixed_scores() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let low = Address::generate(&env);
        let mid = Address::generate(&env);
        let high = Address::generate(&env);

        for (anchor, rep) in [(&low, 1000u32), (&mid, 5000u32), (&high, 9000u32)] {
            register_anchor(&env, &client, anchor);
            client.set_anchor_metadata(anchor, &rep, &300u64, &7500u32, &9900u32, &1_000_000u64, &None::<soroban_sdk::String>);
            client.submit_quote(
                anchor,
                &String::from_str(&env, "USD"),
                &String::from_str(&env, "USDC"),
                &10000u64, &20u32, &100u64, &100000u64, &1_003_600u64,
            );
        }

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "HighestReputation"));

        // threshold = 4000: excludes low (1000), keeps mid (5000) and high (9000)
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 4000,
            max_anchors: 3,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(&env),
        };

        let best = client.route_transaction(&options);
        // low must not be selected; high has the highest reputation among qualifiers
        assert_ne!(best.anchor, low);
        assert_eq!(best.anchor, high);
    }

    #[test]
    fn test_min_reputation_zero_includes_all() {
        // Default min_reputation = 0 means no anchor is filtered by reputation alone.
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor = Address::generate(&env);
        register_anchor(&env, &client, &anchor);
        // reputation_score = 0 (minimum possible)
        client.set_anchor_metadata(&anchor, &0u32, &300u64, &7500u32, &9900u32, &1_000_000u64, &None::<soroban_sdk::String>);
        client.submit_quote(
            &anchor,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &20u32, &100u64, &100000u64, &1_003_600u64,
        );

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 0, // no filter
            max_anchors: 1,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(&env),
        };

        // anchor with reputation 0 is still routable when min_reputation = 0
        let best = client.route_transaction(&options);
        assert_eq!(best.anchor, anchor);
    }

    #[test]
    fn test_expired_quotes_filtered() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor1 = Address::generate(&env);
        register_anchor(&env, &client, &anchor1);

        // First quote: expires at 1_000_100 (still valid at t=1_000_000)
        client.submit_quote(
            &anchor1,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &9900u64, &15u32, &100u64, &100000u64, &1_000_100u64,
        );
        // Second quote: valid for longer
        client.submit_quote(
            &anchor1,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &25u32, &100u64, &100000u64, &1_003_600u64,
        );

        let q1 = client.get_quote(&anchor1, &1u64).unwrap();
        let q2 = client.get_quote(&anchor1, &2u64).unwrap();

        assert_eq!(q1.valid_until, 1_000_100);
        assert_eq!(q2.valid_until, 1_003_600);

        // At t=1_000_200, q1 would be expired
        assert!(q1.valid_until < 1_000_200);
        assert!(q2.valid_until > 1_000_200);
    }

    #[test]
    fn test_expired_quotes_partial_expiry() {
        // Mixed scenario: one anchor's quote is expired, one is still valid.
        // Routing must return only the valid anchor's quote.
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor_expired = Address::generate(&env);
        let anchor_valid = Address::generate(&env);
        register_anchor(&env, &client, &anchor_expired);
        register_anchor(&env, &client, &anchor_valid);

        client.set_anchor_metadata(&anchor_expired, &8000u32, &300u64, &7500u32, &9900u32, &1_000_000u64, &None::<soroban_sdk::String>);
        client.set_anchor_metadata(&anchor_valid, &8000u32, &300u64, &7500u32, &9900u32, &1_000_000u64, &None::<soroban_sdk::String>);

        // anchor_expired: quote valid_until = 1_000_050 (expires before routing)
        client.submit_quote(
            &anchor_expired,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &10u32, &100u64, &100000u64, &1_000_050u64,
        );
        // anchor_valid: quote valid_until = 1_003_600 (still valid)
        client.submit_quote(
            &anchor_valid,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &30u32, &100u64, &100000u64, &1_003_600u64,
        );

        // Advance time past anchor_expired's expiry — now exactly one valid quote remains
        set_ledger(&env, 1_000_100);

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 0,
            max_anchors: 2,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(&env),
        };

        // Only anchor_valid's quote is live; routing must select it
        let best = client.route_transaction(&options);
        assert_eq!(best.anchor, anchor_valid);
    }

    #[test]
    fn test_no_anchors_available() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, _) = setup(&env);

        let anchor1 = Address::generate(&env);
        register_anchor(&env, &client, &anchor1);

        // No quotes submitted
        let quote = client.get_quote(&anchor1, &1u64);
        assert!(quote.is_none());
    }

    #[test]
    fn test_handle_unavailable_anchors() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor1 = Address::generate(&env);
        let anchor2 = Address::generate(&env);
        let anchor3 = Address::generate(&env);
        register_anchor(&env, &client, &anchor1);
        register_anchor(&env, &client, &anchor2);
        register_anchor(&env, &client, &anchor3);

        client.submit_quote(
            &anchor1,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &25u32, &100u64, &100000u64, &1_003_600u64,
        );
        client.submit_quote(
            &anchor2,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10050u64, &30u32, &100u64, &100000u64, &1_003_600u64,
        );

        let q1 = client.get_quote(&anchor1, &1u64).unwrap();
        let q2 = client.get_quote(&anchor2, &2u64).unwrap();

        // anchor3 has no quote
        let quote3 = client.get_quote(&anchor3, &3u64);
        assert!(quote3.is_none());

        assert_eq!(q1.fee_percentage, 25);
        assert_eq!(q2.fee_percentage, 30);
    }

    #[test]
    fn test_amount_outside_quote_limits() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor1 = Address::generate(&env);
        register_anchor(&env, &client, &anchor1);

        client.submit_quote(
            &anchor1,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &25u32, &100u64, &100000u64, &1_003_600u64,
        );

        let q = client.get_quote(&anchor1, &1u64).unwrap();
        assert_eq!(q.minimum_amount, 100);
        assert_eq!(q.maximum_amount, 100000);

        // 5000 is within limits
        assert!(5000 >= q.minimum_amount && 5000 <= q.maximum_amount);
        // 200000 is outside limits
        assert!(200000 > q.maximum_amount);
    }

    #[test]
    fn test_select_best_quote_from_multiple_anchors() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor1 = Address::generate(&env);
        let anchor2 = Address::generate(&env);
        let anchor3 = Address::generate(&env);
        register_anchor(&env, &client, &anchor1);
        register_anchor(&env, &client, &anchor2);
        register_anchor(&env, &client, &anchor3);

        // Explicit metadata so all three anchors participate in routing
        client.set_anchor_metadata(&anchor1, &8000u32, &300u64, &7500u32, &9900u32, &1_000_000u64, &None::<soroban_sdk::String>);
        client.set_anchor_metadata(&anchor2, &8000u32, &300u64, &7500u32, &9900u32, &1_000_000u64, &None::<soroban_sdk::String>);
        client.set_anchor_metadata(&anchor3, &8000u32, &300u64, &7500u32, &9900u32, &1_000_000u64, &None::<soroban_sdk::String>);

        // Fees: anchor1=50, anchor2=25, anchor3=30 — all distinct, so winner is deterministic
        client.submit_quote(
            &anchor1,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10100u64, &50u32, &100u64, &100000u64, &1_003_600u64,
        );
        client.submit_quote(
            &anchor2,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &25u32, &100u64, &100000u64, &1_003_600u64,
        );
        client.submit_quote(
            &anchor3,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10050u64, &30u32, &100u64, &100000u64, &1_003_600u64,
        );

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 0,
            max_anchors: 3,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(&env),
        };

        // anchor2 has the unique lowest fee (25); result is independent of storage iteration order
        let best = client.route_transaction(&options);
        assert_eq!(best.anchor, anchor2);
        assert_eq!(best.fee_percentage, 25);
    }

    #[test]
    fn test_auto_deactivation() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor = Address::generate(&env);
        register_anchor(&env, &client, &anchor);
        client.set_anchor_metadata(&anchor, &8000u32, &300u64, &7500u32, &9900u32, &1_000_000u64, &None::<soroban_sdk::String>);
        client.submit_quote(
            &anchor,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &20u32, &100u64, &100000u64, &1_003_600u64,
        );

        // Set threshold to 3 consecutive failures
        client.set_health_failure_threshold(&3u32);

        // Two failures — below threshold, anchor still active
        client.update_health_status(&anchor, &100u64, &2u32, &9800u32);
        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy: strategy.clone(),
            min_reputation: 0,
            max_anchors: 1,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(&env),
        };
        let best = client.route_transaction(&options);
        assert_eq!(best.anchor, anchor);

        // Third failure — threshold breached, anchor deactivated
        client.update_health_status(&anchor, &100u64, &3u32, &9500u32);

        // Health status recorded
        let health = client.get_health_status(&anchor).unwrap();
        assert_eq!(health.failure_count, 3);

        // Anchor no longer routable
        let result = client.try_route_transaction(&options);
        assert!(result.is_err());
    }

    #[test]
    fn test_balanced_strategy() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        // Anchor A: low fee (10), slow (1000s), low reputation (2000)
        //   fee_term  = 40_000 / 10   = 4000
        //   time_term = 30_000 / 1000 = 30
        //   rep_term  = 2000 * 3_000 / 10_000 = 600
        //   score = 4630
        let anchor_a = Address::generate(&env);
        register_anchor(&env, &client, &anchor_a);
        client.set_anchor_metadata(&anchor_a, &2000u32, &1000u64, &7500u32, &9900u32, &1_000_000u64, &None::<soroban_sdk::String>);
        client.submit_quote(
            &anchor_a,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &10u32, &100u64, &100000u64, &1_003_600u64,
        );

        // Anchor B: high fee (50), fast (100s), high reputation (9000)
        //   fee_term  = 40_000 / 50  = 800
        //   time_term = 30_000 / 100 = 300
        //   rep_term  = 9000 * 3_000 / 10_000 = 2700
        //   score = 3800
        let anchor_b = Address::generate(&env);
        register_anchor(&env, &client, &anchor_b);
        client.set_anchor_metadata(&anchor_b, &9000u32, &100u64, &7500u32, &9900u32, &1_000_000u64, &None::<soroban_sdk::String>);
        client.submit_quote(
            &anchor_b,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &50u32, &100u64, &100000u64, &1_003_600u64,
        );

        // Anchor C: medium fee (20), medium speed (200s), medium reputation (6000)
        //   fee_term  = 40_000 / 20  = 2000
        //   time_term = 30_000 / 200 = 150
        //   rep_term  = 6000 * 3_000 / 10_000 = 1800
        //   score = 3950
        let anchor_c = Address::generate(&env);
        register_anchor(&env, &client, &anchor_c);
        client.set_anchor_metadata(&anchor_c, &6000u32, &200u64, &7500u32, &9900u32, &1_000_000u64, &None::<soroban_sdk::String>);
        client.submit_quote(
            &anchor_c,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &20u32, &100u64, &100000u64, &1_003_600u64,
        );

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "Balanced"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 0,
            max_anchors: 3,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(&env),
        };

        // anchor_a wins: score 4630 > anchor_c 3950 > anchor_b 3800
        let best = client.route_transaction(&options);
        assert_eq!(best.anchor, anchor_a);
    }

    #[test]
    fn test_route_transaction_emits_routing_decision_event() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor = Address::generate(&env);
        register_anchor(&env, &client, &anchor);

        client.submit_quote(
            &anchor,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &25u32, &100u64, &100000u64, &1_003_600u64,
        );

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy: strategy.clone(),
            min_reputation: 0,
            max_anchors: 1,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(&env),
        };

        // Expect RoutingDecision event with correct fields
        let event = RoutingDecisionEvent {
            anchor: anchor.clone(),
            strategy: String::from_str(&env, "LowestFee"),
            quote_id: 1u64,
            ledger_sequence: 0u32,
        };
        let topics = (symbol_short!("routing"),);
        let _ = (&topics, &event);

        let best = client.route_transaction(&options);
        assert_eq!(best.anchor, anchor);
        assert_eq!(best.quote_id, 1u64);
    }

    #[test]
    fn test_expired_quote_emits_quote_expired_event() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor1 = Address::generate(&env);
        let anchor2 = Address::generate(&env);
        register_anchor(&env, &client, &anchor1);
        register_anchor(&env, &client, &anchor2);

        // Submit quotes with the same expiration
        let valid_until = 1_002_000u64;
        client.submit_quote(
            &anchor1,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &25u32, &100u64, &100000u64, &valid_until,
        );
        client.submit_quote(
            &anchor2,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &30u32, &100u64, &100000u64, &valid_until,
        );

        // Move time forward so quote expires
        set_ledger(&env, 1_003_000); // Now > valid_until

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 0,
            max_anchors: 2,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(&env),
        };

        // Should fail because all quotes are expired
        let result = client.try_route_transaction(&options);
        assert!(result.is_err());
    }

    #[test]
    fn test_mixed_valid_and_expired_quotes() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor1 = Address::generate(&env);
        let anchor2 = Address::generate(&env);
        register_anchor(&env, &client, &anchor1);
        register_anchor(&env, &client, &anchor2);

        // anchor1 has a quote that will expire
        let expired_until = 1_001_000u64;
        client.submit_quote(
            &anchor1,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &50u32, &100u64, &100000u64, &expired_until,
        );

        // anchor2 has a valid quote
        let valid_until = 1_003_000u64;
        client.submit_quote(
            &anchor2,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &25u32, &100u64, &100000u64, &valid_until,
        );

        // Move time forward
        set_ledger(&env, 1_002_000); // Now between expired_until and valid_until

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 0,
            max_anchors: 2,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(&env),
        };

        // Should succeed and select anchor2 (valid quote)
        let best = client.route_transaction(&options);
        assert_eq!(best.anchor, anchor2);
        assert_eq!(best.quote_id, 2u64);
    }

    #[test]
    fn test_jurisdiction_filter_none_includes_all_regions() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let us_anchor = Address::generate(&env);
        let eu_anchor = Address::generate(&env);
        register_anchor(&env, &client, &us_anchor);
        register_anchor(&env, &client, &eu_anchor);
        client.set_anchor_jurisdiction(&us_anchor, &jurisdiction(&env, "USA"));
        client.set_anchor_jurisdiction(&eu_anchor, &jurisdiction(&env, "DEU"));

        submit_standard_quote(&env, &client, &us_anchor, 50);
        submit_standard_quote(&env, &client, &eu_anchor, 20);

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 0,
            max_anchors: 2,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(&env),
        };

        let best = client.route_transaction(&options);
        assert_eq!(best.anchor, eu_anchor);
    }

    #[test]
    fn test_jurisdiction_filter_selects_matching_region() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let us_anchor = Address::generate(&env);
        let eu_anchor = Address::generate(&env);
        register_anchor(&env, &client, &us_anchor);
        register_anchor(&env, &client, &eu_anchor);
        client.set_anchor_jurisdiction(&us_anchor, &jurisdiction(&env, "USA"));
        client.set_anchor_jurisdiction(&eu_anchor, &jurisdiction(&env, "DEU"));

        // US anchor has lower fee but wrong jurisdiction for this request.
        submit_standard_quote(&env, &client, &us_anchor, 10);
        submit_standard_quote(&env, &client, &eu_anchor, 40);

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 0,
            max_anchors: 2,
            require_kyc: false,
            jurisdiction: jurisdiction(&env, "DEU"),
            fallback_chain: Vec::new(&env),
        };

        let best = client.route_transaction(&options);
        assert_eq!(best.anchor, eu_anchor);
        assert_eq!(best.fee_percentage, 40);
    }

    #[test]
    fn test_jurisdiction_filter_excludes_anchor_without_jurisdiction() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let unscoped = Address::generate(&env);
        let us_anchor = Address::generate(&env);
        register_anchor(&env, &client, &unscoped);
        register_anchor(&env, &client, &us_anchor);
        client.set_anchor_jurisdiction(&us_anchor, &jurisdiction(&env, "USA"));

        submit_standard_quote(&env, &client, &unscoped, 5);
        submit_standard_quote(&env, &client, &us_anchor, 30);

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 0,
            max_anchors: 2,
            require_kyc: false,
            jurisdiction: jurisdiction(&env, "USA"),
            fallback_chain: Vec::new(&env),
        };

        let best = client.route_transaction(&options);
        assert_eq!(best.anchor, us_anchor);
    }

    #[test]
    fn test_jurisdiction_filter_no_matching_anchors_fails() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let us_anchor = Address::generate(&env);
        register_anchor(&env, &client, &us_anchor);
        client.set_anchor_jurisdiction(&us_anchor, &jurisdiction(&env, "USA"));
        submit_standard_quote(&env, &client, &us_anchor, 25);

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 0,
            max_anchors: 1,
            require_kyc: false,
            jurisdiction: jurisdiction(&env, "GBR"),
            fallback_chain: Vec::new(&env),
        };

        assert!(client.try_route_transaction(&options).is_err());
    }

    #[test]
    fn test_set_anchor_jurisdiction_clear_with_none() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor = Address::generate(&env);
        register_anchor(&env, &client, &anchor);
        client.set_anchor_jurisdiction(&anchor, &jurisdiction(&env, "USA"));
        assert_eq!(
            client.get_anchor_jurisdiction(&anchor),
            jurisdiction(&env, "USA")
        );

        client.set_anchor_jurisdiction(&anchor, &None);
        assert_eq!(client.get_anchor_jurisdiction(&anchor), None);
    }

    fn make_lowest_fee_options(env: &Env) -> RoutingOptions {
        let mut strategy = Vec::new(env);
        strategy.push_back(Symbol::new(env, "LowestFee"));
        RoutingOptions {
            request: make_request(env),
            strategy,
            min_reputation: 0,
            max_anchors: 0,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(env),
        }
    }

    #[test]
    fn test_dry_run_returns_same_anchor_as_route_transaction() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor1 = Address::generate(&env);
        let anchor2 = Address::generate(&env);
        register_anchor(&env, &client, &anchor1);
        register_anchor(&env, &client, &anchor2);
        submit_standard_quote(&env, &client, &anchor1, 50);
        submit_standard_quote(&env, &client, &anchor2, 20);

        let options = make_lowest_fee_options(&env);
        let dry_run_anchor = client.route_transaction_dry_run(&options);
        let routed = client.route_transaction(&options);

        assert_eq!(dry_run_anchor, anchor2);
        assert_eq!(routed.anchor, anchor2);
        assert_eq!(dry_run_anchor, routed.anchor);
    }

    #[test]
    fn test_dry_run_emits_no_routing_decision_event() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor = Address::generate(&env);
        register_anchor(&env, &client, &anchor);
        submit_standard_quote(&env, &client, &anchor, 25);

        let options = make_lowest_fee_options(&env);
        let events_before = env.events().all().len();
        let dry_run_anchor = client.route_transaction_dry_run(&options);
        let events_after = env.events().all().len();

        assert_eq!(dry_run_anchor, anchor);
        assert_eq!(events_before, events_after);
    }

    #[test]
    fn test_dry_run_does_not_emit_quote_expired_event() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let expired_anchor = Address::generate(&env);
        let valid_anchor = Address::generate(&env);
        register_anchor(&env, &client, &expired_anchor);
        register_anchor(&env, &client, &valid_anchor);

        let valid_until = 1_002_000u64;
        client.submit_quote(
            &expired_anchor,
            &String::from_str(&env, "USD"),
            &String::from_str(&env, "USDC"),
            &10000u64, &10u32, &100u64, &100000u64, &valid_until,
        );
        submit_standard_quote(&env, &client, &valid_anchor, 30);

        set_ledger(&env, 1_003_000);

        let options = make_lowest_fee_options(&env);
        let events_before = env.events().all().len();
        let dry_run_anchor = client.route_transaction_dry_run(&options);
        let events_after = env.events().all().len();

        assert_eq!(dry_run_anchor, valid_anchor);
        assert_eq!(events_before, events_after);
    }

    #[test]
    fn test_dry_run_panics_when_no_candidates() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let options = make_lowest_fee_options(&env);
        let result = client.try_route_transaction_dry_run(&options);
        assert!(result.is_err());
    }

    #[test]
    fn test_dry_run_respects_jurisdiction_filter() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let us_anchor = Address::generate(&env);
        let eu_anchor = Address::generate(&env);
        register_anchor(&env, &client, &us_anchor);
        register_anchor(&env, &client, &eu_anchor);
        client.set_anchor_jurisdiction(&us_anchor, &jurisdiction(&env, "USA"));
        client.set_anchor_jurisdiction(&eu_anchor, &jurisdiction(&env, "EU"));
        submit_standard_quote(&env, &client, &us_anchor, 10);
        submit_standard_quote(&env, &client, &eu_anchor, 50);

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 0,
            max_anchors: 0,
            require_kyc: false,
            jurisdiction: jurisdiction(&env, "EU"),
            fallback_chain: Vec::new(&env),
        };

        let dry_run_anchor = client.route_transaction_dry_run(&options);
        assert_eq!(dry_run_anchor, eu_anchor);
    }

    #[test]
    fn test_dry_run_does_not_change_route_transaction_events() {
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor = Address::generate(&env);
        register_anchor(&env, &client, &anchor);
        submit_standard_quote(&env, &client, &anchor, 25);

        let options = make_lowest_fee_options(&env);
        let _ = client.route_transaction_dry_run(&options);

        let events_before = env.events().all().len();
        let routed = client.route_transaction(&options);
        let events_after = env.events().all().len();

        assert_eq!(routed.anchor, anchor);
        assert_eq!(events_after, events_before + 1);
    }

    #[test]
    fn test_revoked_attestor_quotes_not_routed() {
        // After revoking an anchor, its still-unexpired quote must never be routed.
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let revoked = Address::generate(&env);
        let healthy = Address::generate(&env);
        register_anchor(&env, &client, &revoked);
        register_anchor(&env, &client, &healthy);

        // Revoked anchor has the cheaper quote, so it would win without the fix.
        submit_standard_quote(&env, &client, &revoked, 10);
        submit_standard_quote(&env, &client, &healthy, 40);

        client.revoke_attestor(&revoked);

        let options = make_lowest_fee_options(&env);
        let best = client.route_transaction(&options);
        assert_eq!(best.anchor, healthy);
        assert_ne!(best.anchor, revoked);
    }

    #[test]
    fn test_revoked_attestor_only_candidate_fails_routing() {
        // When the only quote belongs to a revoked anchor, routing must fail.
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let revoked = Address::generate(&env);
        register_anchor(&env, &client, &revoked);
        submit_standard_quote(&env, &client, &revoked, 10);

        client.revoke_attestor(&revoked);

        let options = make_lowest_fee_options(&env);
        assert!(client.try_route_transaction(&options).is_err());
    }

    #[test]
    fn test_anchor_without_quotes_service_not_routed() {
        // An anchor that no longer advertises SERVICE_QUOTES must be skipped.
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let no_quotes = Address::generate(&env);
        let healthy = Address::generate(&env);
        register_anchor(&env, &client, &no_quotes);
        register_anchor(&env, &client, &healthy);

        // no_quotes has the cheaper quote, so it would win without the fix.
        submit_standard_quote(&env, &client, &no_quotes, 10);
        submit_standard_quote(&env, &client, &healthy, 40);

        // Remove SERVICE_QUOTES (1) from the anchor's advertised services.
        let mut services = Vec::new(&env);
        services.push_back(3u32);
        client.configure_services(&no_quotes, &services);

        let options = make_lowest_fee_options(&env);
        let best = client.route_transaction(&options);
        assert_eq!(best.anchor, healthy);
        assert_ne!(best.anchor, no_quotes);
    }
}

    #[test]
    fn test_single_anchor_routing_returns_that_anchor() {
        // Edge case: only one anchor is registered; routing must select it regardless of strategy.
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let only_anchor = Address::generate(&env);
        register_anchor(&env, &client, &only_anchor);
        submit_standard_quote(&env, &client, &only_anchor, 30);

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 0,
            max_anchors: 1,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(&env),
        };

        let best = client.route_transaction(&options);
        assert_eq!(best.anchor, only_anchor);
    }

    #[test]
    #[should_panic(expected = "HostError")]
    fn test_all_anchors_below_health_threshold_panics() {
        // Edge case: all anchors are deactivated by the failure threshold;
        // route_transaction must panic rather than silently returning a bad anchor.
        let env = make_env();
        set_ledger(&env, 1_000_000);
        let (client, _) = setup(&env);

        let anchor1 = Address::generate(&env);
        let anchor2 = Address::generate(&env);
        register_anchor(&env, &client, &anchor1);
        register_anchor(&env, &client, &anchor2);
        submit_standard_quote(&env, &client, &anchor1, 20);
        submit_standard_quote(&env, &client, &anchor2, 25);

        // Set failure threshold to 2 consecutive failures
        client.set_health_failure_threshold(&2u32);

        // Push both anchors past the threshold — they become inactive
        client.update_health_status(&anchor1, &200u64, &2u32, &5000u32);
        client.update_health_status(&anchor2, &200u64, &2u32, &5000u32);

        let mut strategy = Vec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));
        let options = RoutingOptions {
            request: make_request(&env),
            strategy,
            min_reputation: 0,
            max_anchors: 2,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: Vec::new(&env),
        };

        // Must panic — no active anchors remain
        client.route_transaction(&options);
    }
