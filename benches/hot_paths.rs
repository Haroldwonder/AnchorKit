//! Criterion benchmark suite for AnchorKit contract hot paths (issue #738).
//!
//! Establishes performance baselines for the three most performance-sensitive
//! contract entry points so regressions can be detected between releases:
//!
//!   * `submit_attestation`     — auth + rate limit + ed25519 verify + storage write
//!   * `get_anchor_health_score` — cached-metadata read + weighted scoring
//!   * `route_transaction`       — anchor filtering + quote selection
//!
//! Benchmarks drive the contract through the Soroban test harness
//! (`soroban-sdk` `testutils`) as an external consumer of the `anchorkit`
//! crate, the same way the in-crate `#[cfg(test)]` suites do. The SEP-10
//! attestor-registration and payload-signing helpers are reproduced here
//! because their in-crate counterparts (`src/sep10_test_util.rs`) are gated
//! behind `#[cfg(test)]` and are therefore invisible to `benches/`.
//!
//! Run with: `cargo bench`

use anchorkit::{
    AnchorKitContract, AnchorKitContractClient, AnchorMetadata, RoutingOptions, RoutingRequest,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Bytes, Env, String as SorobanString, Symbol, Vec as SorobanVec,
};

// ---------------------------------------------------------------------------
// Test-harness helpers
// ---------------------------------------------------------------------------

/// Pin the ledger to a fixed timestamp/sequence so benchmarks are deterministic
/// and metadata/quote TTL windows stay valid for the duration of the run.
fn set_ledger(env: &Env, timestamp: u64) {
    env.ledger().set(LedgerInfo {
        timestamp,
        protocol_version: 21,
        sequence_number: 0,
        network_id: Default::default(),
        base_reserve: 0,
        min_persistent_entry_ttl: 4096,
        min_temp_entry_ttl: 16,
        max_entry_ttl: 6_312_000,
    });
}

fn make_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    set_ledger(&env, 1_000_000);
    env
}

fn setup_contract(env: &Env) -> AnchorKitContractClient {
    let contract_id = env.register_contract(None, AnchorKitContract);
    let client = AnchorKitContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin, &100_u64, &None);
    client
}

/// Build a SEP-10 JWT signed by `signing_key` (mirrors `sep10_test_util`).
fn build_sep10_jwt(signing_key: &SigningKey, sub: &str, exp: u64) -> std::string::String {
    let header = r#"{"alg":"EdDSA","typ":"JWT"}"#;
    let payload = format!(r#"{{"sub":"{}","exp":{}}}"#, sub, exp);
    let header_b64 = URL_SAFE_NO_PAD.encode(header);
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload);
    let signing_input = format!("{}.{}", header_b64, payload_b64);
    let sig = signing_key.sign(signing_input.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig.to_bytes());
    format!("{}.{}", signing_input, sig_b64)
}

/// Register `attestor` as a SEP-10-authenticated attestor with its own verifying key.
fn register_attestor(
    env: &Env,
    client: &AnchorKitContractClient,
    attestor: &Address,
    signing_key: &SigningKey,
) {
    let pk = Bytes::from_slice(env, signing_key.verifying_key().as_bytes());
    client.set_sep10_jwt_verifying_key(attestor, &pk);
    client.add_attestation_key(attestor, &pk);

    let sub = attestor.to_string();
    let mut buf = [0u8; 128];
    let len = (sub.len() as usize).min(128);
    sub.copy_into_slice(&mut buf[..len]);
    let sub_str = core::str::from_utf8(&buf[..len]).unwrap_or("");

    let exp = env.ledger().timestamp().saturating_add(86_400);
    let jwt = build_sep10_jwt(signing_key, sub_str, exp);
    let token = SorobanString::from_str(env, jwt.as_str());
    client.register_attestor(attestor, &token, attestor);
}

