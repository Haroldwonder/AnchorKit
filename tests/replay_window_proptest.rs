//! Property-based tests for the replay attack prevention window.
//!
//! These tests replace the hardcoded timestamp fixtures in
//! `src/replay_window_tests.rs` with randomly generated
//! `(now, window_seconds, nonce_offset)` triples via proptest.
//!
//! Gated behind the `proptest-tests` feature to keep normal CI fast.

#![cfg(feature = "proptest-tests")]

use ed25519_dalek::SigningKey;
use proptest::prelude::*;
use rand::rngs::OsRng;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Bytes, Env,
};

use anchorkit::contract::{AnchorKitContract, AnchorKitContractClient};
use anchorkit::sep10_test_util::{register_attestor_with_sep10, sign_attestation};

// ---------------------------------------------------------------------------
// Test harness helpers
// ---------------------------------------------------------------------------

fn make_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn set_ts(env: &Env, ts: u64) {
    env.ledger().set(LedgerInfo {
        timestamp: ts,
        protocol_version: 20,
        sequence_number: 1,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 16,
        min_persistent_entry_ttl: 4096,
        max_entry_ttl: 6_312_000,
    });
}

fn setup_with_window(
    env: &Env,
    window: Option<u64>,
) -> (AnchorKitContractClient, Address, SigningKey) {
    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let attestor = Address::generate(env);
    let sk = SigningKey::generate(&mut OsRng);

    client.initialize(&admin, &100_u64, &window);
    register_attestor_with_sep10(env, &client, &attestor, &attestor, &sk);

    (client, attestor, sk)
}

fn unique_hash(env: &Env, seed: u64) -> Bytes {
    let mut buf = [0u8; 32];
    buf[..8].copy_from_slice(&seed.to_le_bytes());
    buf[8] = 0xBE; // sentinel
    Bytes::from_slice(env, &buf)
}

