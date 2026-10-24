/// Cross-language compatibility test vectors for `compute_payload_hash`.
/// See docs/test_vectors.md for the full algorithm specification.
///
/// Soroban `Address` values use contract-account XDR encoding in tests; G-strkey
/// vectors in the doc are verified by off-chain tooling. These tests assert the
/// same determinism and sensitivity properties using generated addresses.
#[cfg(test)]
mod payload_hash_vectors {
    use soroban_sdk::{testutils::Address as _, Address, Bytes, Env};

    use crate::deterministic_hash::compute_payload_hash;

    fn env() -> Env {
        Env::default()
    }

    fn bytes(env: &Env, data: &[u8]) -> Bytes {
        Bytes::from_slice(env, data)
    }

    /// Vector 1 — baseline: known subject, timestamp, and data.
    /// Asserts the hash is stable across calls (determinism).
    #[test]
    fn vector_1_baseline() {
        let env = env();
        let subject = Address::generate(&env);
        let data = bytes(&env, b"kyc_approved");
        let ts: u64 = 1_700_000_000;

        let h1 = compute_payload_hash(&env, &subject, ts, &data);
        let h2 = compute_payload_hash(&env, &subject, ts, &data);
        assert_eq!(h1, h2, "hash must be deterministic");
    }

    /// Vector 2 — different timestamp produces a different hash.
    #[test]
    fn vector_2_different_timestamp() {
        let env = env();
        let subject = Address::generate(&env);
        let data = bytes(&env, b"kyc_approved");

        let h_base = compute_payload_hash(&env, &subject, 1_700_000_000, &data);
        let h_diff = compute_payload_hash(&env, &subject, 1, &data);
        assert_ne!(h_base, h_diff, "different timestamps must produce different hashes");
    }

    /// Vector 3 — empty data (edge case).
    #[test]
    fn vector_3_empty_data() {
        let env = env();
        let subject = Address::generate(&env);
        let empty = Bytes::new(&env);
        let ts: u64 = 1_700_000_000;

        let h_empty = compute_payload_hash(&env, &subject, ts, &empty);
        let h_nonempty = compute_payload_hash(&env, &subject, ts, &bytes(&env, b"x"));
        assert_eq!(h_empty.len(), 32);
        assert_ne!(h_empty, h_nonempty);
    }

    /// Vector 4 — max timestamp (u64::MAX edge case).
    #[test]
    fn vector_4_max_timestamp() {
        let env = env();
        let subject = Address::generate(&env);
        let data = bytes(&env, b"payment_confirmed");

        let h_max = compute_payload_hash(&env, &subject, u64::MAX, &data);
        let h_normal = compute_payload_hash(&env, &subject, 1_700_000_000, &data);
        assert_eq!(h_max.len(), 32);
        assert_ne!(h_max, h_normal, "max timestamp must produce a different hash");
    }

    /// Vector 5 — different subject produces a different hash.
    #[test]
    fn vector_5_different_subject() {
        let env = env();
        let data = bytes(&env, b"kyc_approved");
        let ts: u64 = 1_700_000_000;

        let h_a = compute_payload_hash(&env, &Address::generate(&env), ts, &data);
        let h_b = compute_payload_hash(&env, &Address::generate(&env), ts, &data);
        assert_ne!(h_a, h_b, "different subjects must produce different hashes");
    }

    /// Vector 6 — binary data (non-UTF-8 bytes).
    #[test]
    fn vector_6_binary_data() {
        let env = env();
        let subject = Address::generate(&env);
        let data = bytes(&env, &[0x00, 0xff, 0x00, 0xff, 0x00, 0xff, 0x00, 0xff]);
        let ts: u64 = 0;

        let h = compute_payload_hash(&env, &subject, ts, &data);
        assert_eq!(h.len(), 32);
        assert_eq!(h, compute_payload_hash(&env, &subject, ts, &data));
    }
}
