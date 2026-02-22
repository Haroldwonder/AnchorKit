#![cfg(test)]

use crate::{
    types::{AnchorMetadata, AnchorServices, ServiceType},
    AnchorKitContract, AnchorKitContractClient, Error,
};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

/// Test Goal 1: Detect deposit-only anchors
#[test]
fn test_detect_deposit_only_anchor() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let deposit_anchor = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin);

    // Register anchor as attestor
    client.register_attestor(&deposit_anchor);

    // Configure anchor with ONLY deposit service
    let mut services = Vec::new(&env);
    services.push_back(ServiceType::Deposits);

    client.configure_services(&deposit_anchor, &services);

    // Verify capability detection
    let supported_services = client.get_supported_services(&deposit_anchor);
    assert_eq!(supported_services.len(), 1);
    assert_eq!(supported_services.get(0).unwrap(), ServiceType::Deposits);

    // Verify deposit service is supported
    assert!(client.supports_service(&deposit_anchor, &ServiceType::Deposits));

    // Verify withdrawal service is NOT supported
    assert!(!client.supports_service(&deposit_anchor, &ServiceType::Withdrawals));

    // Verify quotes service is NOT supported
    assert!(!client.supports_service(&deposit_anchor, &ServiceType::Quotes));

    // Verify KYC service is NOT supported
    assert!(!client.supports_service(&deposit_anchor, &ServiceType::KYC));
}

/// Test Goal 2: Detect full-service anchors
#[test]
fn test_detect_full_service_anchor() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let full_service_anchor = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin);

    // Register anchor as attestor
    client.register_attestor(&full_service_anchor);

    // Configure anchor with ALL services
    let mut services = Vec::new(&env);
    services.push_back(ServiceType::Deposits);
    services.push_back(ServiceType::Withdrawals);
    services.push_back(ServiceType::Quotes);
    services.push_back(ServiceType::KYC);

    client.configure_services(&full_service_anchor, &services);

    // Verify capability detection
    let supported_services = client.get_supported_services(&full_service_anchor);
    assert_eq!(supported_services.len(), 4);

    // Verify all services are supported
    assert!(client.supports_service(&full_service_anchor, &ServiceType::Deposits));
    assert!(client.supports_service(&full_service_anchor, &ServiceType::Withdrawals));
    assert!(client.supports_service(&full_service_anchor, &ServiceType::Quotes));
    assert!(client.supports_service(&full_service_anchor, &ServiceType::KYC));

    // Verify services are in the list
    assert!(supported_services.contains(&ServiceType::Deposits));
    assert!(supported_services.contains(&ServiceType::Withdrawals));
    assert!(supported_services.contains(&ServiceType::Quotes));
    assert!(supported_services.contains(&ServiceType::KYC));
}

/// Test Goal 3: Reject malformed capability payloads - Empty services
#[test]
fn test_reject_empty_services() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let anchor = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin);

    // Register anchor as attestor
    client.register_attestor(&anchor);

    // Try to configure with empty services - should fail
    let empty_services = Vec::new(&env);

    let result = client.try_configure_services(&anchor, &empty_services);
    assert_eq!(result, Err(Ok(Error::InvalidServiceType)));
}

/// Test Goal 3: Reject malformed capability payloads - Duplicate services
#[test]
fn test_reject_duplicate_services() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let anchor = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin);

    // Register anchor as attestor
    client.register_attestor(&anchor);

    // Try to configure with duplicate services - should fail
    let mut duplicate_services = Vec::new(&env);
    duplicate_services.push_back(ServiceType::Deposits);
    duplicate_services.push_back(ServiceType::Deposits); // Duplicate

    let result = client.try_configure_services(&anchor, &duplicate_services);
    assert_eq!(result, Err(Ok(Error::InvalidServiceType)));
}

/// Test Goal 3: Reject malformed capability payloads - Unregistered anchor
#[test]
fn test_reject_unregistered_anchor_services() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let unregistered_anchor = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin);

    // Try to configure services for unregistered anchor - should fail
    let mut services = Vec::new(&env);
    services.push_back(ServiceType::Deposits);

    let result = client.try_configure_services(&unregistered_anchor, &services);
    assert_eq!(result, Err(Ok(Error::AttestorNotRegistered)));
}

/// Test: Detect withdrawal-only anchor
#[test]
fn test_detect_withdrawal_only_anchor() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let withdrawal_anchor = Address::generate(&env);

    client.initialize(&admin);
    client.register_attestor(&withdrawal_anchor);

    // Configure with only withdrawals
    let mut services = Vec::new(&env);
    services.push_back(ServiceType::Withdrawals);

    client.configure_services(&withdrawal_anchor, &services);

    // Verify
    assert!(!client.supports_service(&withdrawal_anchor, &ServiceType::Deposits));
    assert!(client.supports_service(&withdrawal_anchor, &ServiceType::Withdrawals));
    assert!(!client.supports_service(&withdrawal_anchor, &ServiceType::Quotes));
    assert!(!client.supports_service(&withdrawal_anchor, &ServiceType::KYC));
}

/// Test: Detect quote-provider anchor
#[test]
fn test_detect_quote_provider_anchor() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let quote_anchor = Address::generate(&env);

    client.initialize(&admin);
    client.register_attestor(&quote_anchor);

    // Configure with quotes and KYC
    let mut services = Vec::new(&env);
    services.push_back(ServiceType::Quotes);
    services.push_back(ServiceType::KYC);

    client.configure_services(&quote_anchor, &services);

    // Verify
    assert!(!client.supports_service(&quote_anchor, &ServiceType::Deposits));
    assert!(!client.supports_service(&quote_anchor, &ServiceType::Withdrawals));
    assert!(client.supports_service(&quote_anchor, &ServiceType::Quotes));
    assert!(client.supports_service(&quote_anchor, &ServiceType::KYC));
}

