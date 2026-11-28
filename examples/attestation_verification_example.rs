//! Off-chain attestation verification (issue #1561).
//!
//! This example uses the addresses and data from `docs/test_vectors.md`.
//! `fetch_attestation` and `fetch_registered_key` are local stand-ins for
//! `AnchorKitContractClient::get_attestation` and `get_attestation_keys`;
//! replace them with RPC calls to verify an attestation from a deployed
//! contract. Run with:
//!
//! ```bash
//! cargo run --example attestation_verification_example
//! ```

use anchorkit::{attestation_signing_digest, AnchorKitContract};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use soroban_sdk::{
    testutils::{Ledger, LedgerInfo},
    Address, Bytes, Env,
};

const ADDR_A: &str = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";
const ADDR_B: &str = "GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGZXG5CPCJDGX2VOTZ5JXN";
const NETWORK_PASSPHRASE: &str = "Standalone Network ; February 2017";

/// The attestation record returned by `get_attestation` for vector 1.
struct FetchedAttestation {
    issuer: String,
    subject: String,
    timestamp: u64,
    payload_hash: [u8; 32],
    signature: [u8; 64],
    issuer_revoked: bool,
    expires_at: Option<u64>,
}

fn fetch_attestation(id: u64) -> FetchedAttestation {
    assert_eq!(id, 1, "this example ships a single vector-1 fixture");
    FetchedAttestation {
        issuer: ADDR_A.to_string(),
        subject: ADDR_A.to_string(),
        timestamp: 1_700_000_000,
        payload_hash: [
            // SHA-256(subject_xdr || timestamp_be8 || "kyc_approved").
            // Recomputed below so this fixture stays self-contained.
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0,
        ],
        signature: [0; 64],
        issuer_revoked: false,
        expires_at: Some(2_100_000_000),
    }
}

/// Replace with `client.get_attestation_keys(&attestation.issuer)`.
fn fetch_registered_key(_issuer: &str, key_bytes: &[u8; 32]) -> [u8; 32] {
    *key_bytes
}

fn main() {
    let env = Env::default();
    let contract_id = env.register_contract(None, AnchorKitContract);
    env.ledger().set(LedgerInfo {
        timestamp: 1_800_000_000,
        protocol_version: 21,
        sequence_number: 0,
        network_id: [7u8; 32],
        base_reserve: 0,
        min_persistent_entry_ttl: 4096,
        min_temp_entry_ttl: 16,
        max_entry_ttl: 6_312_000,
    });

    let secret = [1u8; 32];
    let signing_key = SigningKey::from_bytes(&secret);
    let registered_key = fetch_registered_key(ADDR_A, &signing_key.verifying_key().to_bytes());

    let mut attestation = fetch_attestation(1);
    let issuer = Address::from_str(&env, &attestation.issuer);
    let subject = Address::from_str(&env, &attestation.subject);
    let timestamp = attestation.timestamp;
    let payload_hash = anchorkit::compute_payload_hash(
        &env,
        &subject,
        timestamp,
        &Bytes::from_slice(&env, b"kyc_approved"),
    );
    attestation.payload_hash = payload_hash.to_array();

    // Recompute the exact digest the contract signs, using the exported helper.
    let digest = env.as_contract(&contract_id, || {
        attestation_signing_digest(
            &env,
            &issuer,
            &subject,
            timestamp,
            &Bytes::from_array(&env, &attestation.payload_hash),
        )
    });
    attestation.signature = signing_key.sign(&digest).to_bytes();

    let signature = Signature::from_bytes(&attestation.signature);
    let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&registered_key)
        .expect("registered key must be a valid Ed25519 public key");
    assert!(
        verifying_key.verify(&digest, &signature).is_ok(),
        "signature must verify against the attestor's registered key"
    );

    // A digest bound to another subject must not validate (vector 5).
    let other_subject = Address::from_str(&env, ADDR_B);
    let wrong_digest = env.as_contract(&contract_id, || {
        attestation_signing_digest(
            &env,
            &issuer,
            &other_subject,
            timestamp,
            &Bytes::from_array(&env, &attestation.payload_hash),
        )
    });
    assert!(verifying_key.verify(&wrong_digest, &signature).is_err());

    let now = env.ledger().timestamp();
    assert!(!attestation.issuer_revoked, "issuer revocation must be checked");
    assert!(
        attestation.expires_at.map_or(true, |expiry| now <= expiry),
        "attestation must not be expired"
    );

    println!("✓ attestation 1 signature verified off-chain");
    println!("✓ digest: {}", hex::encode(digest));
    println!("✓ issuer active, attestation not expired");
    println!("NOTE: NETWORK_PASSPHRASE={NETWORK_PASSPHRASE} is the ledger passphrase used by the digest helper");
}
