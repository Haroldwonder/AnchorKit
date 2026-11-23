#![cfg(test)]

// Issue #959 – Verify property holder pagination at empty and final pages.
//
// "Holder" enumeration in this contract is exposed via `get_all_attestors`,
// which is the paginated registry of registered attestors (property holders in
// the property-token model).  The tests below exercise four boundary cases:
//
//   1. No holders — empty registry returns an empty page.
//   2. Exact-length page — offset 0, limit == total count returns all holders
//      with no duplicates and no omissions.
//   3. Partial last page — final page covers the tail when total is not a
//      multiple of the page size.
//   4. Start offset beyond end — requesting a page past the last holder returns
//      an empty list rather than panicking.
//
// No new enumeration API is introduced; all assertions use the existing public
// `get_all_attestors(offset: u64, limit: u32)` method.

mod holder_pagination_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Env,
    };
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::sep10_test_util::register_attestor_with_sep10;

    // -----------------------------------------------------------------------
    // Shared helpers
    // -----------------------------------------------------------------------

    fn make_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    fn setup_ledger(env: &Env) {
        env.ledger().set(LedgerInfo {
            timestamp: 1_700_000_000,
            protocol_version: 21,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 100,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6_312_000,
        });
    }

    /// Register `n` distinct attestors and return them in insertion order.
    fn register_n_holders(
        env: &Env,
        client: &AnchorKitContractClient,
        n: usize,
    ) -> soroban_sdk::Vec<Address> {
        let mut holders = soroban_sdk::Vec::new(env);
        for _ in 0..n {
            let holder = Address::generate(env);
            let sk = SigningKey::generate(&mut OsRng);
            register_attestor_with_sep10(env, client, &holder, &holder, &sk);
            holders.push_back(holder);
        }
        holders
    }

    fn setup(env: &Env) -> AnchorKitContractClient<'static> {
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin, &1_000_u64, &None, &None);
        client
    }

    // -----------------------------------------------------------------------
    // Test 1: empty registry – get_all_attestors returns an empty page
    // -----------------------------------------------------------------------

    #[test]
    fn test_holders_empty_registry_returns_empty_page() {
        let env = make_env();
        setup_ledger(&env);
        let client = setup(&env);

        let page = client.get_all_attestors(&0, &10);
        assert_eq!(
            page.len(),
            0,
            "expected empty page when no holders are registered"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: page exactly covers the full list (offset 0, limit == count)
    // -----------------------------------------------------------------------

    #[test]
    fn test_holders_exact_full_page_no_duplicates_no_omissions() {
        let env = make_env();
        setup_ledger(&env);
        let client = setup(&env);

        let holders = register_n_holders(&env, &client, 5);

        // Request exactly 5 — should get all 5, in insertion order, no dups.
        let page = client.get_all_attestors(&0, &5);
        assert_eq!(page.len(), 5, "expected exactly 5 holders");

        for i in 0..5u32 {
            assert_eq!(
                page.get(i).unwrap(),
                holders.get(i).unwrap(),
                "holder at position {} does not match",
                i
            );
        }

        // Verify no duplicates by checking all pairs are distinct.
        for i in 0..5u32 {
            for j in (i + 1)..5u32 {
                assert_ne!(
                    page.get(i).unwrap(),
                    page.get(j).unwrap(),
                    "duplicate holder at positions {} and {}",
                    i,
                    j
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Test 3: partial last page – tail is returned correctly
    // -----------------------------------------------------------------------

    #[test]
    fn test_holders_partial_last_page_returns_correct_tail() {
        let env = make_env();
        setup_ledger(&env);
        let client = setup(&env);

        // 7 holders, page size 3 → pages: [0..3), [3..6), [6..7) (1 item).
        let holders = register_n_holders(&env, &client, 7);

        let page0 = client.get_all_attestors(&0, &3);
        assert_eq!(page0.len(), 3);

        let page1 = client.get_all_attestors(&3, &3);
        assert_eq!(page1.len(), 3);

        // Final partial page: offset 6, limit 3 → should return exactly 1.
        let page2 = client.get_all_attestors(&6, &3);
        assert_eq!(page2.len(), 1, "partial last page should contain exactly 1 holder");
        assert_eq!(
            page2.get(0).unwrap(),
            holders.get(6).unwrap(),
            "partial last page should contain the 7th holder"
        );

        // Cross-page completeness: concatenating all pages yields all 7 distinct holders.
        let mut seen = soroban_sdk::Vec::new(&env);
        for i in 0..page0.len() { seen.push_back(page0.get(i).unwrap()); }
        for i in 0..page1.len() { seen.push_back(page1.get(i).unwrap()); }
        for i in 0..page2.len() { seen.push_back(page2.get(i).unwrap()); }
        assert_eq!(seen.len(), 7, "all pages together must cover all 7 holders");

        for i in 0..7u32 {
            assert!(
                seen.contains(&holders.get(i).unwrap()),
                "holder at original index {} is missing from paginated results",
                i
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 4: start offset beyond end – returns empty, does not panic
    // -----------------------------------------------------------------------

    #[test]
    fn test_holders_offset_beyond_end_returns_empty() {
        let env = make_env();
        setup_ledger(&env);
        let client = setup(&env);

        register_n_holders(&env, &client, 3);

        // Offset 3 is one past the last valid index (0..2).
        let page_at_end = client.get_all_attestors(&3, &10);
        assert_eq!(
            page_at_end.len(),
            0,
            "offset equal to list length must return empty page"
        );

        // Large offset must also return empty without panicking.
        let page_far = client.get_all_attestors(&u64::MAX, &10);
        assert_eq!(
            page_far.len(),
            0,
            "offset far beyond end must return empty page"
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: zero-limit — returns empty page (boundary guard)
    // -----------------------------------------------------------------------

    #[test]
    fn test_holders_zero_limit_returns_empty() {
        let env = make_env();
        setup_ledger(&env);
        let client = setup(&env);

        register_n_holders(&env, &client, 3);

        let page = client.get_all_attestors(&0, &0);
        assert_eq!(
            page.len(),
            0,
            "limit=0 must return an empty page"
        );
    }
}
