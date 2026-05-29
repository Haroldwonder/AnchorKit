use soroban_sdk::{xdr::ToXdr, Address, Bytes, BytesN, Env};

/// Compute a canonical SHA-256 hash over attestation payload fields.
///
/// Field ordering is fixed (canonical): subject bytes || timestamp (8-byte BE) || data bytes.
/// This guarantees the same inputs always produce the same 32-byte hash.
///
/// # Off-chain callers
///
/// Prefer calling the contract method `compute_payload_hash_public` instead of this function
/// directly. The contract method goes through the Soroban host environment, which ensures
/// `Address` XDR serialisation uses the same host-side encoding as on-chain attestation
/// submission. Calling this module function outside the host context may produce different
/// XDR bytes for `Address` types, causing hash divergence that breaks attestation verification.
pub fn compute_payload_hash(
    env: &Env,
    subject: &Address,
    timestamp: u64,
    data: &Bytes,
) -> BytesN<32> {
    let mut input = Bytes::new(env);

    // 1. subject — serialised as its raw XDR bytes via to_xdr
    let subject_bytes = subject.clone().to_xdr(env);
    input.append(&subject_bytes);

    // 2. timestamp — 8-byte big-endian
    for b in timestamp.to_be_bytes().iter() {
        input.push_back(*b);
    }

    // 3. data payload
    input.append(data);

    env.crypto().sha256(&input).into()
}

/// Verify that the stored attestation's payload hash matches the expected hash.
///
/// Returns `true` when the hashes are equal.
pub fn verify_payload_hash(stored: &BytesN<32>, expected: &BytesN<32>) -> bool {
    stored == expected
}

#[cfg(test)]
mod deterministic_hash_tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_same_inputs_produce_same_hash() {
        let env = Env::default();
        let subject = Address::generate(&env);
        let data = Bytes::from_slice(&env, b"kyc_approved");
        let ts: u64 = 1_700_000_000;

        let h1 = compute_payload_hash(&env, &subject, ts, &data);
        let h2 = compute_payload_hash(&env, &subject, ts, &data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_different_timestamp_produces_different_hash() {
        let env = Env::default();
        let subject = Address::generate(&env);
        let data = Bytes::from_slice(&env, b"kyc_approved");

        let h1 = compute_payload_hash(&env, &subject, 1_000, &data);
        let h2 = compute_payload_hash(&env, &subject, 2_000, &data);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_different_data_produces_different_hash() {
        let env = Env::default();
        let subject = Address::generate(&env);
        let ts: u64 = 1_700_000_000;

        let h1 = compute_payload_hash(&env, &subject, ts, &Bytes::from_slice(&env, b"data_a"));
        let h2 = compute_payload_hash(&env, &subject, ts, &Bytes::from_slice(&env, b"data_b"));
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_verify_payload_hash_match() {
        let env = Env::default();
        let subject = Address::generate(&env);
        let data = Bytes::from_slice(&env, b"payment_confirmed");
        let ts: u64 = 1_700_000_000;

        let hash = compute_payload_hash(&env, &subject, ts, &data);
        assert!(verify_payload_hash(&hash, &hash));
    }

    #[test]
    fn test_verify_payload_hash_mismatch() {
        let env = Env::default();
        let subject = Address::generate(&env);
        let data = Bytes::from_slice(&env, b"payment_confirmed");
        let ts: u64 = 1_700_000_000;

        let h1 = compute_payload_hash(&env, &subject, ts, &data);
        let h2 = compute_payload_hash(&env, &subject, ts + 1, &data);
        assert!(!verify_payload_hash(&h1, &h2));
    }
}
