#![cfg(test)]

use crate::contract::{AnchorKitContract, AnchorKitContractClient};
use crate::types::AnchorMetadata;
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env};

/// Helper to create a test environment with initialized contract
/// Helper to create a test environment with initialized contract
fn setup_test_env() -> (Env, AnchorKitContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let anchor = Address::generate(&env);
    client.initialize(&admin, &100_u64, &None, &None);
    (env, client, admin, anchor)
}

/// Helper to create metadata with specific values
fn create_metadata(
    env: &Env,
    anchor: &Address,
    uptime: u32,
    reputation: u32,
    settlement_time: u64,
) -> AnchorMetadata {
    AnchorMetadata {
        anchor: anchor.clone(),
        reputation_score: reputation,
        liquidity_score: 5000,
        uptime_percentage: uptime,
        total_volume: 1_000_000,
        average_settlement_time: settlement_time,
        is_active: true,
        homepage_url: None,
    }
}

#[test]
fn test_perfect_health_score() {
    let (env, client, _admin, anchor) = setup_test_env();

    // Perfect metrics: 100% uptime, 100% reputation, 5min settlement
    let metadata = create_metadata(&env, &anchor, 10000, 10000, 300);
    client.cache_metadata(&anchor, &metadata, &3600);

    let score = client.get_anchor_health_score(&anchor);
    
    // Expected: (40 * 100 + 35 * 100 + 25 * 100) / 100 = 100
    assert_eq!(score, 100);
}

#[test]
fn test_good_health_score() {
    let (env, client, _admin, anchor) = setup_test_env();

    // Good metrics: 95% uptime, 85% reputation, 8min settlement
    let metadata = create_metadata(&env, &anchor, 9500, 8500, 480);
    client.cache_metadata(&anchor, &metadata, &3600);

    let score = client.get_anchor_health_score(&anchor);
    
    // Expected: (40 * 95 + 35 * 85 + 25 * 80) / 100 = 87.75 → 87
    assert_eq!(score, 87);
}

#[test]
fn test_acceptable_health_score() {
    let (env, client, _admin, anchor) = setup_test_env();

    // Acceptable metrics: 80% uptime, 70% reputation, 20min settlement
    let metadata = create_metadata(&env, &anchor, 8000, 7000, 1200);
    client.cache_metadata(&anchor, &metadata, &3600);

    let score = client.get_anchor_health_score(&anchor);
    
    // Expected: (40 * 80 + 35 * 70 + 25 * 60) / 100 = 71.5 → 71
    assert_eq!(score, 71);
}

#[test]
fn test_poor_health_score() {
    let (env, client, _admin, anchor) = setup_test_env();

    // Poor metrics: 50% uptime, 40% reputation, 45min settlement
    let metadata = create_metadata(&env, &anchor, 5000, 4000, 2700);
    client.cache_metadata(&anchor, &metadata, &3600);

    let score = client.get_anchor_health_score(&anchor);
    
    // Expected: (40 * 50 + 35 * 40 + 25 * 40) / 100 = 44
    assert_eq!(score, 44);
}

#[test]
fn test_very_poor_health_score() {
    let (env, client, _admin, anchor) = setup_test_env();

    // Very poor metrics: 20% uptime, 10% reputation, 2hr settlement
    let metadata = create_metadata(&env, &anchor, 2000, 1000, 7200);
    client.cache_metadata(&anchor, &metadata, &3600);

    let score = client.get_anchor_health_score(&anchor);
    
    // Expected: (40 * 20 + 35 * 10 + 25 * 20) / 100 = 16.5 → 16
    assert_eq!(score, 16);
}

#[test]
fn test_settlement_time_boundaries() {
    let (env, client, _admin, anchor) = setup_test_env();

    // Test exact boundary at 300s (should get 100 points)
    let metadata = create_metadata(&env, &anchor, 10000, 10000, 300);
    client.cache_metadata(&anchor, &metadata, &3600);
    let score = client.get_anchor_health_score(&anchor);
    assert_eq!(score, 100);

    // Test just over 300s (should get 80 points for speed)
    let metadata = create_metadata(&env, &anchor, 10000, 10000, 301);
    client.cache_metadata(&anchor, &metadata, &3600);
    let score = client.get_anchor_health_score(&anchor);
    // (40 * 100 + 35 * 100 + 25 * 80) / 100 = 95
    assert_eq!(score, 95);

    // Test exact boundary at 600s (should get 80 points)
    let metadata = create_metadata(&env, &anchor, 10000, 10000, 600);
    client.cache_metadata(&anchor, &metadata, &3600);
    let score = client.get_anchor_health_score(&anchor);
    assert_eq!(score, 95);

    // Test just over 600s (should get 60 points for speed)
    let metadata = create_metadata(&env, &anchor, 10000, 10000, 601);
    client.cache_metadata(&anchor, &metadata, &3600);
    let score = client.get_anchor_health_score(&anchor);
    // (40 * 100 + 35 * 100 + 25 * 60) / 100 = 90
    assert_eq!(score, 90);
}

