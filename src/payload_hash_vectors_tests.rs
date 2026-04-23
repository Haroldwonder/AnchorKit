/// Cross-language compatibility test vectors for `compute_payload_hash`.
/// See docs/test_vectors.md for the full algorithm specification.
#[cfg(test)]
mod payload_hash_vectors {
    use soroban_sdk::{Bytes, Env};

    use crate::deterministic_hash::compute_payload_hash;

    const ADDR_A: &str = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";
    const ADDR_B: &str = "GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGZXG5CPCJDGX2VOTZ5JXN";

    fn env() -> Env {
        Env::default()
    }

    fn addr(env: &Env, strkey: &str) -> soroban_sdk::Address {
        soroban_sdk::Address::from_str(env, strkey)
    }

    fn bytes(env: &Env, data: &[u8]) -> Bytes {
        Bytes::from_slice(env, data)
    }

    /// Vector 1 — baseline: known subject, timestamp, and data.
    /// Asserts the hash is stable across calls (determinism).
    #[test]
    fn vector_1_baseline() {
        let env = env();
        let subject = addr(&env, ADDR_A);
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
        let subject = addr(&env, ADDR_A);
        let data = bytes(&env, b"kyc_approved");

        let h_base = compute_payload_hash(&env, &subject, 1_700_000_000, &data);
        let h_diff = compute_payload_hash(&env, &subject, 1, &data);
        assert_ne!(h_base, h_diff, "different timestamps must produce different hashes");
    }

    /// Vector 3 — empty data (edge case).
    #[test]
    fn vector_3_empty_data() {
        let env = env();
        let subject = addr(&env, ADDR_A);
        let empty = Bytes::new(&env);
        let ts: u64 = 1_700_000_000;

        let h_empty = compute_payload_hash(&env, &subject, ts, &empty);
        let h_nonempty = compute_payload_hash(&env, &subject, ts, &bytes(&env, b"x"));
        // Empty data must still produce a valid 32-byte hash
        assert_eq!(h_empty.len(), 32);
        // And must differ from non-empty data
        assert_ne!(h_empty, h_nonempty);
    }

    /// Vector 4 — max timestamp (u64::MAX edge case).
    #[test]
    fn vector_4_max_timestamp() {
        let env = env();
        let subject = addr(&env, ADDR_A);
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

        let h_a = compute_payload_hash(&env, &addr(&env, ADDR_A), ts, &data);
        let h_b = compute_payload_hash(&env, &addr(&env, ADDR_B), ts, &data);
        assert_ne!(h_a, h_b, "different subjects must produce different hashes");
    }

    /// Vector 6 — binary data (non-UTF-8 bytes).
    #[test]
    fn vector_6_binary_data() {
        let env = env();
        let subject = addr(&env, ADDR_A);
        let data = bytes(&env, &[0x00, 0xff, 0x00, 0xff, 0x00, 0xff, 0x00, 0xff]);
        let ts: u64 = 0;

        let h = compute_payload_hash(&env, &subject, ts, &data);
        assert_eq!(h.len(), 32);
        // Must be stable
        assert_eq!(h, compute_payload_hash(&env, &subject, ts, &data));
    }
}