/// Test: Capability detection with metadata
#[test]
fn test_capability_detection_with_metadata() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let anchor = Address::generate(&env);

    client.initialize(&admin);
    client.register_attestor(&anchor);

    // Configure services
    let mut services = Vec::new(&env);
    services.push_back(ServiceType::Deposits);
    services.push_back(ServiceType::Withdrawals);

    client.configure_services(&anchor, &services);

    // Set metadata
    client.set_anchor_metadata(
        &anchor,
        &8500u32,  // reputation_score (85%)
        &3600u64,  // average_settlement_time (1 hour)
        &9000u32,  // liquidity_score (90%)
        &9950u32,  // uptime_percentage (99.5%)
        &1_000_000u64, // total_volume
    );

    // Verify metadata
    let metadata = client.get_anchor_metadata(&anchor);
    assert_eq!(metadata.anchor, anchor);
    assert_eq!(metadata.reputation_score, 8500);
    assert_eq!(metadata.average_settlement_time, 3600);
    assert_eq!(metadata.liquidity_score, 9000);
    assert_eq!(metadata.uptime_percentage, 9950);
    assert_eq!(metadata.total_volume, 1_000_000);
    assert!(metadata.is_active);

    // Verify services still work
    assert!(client.supports_service(&anchor, &ServiceType::Deposits));
    assert!(client.supports_service(&anchor, &ServiceType::Withdrawals));
}

/// Test: Get services for non-configured anchor
#[test]
fn test_get_services_for_non_configured_anchor() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let anchor = Address::generate(&env);

    client.initialize(&admin);
    client.register_attestor(&anchor);

    // Try to get services without configuring - should fail
    let result = client.try_get_supported_services(&anchor);
    assert_eq!(result, Err(Ok(Error::ServicesNotConfigured)));

    // supports_service should return false
    assert!(!client.supports_service(&anchor, &ServiceType::Deposits));
}

/// Test: Update anchor capabilities
#[test]
fn test_update_anchor_capabilities() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let anchor = Address::generate(&env);

    client.initialize(&admin);
    client.register_attestor(&anchor);

    // Initially configure with only deposits
    let mut initial_services = Vec::new(&env);
    initial_services.push_back(ServiceType::Deposits);

    client.configure_services(&anchor, &initial_services);

    // Verify initial state
    assert!(client.supports_service(&anchor, &ServiceType::Deposits));
    assert!(!client.supports_service(&anchor, &ServiceType::Withdrawals));

    // Update to add withdrawals
    let mut updated_services = Vec::new(&env);
    updated_services.push_back(ServiceType::Deposits);
    updated_services.push_back(ServiceType::Withdrawals);

    client.configure_services(&anchor, &updated_services);

    // Verify updated state
    assert!(client.supports_service(&anchor, &ServiceType::Deposits));
    assert!(client.supports_service(&anchor, &ServiceType::Withdrawals));
}

/// Test: Malformed metadata - Invalid scores
#[test]
fn test_reject_invalid_metadata_scores() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let anchor = Address::generate(&env);

    client.initialize(&admin);
    client.register_attestor(&anchor);

    // Try to set metadata with invalid reputation score (> 10000)
    let result = client.try_set_anchor_metadata(
        &anchor,
        &10001u32, // Invalid: > 10000
        &3600u64,
        &9000u32,
        &9950u32,
        &1_000_000u64,
    );
    assert_eq!(result, Err(Ok(Error::InvalidAnchorMetadata)));

    // Try to set metadata with invalid liquidity score
    let result = client.try_set_anchor_metadata(
        &anchor,
        &8500u32,
        &3600u64,
        &10001u32, // Invalid: > 10000
        &9950u32,
        &1_000_000u64,
    );
    assert_eq!(result, Err(Ok(Error::InvalidAnchorMetadata)));

    // Try to set metadata with invalid uptime percentage
    let result = client.try_set_anchor_metadata(
        &anchor,
        &8500u32,
        &3600u64,
        &9000u32,
        &10001u32, // Invalid: > 10000
        &1_000_000u64,
    );
    assert_eq!(result, Err(Ok(Error::InvalidAnchorMetadata)));
}

/// Test: Capability detection for inactive anchor
#[test]
fn test_capability_detection_inactive_anchor() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let anchor = Address::generate(&env);

    client.initialize(&admin);
    client.register_attestor(&anchor);

    // Configure services
    let mut services = Vec::new(&env);
    services.push_back(ServiceType::Deposits);

    client.configure_services(&anchor, &services);

    // Set metadata
    client.set_anchor_metadata(&anchor, &8500u32, &3600u64, &9000u32, &9950u32, &1_000_000u64);

    // Deactivate anchor
    client.deactivate_anchor(&anchor);

    // Verify metadata shows inactive
    let metadata = client.get_anchor_metadata(&anchor);
    assert!(!metadata.is_active);

    // Services should still be queryable
    assert!(client.supports_service(&anchor, &ServiceType::Deposits));

    // Reactivate
    client.reactivate_anchor(&anchor);

    // Verify metadata shows active again
    let metadata = client.get_anchor_metadata(&anchor);
    assert!(metadata.is_active);
}