#[test]
#[should_panic(expected = "Error(Contract, #49)")]
fn test_cache_not_found_error() {
    let (_env, client, _admin, anchor) = setup_test_env();

    // Try to get health score without caching metadata first
    client.get_anchor_health_score(&anchor);
}

#[test]
#[should_panic(expected = "Error(Contract, #48)")]
fn test_cache_expired_error() {
    let (env, client, _admin, anchor) = setup_test_env();

    // Cache metadata with 1 second TTL
    let metadata = create_metadata(&env, &anchor, 9000, 8000, 500);
    client.cache_metadata(&anchor, &metadata, &1);

    // Advance ledger time by 2 seconds
    env.ledger().with_mut(|li| {
        li.timestamp += 2;
    });

    // Should panic with CacheExpired
    client.get_anchor_health_score(&anchor);
}

#[test]
fn test_zero_values() {
    let (env, client, _admin, anchor) = setup_test_env();

    // All zero metrics
    let metadata = create_metadata(&env, &anchor, 0, 0, 10000);
    client.cache_metadata(&anchor, &metadata, &3600);

    let score = client.get_anchor_health_score(&anchor);
    
    // Expected: (40 * 0 + 35 * 0 + 25 * 20) / 100 = 5
    assert_eq!(score, 5);
}

#[test]
fn test_multiple_anchors() {
    let (env, client, _admin, _) = setup_test_env();

    let anchor1 = Address::generate(&env);
    let anchor2 = Address::generate(&env);
    let anchor3 = Address::generate(&env);

    // Cache different metadata for each anchor
    let metadata1 = create_metadata(&env, &anchor1, 9500, 9000, 250);
    let metadata2 = create_metadata(&env, &anchor2, 7000, 6000, 1500);
    let metadata3 = create_metadata(&env, &anchor3, 5000, 5000, 4000);

    client.cache_metadata(&anchor1, &metadata1, &3600);
    client.cache_metadata(&anchor2, &metadata2, &3600);
    client.cache_metadata(&anchor3, &metadata3, &3600);

    let score1 = client.get_anchor_health_score(&anchor1);
    let score2 = client.get_anchor_health_score(&anchor2);
    let score3 = client.get_anchor_health_score(&anchor3);

    // Verify scores are different and in expected order
    assert!(score1 > score2);
    assert!(score2 > score3);
    
    // Verify approximate values
    assert!(score1 >= 90); // Excellent anchor
    assert!(score2 >= 60 && score2 <= 75); // Acceptable anchor
    assert!(score3 >= 30 && score3 <= 45); // Poor anchor
}

#[test]
fn test_edge_case_max_values() {
    let (env, client, _admin, anchor) = setup_test_env();

    // Maximum possible values (should not exceed 100)
    let metadata = create_metadata(&env, &anchor, 10000, 10000, 0);
    client.cache_metadata(&anchor, &metadata, &3600);

    let score = client.get_anchor_health_score(&anchor);
    
    // Should be capped at 100
    assert_eq!(score, 100);
}

#[test]
fn test_realistic_scenarios() {
    let (env, client, _admin, _) = setup_test_env();

    // Scenario 1: High-quality established anchor
    let anchor_premium = Address::generate(&env);
    let metadata = create_metadata(&env, &anchor_premium, 9800, 9500, 180);
    client.cache_metadata(&anchor_premium, &metadata, &3600);
    let score = client.get_anchor_health_score(&anchor_premium);
    assert!(score >= 95, "Premium anchor should score 95+");

    // Scenario 2: New anchor with good tech but low reputation
    let anchor_new = Address::generate(&env);
    let metadata = create_metadata(&env, &anchor_new, 9500, 3000, 200);
    client.cache_metadata(&anchor_new, &metadata, &3600);
    let score = client.get_anchor_health_score(&anchor_new);
    assert!(score >= 65 && score <= 75, "New anchor should score 65-75");

    // Scenario 3: Struggling anchor with issues
    let anchor_struggling = Address::generate(&env);
    let metadata = create_metadata(&env, &anchor_struggling, 6000, 5000, 3000);
    client.cache_metadata(&anchor_struggling, &metadata, &3600);
    let score = client.get_anchor_health_score(&anchor_struggling);
    assert!(score >= 40 && score <= 55, "Struggling anchor should score 40-55");
}
