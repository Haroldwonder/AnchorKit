#![cfg(test)]

mod metadata_cache_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Env, String,
    };

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::types::AnchorMetadata;

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

    fn sample_metadata(env: &Env, anchor: &Address) -> AnchorMetadata {
        AnchorMetadata {
            anchor: anchor.clone(),
            reputation_score: 9000,
            liquidity_score: 8500,
            uptime_percentage: 9900,
            total_volume: 1_000_000,
            average_settlement_time: 300,
            is_active: true,
        }
    }

    #[test]
    fn test_cache_not_found() {
        let env = make_env();
        set_ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        let result = client.try_get_cached_metadata(&anchor);
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_and_retrieve_metadata() {
        let env = make_env();
        set_ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        let meta = sample_metadata(&env, &anchor);
        client.cache_metadata(&anchor, &meta, &3600u64);

        let retrieved = client.get_cached_metadata(&anchor);
        assert_eq!(retrieved.reputation_score, 9000);
        assert_eq!(retrieved.is_active, true);
    }

    #[test]
    fn test_cache_expiration() {
        let env = make_env();
        set_ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        let meta = sample_metadata(&env, &anchor);
        client.cache_metadata(&anchor, &meta, &10u64);

        // advance past TTL
        set_ledger(&env, 11);
        let result = client.try_get_cached_metadata(&anchor);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_ttl_never_expires() {
        // ttl_seconds = 0 must be treated as "never expire", not as immediately expired.
        let env = make_env();
        set_ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        let meta = sample_metadata(&env, &anchor);
        client.cache_metadata(&anchor, &meta, &0u64);

        // Advance time arbitrarily far — the entry must still be accessible
        set_ledger(&env, 999_999_999);
        let result = client.try_get_cached_metadata(&anchor);
        assert!(result.is_ok(), "ttl_seconds=0 entry should never expire");
        assert_eq!(result.unwrap().unwrap().reputation_score, 9000);
    }

    #[test]
    fn test_manual_refresh() {
        let env = make_env();
        set_ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        let meta = sample_metadata(&env, &anchor);
        client.cache_metadata(&anchor, &meta, &3600u64);

        // verify it's there
        let _ = client.get_cached_metadata(&anchor);

        // #272: refresh now returns the cached data so callers avoid a second read
        let refreshed = client.refresh_metadata_cache(&anchor);
        assert_eq!(refreshed.reputation_score, 9000);
        assert_eq!(refreshed.is_active, true);

        // entry is gone after refresh
        let result = client.try_get_cached_metadata(&anchor);
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_capabilities() {
        let env = make_env();
        set_ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        let toml_url = String::from_str(&env, "https://anchor.example/.well-known/stellar.toml");
        let mut caps = soroban_sdk::Vec::new(&env);
        caps.push_back(1u32); // SERVICE_DEPOSITS
        caps.push_back(2u32); // SERVICE_WITHDRAWALS
        client.cache_capabilities(&anchor, &toml_url, &caps, &3600u64);

        let cached = client.get_cached_capabilities(&anchor);
        assert_eq!(cached.capabilities, caps);
        assert_eq!(cached.toml_url, toml_url);
    }

    #[test]
    fn test_capabilities_expiration() {
        let env = make_env();
        set_ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        let toml_url = String::from_str(&env, "https://anchor.example/.well-known/stellar.toml");
        let mut caps = soroban_sdk::Vec::new(&env);
        caps.push_back(1u32);
        client.cache_capabilities(&anchor, &toml_url, &caps, &5u64);

        set_ledger(&env, 6);
        let result = client.try_get_cached_capabilities(&anchor);
        assert!(result.is_err());
    }

    #[test]
    fn test_refresh_capabilities() {
        let env = make_env();
        set_ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        let toml_url = String::from_str(&env, "https://anchor.example/.well-known/stellar.toml");
        let mut caps = soroban_sdk::Vec::new(&env);
        caps.push_back(1u32);
        client.cache_capabilities(&anchor, &toml_url, &caps, &3600u64);

        client.refresh_capabilities_cache(&anchor);

        let result = client.try_get_cached_capabilities(&anchor);
        assert!(result.is_err());
    }

    // Issue #259: cache_metadata skips write when data is unchanged
    #[test]
    fn test_cache_metadata_no_write_if_unchanged() {
        let env = make_env();
        set_ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        let meta = sample_metadata(&env, &anchor);
        client.cache_metadata(&anchor, &meta, &3600u64);

        // Advance time and call again with identical metadata — cached_at should NOT update
        set_ledger(&env, 100);
        client.cache_metadata(&anchor, &meta, &3600u64);

        // The cache entry should still have cached_at == 0 (original write)
        // We verify by checking the age is >= 100 seconds
        let age = client.get_cache_age_seconds(&anchor);
        assert!(age >= 100);
    }

    // Issue #260: get_cache_age_seconds returns None when no entry, Some(age) when cached
    #[test]
    fn test_get_cache_age_seconds() {
        let env = make_env();
        set_ledger(&env, 1000);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        // No entry yet — must return Err(CacheNotFound), not Ok(0)
        assert!(client.try_get_cache_age_seconds(&anchor).is_err());

        let meta = sample_metadata(&env, &anchor);
        client.cache_metadata(&anchor, &meta, &3600u64);

        // Advance 50 seconds
        set_ledger(&env, 1050);
        let age = client.get_cache_age_seconds(&anchor);
        assert_eq!(age, 50);
    }

    // Issue #258: invalidate_all_caches removes all entries and emits event
    #[test]
    fn test_invalidate_all_caches() {
        let env = make_env();
        set_ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let anchor1 = Address::generate(&env);
        let anchor2 = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        let meta1 = sample_metadata(&env, &anchor1);
        let meta2 = sample_metadata(&env, &anchor2);
        client.cache_metadata(&anchor1, &meta1, &3600u64);
        client.cache_metadata(&anchor2, &meta2, &3600u64);

        // Both readable before flush
        assert!(client.try_get_cached_metadata(&anchor1).is_ok());
        assert!(client.try_get_cached_metadata(&anchor2).is_ok());

        client.invalidate_all_caches();

        // Both gone after flush
        assert!(client.try_get_cached_metadata(&anchor1).is_err());
        assert!(client.try_get_cached_metadata(&anchor2).is_err());

        // Anchor list also cleared
        assert_eq!(client.list_cached_anchors().len(), 0);
    }

    // Issue #276: list_cached_anchors returns all anchors with active cache entries
    #[test]
    fn test_list_cached_anchors() {
        let env = make_env();
        set_ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let anchor1 = Address::generate(&env);
        let anchor2 = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        // Initially empty
        let list = client.list_cached_anchors();
        assert_eq!(list.len(), 0);

        // Cache metadata for anchor1
        let meta1 = sample_metadata(&env, &anchor1);
        client.cache_metadata(&anchor1, &meta1, &3600u64);

        let list = client.list_cached_anchors();
        assert_eq!(list.len(), 1);
        assert!(list.contains(&anchor1));

        // Cache metadata for anchor2
        let meta2 = sample_metadata(&env, &anchor2);
        client.cache_metadata(&anchor2, &meta2, &3600u64);

        let list = client.list_cached_anchors();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&anchor1));
        assert!(list.contains(&anchor2));

        // Invalidate anchor1 — it should be removed from the list
        let _ = client.refresh_metadata_cache(&anchor1);

        let list = client.list_cached_anchors();
        assert_eq!(list.len(), 1);
        assert!(!list.contains(&anchor1));
        assert!(list.contains(&anchor2));
    }

    // Issue #458: cache_metadata must append anchor to CANCHORS persistent list
    #[test]
    fn test_cache_metadata_appends_to_canchors() {
        let env = make_env();
        set_ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        client.initialize(&admin, &100_u64, &None);

        // Before caching, list is empty
        assert_eq!(client.list_cached_anchors().len(), 0);

        let meta = sample_metadata(&env, &anchor);
        client.cache_metadata(&anchor, &meta, &3600u64);

        // After caching, anchor appears in list
        let list = client.list_cached_anchors();
        assert_eq!(list.len(), 1);
        assert!(list.contains(&anchor));

        // Calling cache_metadata again with same anchor must not duplicate it
        client.cache_metadata(&anchor, &sample_metadata(&env, &anchor), &7200u64);
        assert_eq!(client.list_cached_anchors().len(), 1);
    }
}
