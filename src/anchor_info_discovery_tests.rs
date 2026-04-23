The #![cfg(test)]

mod anchor_info_discovery_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Env, String, Vec,
    };

 feat/get-anchor-currencies
    use crate::contract::{AnchorKitContract, AnchorKitContractClient, AssetInfo, FiatCurrency, StellarToml};

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::types::{AssetInfo, StellarToml};
 main

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

    fn usdc_asset(env: &Env) -> AssetInfo {
        AssetInfo {
            code: String::from_str(env, "USDC"),
            issuer: String::from_str(env, "GABC123"),
            deposit_enabled: true,
            withdrawal_enabled: true,
            deposit_fee_fixed: 100,
            deposit_fee_percent: 10,
            withdrawal_fee_fixed: 50,
            withdrawal_fee_percent: 5,
            deposit_min_amount: 1000,
            deposit_max_amount: 1_000_000,
            withdrawal_min_amount: 500,
            withdrawal_max_amount: 500_000,
        }
    }

    fn xlm_asset(env: &Env) -> AssetInfo {
        AssetInfo {
            code: String::from_str(env, "XLM"),
            issuer: String::from_str(env, "native"),
            deposit_enabled: true,
            withdrawal_enabled: true,
            deposit_fee_fixed: 0,
            deposit_fee_percent: 0,
            withdrawal_fee_fixed: 0,
            withdrawal_fee_percent: 0,
            deposit_min_amount: 100,
            deposit_max_amount: 10_000_000,
            withdrawal_min_amount: 100,
            withdrawal_max_amount: 10_000_000,
        }
    }

    fn sample_toml(env: &Env) -> StellarToml {
        let mut currencies = Vec::new(env);
        currencies.push_back(usdc_asset(env));
        currencies.push_back(xlm_asset(env));

        let mut accounts = Vec::new(env);
        accounts.push_back(String::from_str(env, "GANCHOR1"));

        StellarToml {
            version: String::from_str(env, "2.0.0"),
            network_passphrase: String::from_str(env, "Test SDF Network ; September 2015"),
            accounts,
            signing_key: Some(String::from_str(env, "GSIGN123")),
            currencies,
            fiat_currencies: Vec::new(env),
            transfer_server: String::from_str(env, "https://api.example.com"),
            transfer_server_sep0024: String::from_str(env, "https://api.example.com/sep24"),
            kyc_server: String::from_str(env, "https://kyc.example.com"),
            web_auth_endpoint: String::from_str(env, "https://auth.example.com"),
        }
    }

    fn setup(env: &Env) -> (AnchorKitContractClient, Address) {
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(env, &contract_id);
        let anchor = Address::generate(env);
        (client, anchor)
    }

    #[test]
    fn test_fetch_and_cache_toml() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        let toml = client.get_anchor_toml(&anchor);
        assert_eq!(toml.version, String::from_str(&env, "2.0.0"));
        assert_eq!(toml.signing_key, Some(String::from_str(&env, "GSIGN123")));
    }

    #[test]
    fn test_signing_key_is_none_when_not_provided() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        let mut currencies = Vec::new(&env);
        currencies.push_back(usdc_asset(&env));
        let mut accounts = Vec::new(&env);
        accounts.push_back(String::from_str(&env, "GANCHOR1"));

        let toml_no_key = StellarToml {
            version: String::from_str(&env, "2.0.0"),
            network_passphrase: String::from_str(&env, "Test SDF Network ; September 2015"),
            accounts,
            signing_key: None,
            currencies,
            transfer_server: String::from_str(&env, "https://api.example.com"),
            transfer_server_sep0024: String::from_str(&env, "https://api.example.com/sep24"),
            kyc_server: String::from_str(&env, "https://kyc.example.com"),
            web_auth_endpoint: String::from_str(&env, "https://auth.example.com"),
        };

        client.fetch_anchor_info(&anchor, &toml_no_key, &3600u64);

        let toml = client.get_anchor_toml(&anchor);
        assert_eq!(toml.signing_key, None);
    }

    #[test]
    fn test_get_cached_toml() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        let toml = client.get_anchor_toml(&anchor);
        assert_eq!(toml.network_passphrase, String::from_str(&env, "Test SDF Network ; September 2015"));
        assert_eq!(toml.transfer_server, String::from_str(&env, "https://api.example.com"));
    }

    #[test]
    fn test_cache_not_found() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        let result = client.try_get_anchor_toml(&anchor);
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_expiration() {
        let env = make_env();
        set_ledger(&env, 1000);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(1u64));

        set_ledger(&env, 1002);
        let result = client.try_get_anchor_toml(&anchor);
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_ttl_custom() {
        let env = make_env();
        set_ledger(&env, 1000);
        let (client, anchor) = setup(&env);

        // Override TTL to 7200s; cache should still be valid at timestamp 5000
        // (1000 + 7200 = 8200 > 5000)
        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(7200u64));
        set_ledger(&env, 5000);
        assert!(client.try_get_anchor_toml(&anchor).is_ok(), "cache should be valid within custom TTL");

        // At timestamp 9000: 1000 + 7200 = 8200 < 9000, so expired
        set_ledger(&env, 9000);
        assert!(client.try_get_anchor_toml(&anchor).is_err(), "cache should be expired after custom TTL");

        // None falls back to default 3600s TTL
        let anchor2 = Address::generate(&env);
        set_ledger(&env, 1000);
        client.fetch_anchor_info(&anchor2, &sample_toml(&env), &None);
        set_ledger(&env, 5000);
        assert!(client.try_get_anchor_toml(&anchor2).is_err(), "default 3600s TTL should expire at 5000");
    }

    #[test]
    fn test_get_supported_assets() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        let assets = client.get_anchor_assets(&anchor).unwrap();
        assert_eq!(assets.len(), 2);
        assert!(assets.contains(&String::from_str(&env, "USDC")));
        assert!(assets.contains(&String::from_str(&env, "XLM")));
    }

    #[test]
    fn test_get_asset_info() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        let info = client.get_anchor_asset_info(&anchor, &String::from_str(&env, "USDC"));
        assert_eq!(info.issuer, String::from_str(&env, "GABC123"));
        assert_eq!(info.deposit_fee_fixed, 100);
    }

    #[test]
    fn test_get_asset_info_not_found() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        let result = client.try_get_anchor_asset_info(&anchor, &String::from_str(&env, "BTC"));
        assert!(result.is_err());
    }

    #[test]
    fn test_get_anchor_assets_uncached_returns_error() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        // No fetch_anchor_info call — cache is empty
        let result = client.try_get_anchor_assets(&anchor);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            crate::errors::ErrorCode::CacheNotFound
        );
    }

    #[test]
    fn test_get_deposit_limits() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        let (min, max) = client.get_anchor_deposit_limits(&anchor, &String::from_str(&env, "USDC"));
        assert_eq!(min, 1000);
        assert_eq!(max, 1_000_000);
    }

    // Issue #274: uncached anchor must return Err(CacheNotFound), not (0, 0)
    #[test]
    fn test_get_deposit_limits_uncached_returns_error() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        let result = client.try_get_anchor_deposit_limits(&anchor, &String::from_str(&env, "USDC"));
        assert!(result.is_err(), "expected CacheNotFound error for uncached anchor");
    }

    #[test]
    fn test_get_withdrawal_limits() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        let (min, max) = client.get_anchor_withdrawal_limits(&anchor, &String::from_str(&env, "USDC"));
        assert_eq!(min, 500);
        assert_eq!(max, 500_000);
    }

    // Issue #274: uncached anchor must return Err(CacheNotFound), not (0, 0)
    #[test]
    fn test_get_withdrawal_limits_uncached_returns_error() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        let result = client.try_get_anchor_withdrawal_limits(&anchor, &String::from_str(&env, "USDC"));
        assert!(result.is_err(), "expected CacheNotFound error for uncached anchor");
    }

    #[test]
    fn test_get_deposit_fees() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        let (fixed, percent) = client.get_anchor_deposit_fees(&anchor, &String::from_str(&env, "USDC"));
        assert_eq!(fixed, 100);
        assert_eq!(percent, 10);
    }

    #[test]
    fn test_get_withdrawal_fees() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        let (fixed, percent) = client.get_anchor_withdrawal_fees(&anchor, &String::from_str(&env, "USDC"));
        assert_eq!(fixed, 50);
        assert_eq!(percent, 5);
    }

    #[test]
    fn test_supports_deposits() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        assert!(client.anchor_supports_deposits(&anchor, &String::from_str(&env, "USDC")));
    }

    #[test]
    fn test_supports_withdrawals() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        assert!(client.anchor_supports_withdrawals(&anchor, &String::from_str(&env, "USDC")));
    }

    #[test]
    fn test_refresh_cache() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));
        let _ = client.get_anchor_toml(&anchor);

        client.refresh_anchor_info(&anchor, &true);

        let result = client.try_get_anchor_toml(&anchor);
        assert!(result.is_err());
    }

    #[test]
    fn test_refresh_cache_force_false() {
        let env = make_env();
        set_ledger(&env, 1000);
        let (client, anchor) = setup(&env);

        // Cache with 3600s TTL
        client.fetch_anchor_info(&anchor, &sample_toml(&env), &3600u64);
        
        // At 2000, still valid
        set_ledger(&env, 2000);
        client.refresh_anchor_info(&anchor, &false);

        // Should still be in cache because force=false and not expired
        let result = client.try_get_anchor_toml(&anchor);
        assert!(result.is_ok());

        // At 5000, expired
        set_ledger(&env, 5000);
        client.refresh_anchor_info(&anchor, &false);

        // Should be gone now
        let result = client.try_get_anchor_toml(&anchor);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_assets() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        let usdc_info = client.get_anchor_asset_info(&anchor, &String::from_str(&env, "USDC"));
        let xlm_info = client.get_anchor_asset_info(&anchor, &String::from_str(&env, "XLM"));

        assert_eq!(usdc_info.deposit_fee_fixed, 100);
        assert_eq!(xlm_info.deposit_fee_fixed, 0);
    }

    #[test]
    fn test_xlm_native_asset() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        let info = client.get_anchor_asset_info(&anchor, &String::from_str(&env, "XLM"));
        assert_eq!(info.issuer, String::from_str(&env, "native"));
        assert_eq!(info.deposit_fee_fixed, 0);
        assert_eq!(info.deposit_fee_percent, 0);
    }

    #[test]
    fn test_multiple_anchors() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor1) = setup(&env);
        let anchor2 = Address::generate(&env);

        let mut currencies2 = Vec::new(&env);
        currencies2.push_back(AssetInfo {
            code: String::from_str(&env, "USDC"),
            issuer: String::from_str(&env, "GOTHER"),
            deposit_enabled: true,
            withdrawal_enabled: false,
            deposit_fee_fixed: 200,
            deposit_fee_percent: 20,
            withdrawal_fee_fixed: 0,
            withdrawal_fee_percent: 0,
            deposit_min_amount: 500,
            deposit_max_amount: 500_000,
            withdrawal_min_amount: 0,
            withdrawal_max_amount: 0,
        });
        let mut accounts2 = Vec::new(&env);
        accounts2.push_back(String::from_str(&env, "GANCHOR2"));
        let toml2 = StellarToml {
            version: String::from_str(&env, "2.0.0"),
            network_passphrase: String::from_str(&env, "Test SDF Network ; September 2015"),
            accounts: accounts2,
            signing_key: Some(String::from_str(&env, "GSIGN456")),
            currencies: currencies2,
            fiat_currencies: Vec::new(&env),
            transfer_server: String::from_str(&env, "https://api2.example.com"),
            transfer_server_sep0024: String::from_str(&env, "https://api2.example.com/sep24"),
            kyc_server: String::from_str(&env, "https://kyc2.example.com"),
            web_auth_endpoint: String::from_str(&env, "https://auth2.example.com"),
        };

        client.fetch_anchor_info(&anchor1, &sample_toml(&env), &Some(3600u64));
        client.fetch_anchor_info(&anchor2, &toml2, &Some(3600u64));

        let info1 = client.get_anchor_asset_info(&anchor1, &String::from_str(&env, "USDC"));
        let info2 = client.get_anchor_asset_info(&anchor2, &String::from_str(&env, "USDC"));

        assert_eq!(info1.issuer, String::from_str(&env, "GABC123"));
        assert_eq!(info2.issuer, String::from_str(&env, "GOTHER"));
        assert_eq!(info1.deposit_fee_fixed, 100);
        assert_eq!(info2.deposit_fee_fixed, 200);
    }

    #[test]
    fn test_asset_limits_validation() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        let (dep_min, dep_max) = client.get_anchor_deposit_limits(&anchor, &String::from_str(&env, "USDC"));
        let (wit_min, wit_max) = client.get_anchor_withdrawal_limits(&anchor, &String::from_str(&env, "USDC"));

        assert!(dep_min < dep_max);
        assert!(wit_min < wit_max);
        assert_eq!(dep_min, 1000);
        assert_eq!(dep_max, 1_000_000);
        assert_eq!(wit_min, 500);
        assert_eq!(wit_max, 500_000);
    }

    #[test]
    fn test_fee_structure() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        let (dep_fixed, dep_pct) = client.get_anchor_deposit_fees(&anchor, &String::from_str(&env, "USDC"));
        let (wit_fixed, wit_pct) = client.get_anchor_withdrawal_fees(&anchor, &String::from_str(&env, "USDC"));

        assert_eq!(dep_fixed, 100);
        assert_eq!(dep_pct, 10);
        assert_eq!(wit_fixed, 50);
        assert_eq!(wit_pct, 5);
    }

 feat/get-anchor-currencies
    #[test]
    fn test_get_anchor_currencies_with_fiat_entries() {

    // Issue #277: zero-fee anchors (common on testnet) must be handled without divide-by-zero
    #[test]
    fn test_fee_structure_zero_fee_anchor() {
 main
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

 feat/get-anchor-currencies
        let mut fiat = Vec::new(&env);
        fiat.push_back(FiatCurrency {
            code: String::from_str(&env, "USD"),
            name: String::from_str(&env, "US Dollar"),
            deposit_enabled: true,
            withdrawal_enabled: true,
        });
        fiat.push_back(FiatCurrency {
            code: String::from_str(&env, "EUR"),
            name: String::from_str(&env, "Euro"),
            deposit_enabled: true,
            withdrawal_enabled: false,
        });

        let mut currencies = Vec::new(&env);
        currencies.push_back(usdc_asset(&env));
        let mut accounts = Vec::new(&env);
        accounts.push_back(String::from_str(&env, "GANCHOR1"));

        let toml = StellarToml {
            version: String::from_str(&env, "2.0.0"),
            network_passphrase: String::from_str(&env, "Test SDF Network ; September 2015"),
            accounts,
            signing_key: String::from_str(&env, "GSIGN123"),
            currencies,
            fiat_currencies: fiat,
            transfer_server: String::from_str(&env, "https://api.example.com"),
            transfer_server_sep0024: String::from_str(&env, "https://api.example.com/sep24"),
            kyc_server: String::from_str(&env, "https://kyc.example.com"),
            web_auth_endpoint: String::from_str(&env, "https://auth.example.com"),
        };

        client.fetch_anchor_info(&anchor, &toml, &3600u64);

        let result = client.get_anchor_currencies(&anchor).unwrap();
        assert_eq!(result.len(), 2);

        let usd = result.get(0).unwrap();
        assert_eq!(usd.code, String::from_str(&env, "USD"));
        assert_eq!(usd.name, String::from_str(&env, "US Dollar"));
        assert!(usd.deposit_enabled);
        assert!(usd.withdrawal_enabled);

        let eur = result.get(1).unwrap();
        assert_eq!(eur.code, String::from_str(&env, "EUR"));
        assert!(eur.deposit_enabled);
        assert!(!eur.withdrawal_enabled);
    }

    #[test]
    fn test_get_anchor_currencies_empty_when_none_defined() {
        let env = make_env();
        set_ledger(&env, 0);
        let (client, anchor) = setup(&env);

        client.fetch_anchor_info(&anchor, &sample_toml(&env), &Some(3600u64));

        // XLM asset has fee_fixed = 0 and fee_percent = 0 (see xlm_asset helper)
        let (dep_fixed, dep_pct) = client.get_anchor_deposit_fees(&anchor, &String::from_str(&env, "XLM"));
        let (wit_fixed, wit_pct) = client.get_anchor_withdrawal_fees(&anchor, &String::from_str(&env, "XLM"));

        assert_eq!(dep_fixed, 0, "zero-fee anchor deposit fixed fee should be 0");
        assert_eq!(dep_pct, 0, "zero-fee anchor deposit percent fee should be 0");
        assert_eq!(wit_fixed, 0, "zero-fee anchor withdrawal fixed fee should be 0");
        assert_eq!(wit_pct, 0, "zero-fee anchor withdrawal percent fee should be 0");
 main
    }
}