/// Sign an attestation (contract, issuer, subject, timestamp, payload hash), returning a 64-byte ed25519 signature as `Bytes`.
fn sign_attestation(
    env: &Env,
    signing_key: &SigningKey,
    contract_id: &Address,
    issuer: &Address,
    subject: &Address,
    timestamp: u64,
    payload_hash: &Bytes,
) -> Bytes {
    use soroban_sdk::xdr::ToXdr;

    // Mirrors the contract's `attestation_signing_digest`; see docs/test_vectors.md.
    let digest = env.as_contract(contract_id, || {
        let mut msg = Bytes::from_slice(env, b"anchorkit-attest-v1");
        msg.append(&Bytes::from_array(env, &env.ledger().network_id().to_array()));
        msg.append(&env.current_contract_address().to_xdr(env));
        msg.append(&issuer.clone().to_xdr(env));
        msg.append(&subject.clone().to_xdr(env));
        msg.append(&Bytes::from_array(env, &timestamp.to_be_bytes()));
        msg.append(payload_hash);
        env.crypto().sha256(&msg).to_array()
    });
    let sig = signing_key.sign(&digest);
    Bytes::from_slice(env, &sig.to_bytes())
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

/// Baseline for the attestation submission path. A fresh payload hash is
/// generated per iteration so replay protection never rejects a submission;
/// rate limiting is left unconfigured (opt-in), so it is a no-op here.
fn bench_submit_attestation(c: &mut Criterion) {
    let env = make_env();
    let client = setup_contract(&env);

    let attestor = Address::generate(&env);
    let signing_key = SigningKey::generate(&mut OsRng);
    register_attestor(&env, &client, &attestor, &signing_key);

    let subject = Address::generate(&env);
    let timestamp = env.ledger().timestamp();

    c.bench_function("submit_attestation", |b| {
        let mut counter: u64 = 0;
        b.iter_batched(
            || {
                // Unique payload hash per iteration -> no ReplayAttack panic.
                counter += 1;
                let mut hash = [0u8; 32];
                hash[..8].copy_from_slice(&counter.to_be_bytes());
                let payload_hash = Bytes::from_slice(&env, &hash);
                let signature = sign_attestation(&env, &signing_key, &client.address, &attestor, &subject, timestamp, &payload_hash);
                (payload_hash, signature)
            },
            |(payload_hash, signature)| {
                client.submit_attestation(
                    &attestor,
                    &subject,
                    &timestamp,
                    &payload_hash,
                    &signature,
                    &None,
                );
            },
            BatchSize::SmallInput,
        );
    });
}

/// Baseline for the health-score read path: a cached-metadata lookup followed
/// by the weighted scoring arithmetic.
fn bench_get_anchor_health_score(c: &mut Criterion) {
    let env = make_env();
    let client = setup_contract(&env);

    let anchor = Address::generate(&env);
    let metadata = AnchorMetadata {
        anchor: anchor.clone(),
        reputation_score: 9500,
        liquidity_score: 5000,
        uptime_percentage: 9800,
        total_volume: 1_000_000,
        average_settlement_time: 480,
        is_active: true,
        homepage_url: None,
    };
    client.cache_metadata(&anchor, &metadata, &3600);

    c.bench_function("get_anchor_health_score", |b| {
        b.iter(|| client.get_anchor_health_score(&anchor));
    });
}

/// Baseline for the routing path, parameterized by the number of registered
/// anchors so the cost of candidate filtering/selection can be tracked as the
/// anchor set grows.
fn bench_route_transaction(c: &mut Criterion) {
    let mut group = c.benchmark_group("route_transaction");

    for n in [1usize, 5, 10, 25] {
        let env = make_env();
        let client = setup_contract(&env);

        for i in 0..n {
            let anchor = Address::generate(&env);
            let signing_key = SigningKey::generate(&mut OsRng);
            register_attestor(&env, &client, &anchor, &signing_key);

            let mut services = SorobanVec::new(&env);
            services.push_back(1u32); // deposit
            services.push_back(3u32); // KYC
            client.configure_services(&anchor, &services);

            // Vary settlement time and fee per anchor so the selection strategy
            // does real comparison work rather than trivially picking the first.
            client.set_anchor_metadata(
                &anchor,
                &8000u32,
                &(300 + i as u64),
                &7500u32,
                &9900u32,
                &1_000_000u64,
                &None::<SorobanString>,
            );
            client.submit_quote(
                &anchor,
                &SorobanString::from_str(&env, "USD"),
                &SorobanString::from_str(&env, "USDC"),
                &10_000u64,
                &(20 + i as u32),
                &100u64,
                &100_000u64,
                &1_003_600u64,
            );
        }

        let mut strategy = SorobanVec::new(&env);
        strategy.push_back(Symbol::new(&env, "LowestFee"));
        let options = RoutingOptions {
            request: RoutingRequest {
                base_asset: SorobanString::from_str(&env, "USD"),
                quote_asset: SorobanString::from_str(&env, "USDC"),
                amount: 5_000,
                operation_type: 1,
            },
            strategy,
            min_reputation: 0,
            max_anchors: n as u32,
            require_kyc: false,
            jurisdiction: None,
            fallback_chain: SorobanVec::new(&env),
        };

        group.bench_with_input(BenchmarkId::from_parameter(n), &options, |b, opts| {
            b.iter(|| client.route_transaction(opts));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_submit_attestation,
    bench_get_anchor_health_score,
    bench_route_transaction
);
criterion_main!(benches);
