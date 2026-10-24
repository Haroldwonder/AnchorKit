#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{testutils::Address as _, Bytes, Env};

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    let env = Env::default();

    let timestamp = u64::from_be_bytes(data[..8].try_into().unwrap());
    let payload = &data[8..];

    let subject = soroban_sdk::Address::generate(&env);
    let bytes = Bytes::from_slice(&env, payload);

    // Must not panic regardless of input
    let hash = anchorkit::deterministic_hash::compute_payload_hash(&env, &subject, timestamp, &bytes);

    // Determinism: same inputs must always produce the same hash
    let hash2 = anchorkit::deterministic_hash::compute_payload_hash(&env, &subject, timestamp, &bytes);
    assert_eq!(hash, hash2, "hash is non-deterministic");

    // verify_payload_hash must agree with itself
    assert!(anchorkit::deterministic_hash::verify_payload_hash(&hash, &hash));
});