// ---------------------------------------------------------------------------
// Property: any timestamp strictly within [now - window, now] must succeed
// ---------------------------------------------------------------------------
//
// We use bounded ranges to stay in the valid u64 domain:
//   now        ∈ [1_000, u64::MAX / 2]
//   window     ∈ [1, 3_600]          (1 second … 1 hour)
//   age_offset ∈ [0, window - 1]     (inside the acceptance window)

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        max_shrink_iters: 64,
        ..ProptestConfig::default()
    })]

    /// No false negatives: a timestamp that falls inside the configured window
    /// must always be accepted by `submit_attestation`.
    #[test]
    fn prop_no_false_negatives_inside_window(
        now        in 1_000_u64..=u64::MAX / 2,
        window     in 1_u64..=3_600_u64,
        age_offset in 0_u64..=3_599_u64,
        nonce      in 0_u64..u64::MAX,
    ) {
        // Clamp age_offset to at most window - 1 so the timestamp stays inside.
        let age = age_offset % window;
        let ts = now.saturating_sub(age);

        let env = make_env();
        set_ts(&env, now);
        let (client, attestor, sk) = setup_with_window(&env, Some(window));

        let hash = unique_hash(&env, nonce);
        let subject = Address::generate(&env);
        let sig  = sign_attestation(&env, &sk, &client.address, &attestor, &subject, ts, &hash);
        // Must not panic — an inside-window timestamp is always valid.
        let id = client.submit_attestation(&attestor, &subject, &ts, &hash, &sig);
        // The returned ID is the current attestation counter; must be ≥ 0.
        prop_assert!(id >= 0, "expected non-negative attestation id, got {}", id);
    }

    /// No false positives: a timestamp that is strictly outside the configured
    /// window must always be rejected (contract panics).
    #[test]
    fn prop_no_false_positives_outside_window(
        now    in 3_601_u64..=u64::MAX / 2,
        window in 1_u64..=3_600_u64,
        excess in 1_u64..=3_600_u64,
        nonce  in 0_u64..u64::MAX,
    ) {
        // age = window + excess  →  strictly outside the acceptance window.
        let age = window.saturating_add(excess);
        let ts  = now.saturating_sub(age);

        let env = make_env();
        set_ts(&env, now);
        let (client, attestor, sk) = setup_with_window(&env, Some(window));

        let hash = unique_hash(&env, nonce);
        let subject = Address::generate(&env);
        let sig  = sign_attestation(&env, &sk, &client.address, &attestor, &subject, ts, &hash);
        // The contract should panic (Soroban panics propagate as Err in catch_unwind).
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.submit_attestation(&attestor, &subject, &ts, &hash, &sig)
        }));
        prop_assert!(
            result.is_err(),
            "expected rejection for ts={} now={} window={} age={}",
            ts, now, window, age
        );
    }

    /// Future timestamps (ts > now) must always be rejected regardless of
    /// the configured window size.
    #[test]
    fn prop_future_timestamps_always_rejected(
        now    in 1_000_u64..u64::MAX / 2,
        window in 1_u64..=3_600_u64,
        future_offset in 1_u64..=3_600_u64,
        nonce  in 0_u64..u64::MAX,
    ) {
        let ts = now.saturating_add(future_offset);

        let env = make_env();
        set_ts(&env, now);
        let (client, attestor, sk) = setup_with_window(&env, Some(window));

        let hash = unique_hash(&env, nonce);
        let subject = Address::generate(&env);
        let sig  = sign_attestation(&env, &sk, &client.address, &attestor, &subject, ts, &hash);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.submit_attestation(&attestor, &subject, &ts, &hash, &sig)
        }));
        prop_assert!(
            result.is_err(),
            "expected rejection for future ts={} now={} window={}",
            ts, now, window
        );
    }

    /// Exact boundary (ts == now - window) must be accepted — the boundary
    /// itself is inclusive in the acceptance range.
    #[test]
    fn prop_boundary_timestamp_accepted(
        now    in 3_601_u64..=u64::MAX / 2,
        window in 1_u64..=3_600_u64,
        nonce  in 0_u64..u64::MAX,
    ) {
        // ts is exactly at the oldest accepted edge.
        let ts = now.saturating_sub(window);

        let env = make_env();
        set_ts(&env, now);
        let (client, attestor, sk) = setup_with_window(&env, Some(window));

        let hash = unique_hash(&env, nonce);
        let subject = Address::generate(&env);
        let sig  = sign_attestation(&env, &sk, &client.address, &attestor, &subject, ts, &hash);
        let id = client.submit_attestation(&attestor, &subject, &ts, &hash, &sig);
        prop_assert!(id >= 0, "expected acceptance at boundary ts={} now={} window={}", ts, now, window);
    }

    /// Distinct nonces within the same window must all be accepted — the replay
    /// guard must not confuse different payloads as duplicates.
    #[test]
    fn prop_distinct_nonces_no_replay_false_positive(
        now    in 1_000_u64..=u64::MAX / 2,
        window in 1_u64..=3_600_u64,
        // two independently drawn nonces; proptest guarantees they're independent
        nonce_a in 0_u64..u64::MAX / 2,
        nonce_b in u64::MAX / 2..u64::MAX,
    ) {
        // Timestamps both set to exactly `now` (inside the window).
        let env = make_env();
        set_ts(&env, now);
        let (client, attestor, sk) = setup_with_window(&env, Some(window));

        let hash_a = unique_hash(&env, nonce_a);
        let subject = Address::generate(&env);
        let sig_a  = sign_attestation(&env, &sk, &client.address, &attestor, &subject, now, &hash_a);
        let hash_b = unique_hash(&env, nonce_b);
        let sig_b  = sign_attestation(&env, &sk, &client.address, &attestor, &subject, now, &hash_b);

        // Both submissions must succeed because the payloads are distinct.
        let id_a = client.submit_attestation(&attestor, &subject, &now, &hash_a, &sig_a);
        let id_b = client.submit_attestation(&attestor, &subject, &now, &hash_b, &sig_b);

        prop_assert!(id_a >= 0);
        prop_assert!(id_b >= 0);
        prop_assert_ne!(id_a, id_b, "duplicate attestation IDs for distinct nonces");
    }

    /// Replayed nonce (same payload hash submitted twice) must be rejected the
    /// second time regardless of timestamp or window size.
    #[test]
    fn prop_replay_always_rejected(
        now    in 1_000_u64..=u64::MAX / 2,
        window in 1_u64..=3_600_u64,
        nonce  in 0_u64..u64::MAX,
    ) {
        let env = make_env();
        set_ts(&env, now);
        let (client, attestor, sk) = setup_with_window(&env, Some(window));

        let hash = unique_hash(&env, nonce);
        let subject = Address::generate(&env);
        let sig  = sign_attestation(&env, &sk, &client.address, &attestor, &subject, now, &hash);
        // First submission: must succeed.
        client.submit_attestation(&attestor, &subject, &now, &hash, &sig);

        // Second submission with the same hash: must be rejected.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.submit_attestation(&attestor, &subject, &now, &hash, &sig)
        }));
        prop_assert!(
            result.is_err(),
            "expected replay rejection for nonce={} now={} window={}",
            nonce, now, window
        );
    }
}
