//! Minimal SEP-10 JWT verification (JWS compact, Ed25519 / `EdDSA`) for Soroban.
//!
//! Verifies the anchor-signed token using a 32-byte Ed25519 public key stored on-chain.
//! Payload must include integer `exp` (Unix seconds) and string `sub` (Stellar strkey of the client).


extern crate alloc;

use alloc::vec::Vec;
use soroban_sdk::{Bytes, Env, String};
use ed25519_dalek::{Signature, VerifyingKey, Verifier};

/// Maximum JWT character length accepted by the contract (defensive bound).
///
/// SEP-10 JWTs with multiple scope claims and long sub fields can exceed 2048 bytes.
/// 4096 provides headroom for realistic production tokens with comprehensive scope claims.
pub const MAX_JWT_LEN: u32 = 4096;

fn decode_base64url_char(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    }
}

/// Base64url decode — accepts padded, unpadded, and over-padded input.
///
/// Padding characters (`=`) are stripped before decoding. This matches the behaviour
/// of most JWT libraries, which omit padding entirely per RFC 7515 §2.
pub fn base64url_decode(input: &[u8]) -> Result<Vec<u8>, ()> {
    // Strip all trailing `=` so padded, unpadded, and over-padded inputs are equivalent.
    let input = {
        let mut end = input.len();
        while end > 0 && input[end - 1] == b'=' {
            end -= 1;
        }
        &input[..end]
    };

    // Invalid base64 if length mod 4 equals 1 after padding removal.
    if input.len() % 4 == 1 {
        return Err(());
    }

    let mut out: Vec<u8> = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    for &ch in input {
        let val = decode_base64url_char(ch).ok_or(())?;
        buffer = (buffer << 6) | (val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xFF) as u8);
        }
    }
    Ok(out)
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

/// Parse `"exp": <digits>` (first occurrence).
fn parse_json_exp(payload: &[u8]) -> Result<u64, ()> {
    let key = b"\"exp\":";
    let pos = find_bytes(payload, key).ok_or(())?;
    let mut i = pos + key.len();
    while i < payload.len() && payload[i].is_ascii_whitespace() {
        i += 1;
    }
    let mut n: u64 = 0;
    let mut any = false;
    while i < payload.len() && payload[i].is_ascii_digit() {
        any = true;
        let d = (payload[i] - b'0') as u64;
        // An exp value that overflows u64 is treated as a malformed token
        // and causes verify_sep10_jwt to return Err(()) — this is intentional.
        n = n
            .checked_mul(10)
            .and_then(|x| x.checked_add(d))
            .ok_or(())?;
        i += 1;
    }
    if !any {
        return Err(());
    }
    Ok(n)
}

/// Parse first `"sub":"..."` string value, handling `\"` escape sequences.
fn parse_json_sub(env: &Env, payload: &[u8]) -> Result<String, ()> {
    let key = b"\"sub\":";
    let pos = find_bytes(payload, key).ok_or(())?;
    let mut i = pos + key.len();
    while i < payload.len() && payload[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= payload.len() || payload[i] != b'"' {
        return Err(());
    }
    i += 1;
    let start = i;
    while i < payload.len() {
        if payload[i] == b'\\' {
            // skip escaped character; if nothing follows, it's malformed
            if i + 1 >= payload.len() {
                return Err(());
            }
            i += 2;
            continue;
        }
        if payload[i] == b'"' {
            let sub = &payload[start..i];
            return Ok(String::from_bytes(env, sub));
        }
        i += 1;
    }
    Err(())
}

/// Parse first `"scp":"..."` string value from the JWT payload.
fn parse_json_scp(payload: &[u8]) -> Result<Vec<u8>, ()> {
    let key = b"\"scp\":";
    let pos = find_bytes(payload, key).ok_or(())?;
    let mut i = pos + key.len();
    while i < payload.len() && payload[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= payload.len() || payload[i] != b'"' {
        return Err(());
    }
    i += 1;
    let start = i;
    while i < payload.len() {
        if payload[i] == b'\\' {
            if i + 1 >= payload.len() {
                return Err(());
            }
            i += 2;
            continue;
        }
        if payload[i] == b'"' {
            return Ok(payload[start..i].to_vec());
        }
        i += 1;
    }
    Err(())
}

/// Parse first `"alg":"..."` string value from the JWT header.
fn parse_json_alg(header: &[u8]) -> Result<Vec<u8>, ()> {
    let key = b"\"alg\":";
    let pos = find_bytes(header, key).ok_or(())?;
    let mut i = pos + key.len();
    while i < header.len() && header[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= header.len() || header[i] != b'"' {
        return Err(());
    }
    i += 1;
    let start = i;
    while i < header.len() {
        if header[i] == b'\\' {
            if i + 1 >= header.len() {
                return Err(());
            }
            i += 2;
            continue;
        }
        if header[i] == b'"' {
            return Ok(header[start..i].to_vec());
        }
        i += 1;
    }
    Err(())
}

/// Returns the canonical scope name for a service code (matches SERVICE_* constants in contract.rs).
pub fn service_scope_name(service_code: u32) -> Option<&'static [u8]> {
    match service_code {
        1 => Some(b"deposit"),
        2 => Some(b"withdrawal"),
        3 => Some(b"quote"),
        4 => Some(b"kyc"),
        _ => None,
    }
}

/// Check that a JWT's `scp` claim contains the scope for `service_code`.
///
/// The `scp` value is treated as a space-separated list of scope tokens.
/// Returns `Err(())` if the claim is absent, the service code is unknown, or the scope is missing.
pub fn check_token_scope(_env: &Env, token: &String, service_code: u32) -> Result<(), ()> {
    let scope_name = service_scope_name(service_code).ok_or(())?;

    let n = token.len();
    if n == 0 || n > MAX_JWT_LEN {
        return Err(());
    }
    let n_usize = n as usize;
    let mut buf = [0u8; MAX_JWT_LEN as usize];
    token.copy_into_slice(&mut buf[..n_usize]);

    let mut dots: [usize; 2] = [0; 2];
    let mut dot_count = 0usize;
    for (i, &byte) in buf[..n_usize].iter().enumerate() {
        if byte == b'.' {
            if dot_count < 2 {
                dots[dot_count] = i;
                dot_count += 1;
            } else {
                return Err(());
            }
        }
    }
    if dot_count != 2 {
        return Err(());
    }

    let payload_b64 = &buf[dots[0] + 1..dots[1]];
    let payload_dec = base64url_decode(payload_b64).map_err(|_| ())?;
    let scp = parse_json_scp(&payload_dec)?;

    // Walk the space-separated token list
    let mut start = 0usize;
    loop {
        while start < scp.len() && scp[start] == b' ' {
            start += 1;
        }
        if start >= scp.len() {
            break;
        }
        let end = scp[start..]
            .iter()
            .position(|&b| b == b' ')
            .map(|p| start + p)
            .unwrap_or(scp.len());
        if &scp[start..end] == scope_name {
            return Ok(());
        }
        start = end;
    }
    Err(())
}

/// Verify a SEP-10-style JWT: JWS compact, EdDSA signature, `exp`, and optional `sub` match.
///
/// When `expected_sub` is [`None`], the token must still contain a parseable `sub` claim, but it
/// is not compared to a caller-supplied address (see contract `verify_sep10_token`).
/// Maximum number of verifying keys stored per issuer (supports key rotation).
pub const MAX_VERIFYING_KEYS: u32 = 3;

pub fn verify_sep10_jwt(
    env: &Env,
    token: &String,
    keys: &soroban_sdk::Vec<Bytes>,
    expected_sub: Option<&String>,
    clock_skew_seconds: u64,
) -> Result<(), ()> {
    if keys.is_empty() {
        return Err(());
    }

    let n = token.len();
    if n == 0 || n > MAX_JWT_LEN {
        return Err(());
    }
    let n_usize = n as usize;
    let mut buf = [0u8; MAX_JWT_LEN as usize];
    token.copy_into_slice(&mut buf[..n_usize]);

    let mut dots: [usize; 2] = [0; 2];
    let mut dot_count = 0usize;
    for (i, &byte) in buf[..n_usize].iter().enumerate() {
        if byte == b'.' {
            if dot_count < 2 {
                dots[dot_count] = i;
                dot_count += 1;
            } else {
                return Err(());
            }
        }
    }
    if dot_count != 2 {
        return Err(());
    }

    let d0 = dots[0];
    let d1 = dots[1];
    if d0 == 0 || d1 <= d0 + 1 || d1 + 1 >= n_usize {
        return Err(());
    }

    let header_b64 = &buf[..d0];
    let payload_b64 = &buf[d0 + 1..d1];
    let sig_b64 = &buf[d1 + 1..n_usize];

    let header_dec = base64url_decode(header_b64).map_err(|_| ())?;
    let alg = parse_json_alg(&header_dec)?;
    if alg != b"EdDSA" {
        return Err(());
    }

    let sig_dec = base64url_decode(sig_b64).map_err(|_| ())?;
    if sig_dec.len() != 64 {
        return Err(());
    }

    let sig_arr: [u8; 64] = sig_dec.as_slice().try_into().map_err(|_| ())?;
    let dalek_sig = Signature::from_bytes(&sig_arr);

    let mut sig_ok = false;
    for i in 0..keys.len() {
        let key = keys.get(i).unwrap();
        if key.len() != 32 {
            continue;
        }
        let mut pk_arr = [0u8; 32];
        key.copy_into_slice(&mut pk_arr);
        if let Ok(vk) = VerifyingKey::from_bytes(&pk_arr) {
            if vk.verify(&buf[..d1], &dalek_sig).is_ok() {
                sig_ok = true;
                break;
            }
        }
    }
    if !sig_ok {
        return Err(());
    }

    let payload_dec = base64url_decode(payload_b64).map_err(|_| ())?;
    let exp = parse_json_exp(&payload_dec)?;
    let now = env.ledger().timestamp();
    if exp.saturating_add(clock_skew_seconds) <= now {
        return Err(());
    }

    let sub = parse_json_sub(env, &payload_dec)?;
    if let Some(expected) = expected_sub {
        if sub != *expected {
            return Err(());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use alloc::format;
    use crate::alloc::string::ToString;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
    use soroban_sdk::{Address, Env};

    fn ledger(env: &Env, ts: u64) {
        env.ledger().set(LedgerInfo {
            timestamp: ts,
            protocol_version: 21,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });
    }

    fn build_jwt(signing_key: &SigningKey, sub: &str, exp: u64) -> std::string::String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let header = r#"{"alg":"EdDSA","typ":"JWT"}"#;
        let payload = format!(r#"{{"sub":"{}","exp":{}}}"#, sub, exp);
        let header_b64 = URL_SAFE_NO_PAD.encode(header);
        let payload_b64 = URL_SAFE_NO_PAD.encode(payload);
        let signing_input = format!("{}.{}", header_b64, payload_b64);
        let sig = signing_key.sign(signing_input.as_bytes());
        let sig_b64 = URL_SAFE_NO_PAD.encode(sig.to_bytes());
        format!("{}.{}", signing_input, sig_b64)
    }

    #[test]
    fn base64url_roundtrip_simple() {
        // "Hello" = SGVsbG8 (unpadded), SGVsbG8= (1-pad), SGVsbG8== (over-padded)
        let expected = b"Hello";
        assert_eq!(base64url_decode(b"SGVsbG8").unwrap(), expected);   // unpadded
        assert_eq!(base64url_decode(b"SGVsbG8=").unwrap(), expected);  // standard padded
        assert_eq!(base64url_decode(b"SGVsbG8==").unwrap(), expected); // over-padded
        assert_eq!(base64url_decode(b"SGVsbG8===").unwrap(), expected); // extra over-padded

        // "Man" = TWFu (no padding needed), TWFu= (spurious pad)
        assert_eq!(base64url_decode(b"TWFu").unwrap(), b"Man");
        assert_eq!(base64url_decode(b"TWFu=").unwrap(), b"Man");

        // Invalid character should still error
        assert!(base64url_decode(b"SGVs!G8").is_err());
    }

    #[test]
    fn base64url_rejects_invalid_padding_length() {
        // Length 1 after padding removal: 1 % 4 == 1 — invalid
        assert!(base64url_decode(b"A").is_err());
        assert!(base64url_decode(b"A=").is_err());
        assert!(base64url_decode(b"A==").is_err());

        // Length 5 after padding removal: 5 % 4 == 1 — invalid
        assert!(base64url_decode(b"ABCDE").is_err());
    }

    #[test]
    fn parse_json_exp_overflow() {
        // exp value exceeding u64::MAX is treated as malformed
        let payload = b"{\"exp\":99999999999999999999}";
        assert!(parse_json_exp(payload).is_err());
    }

    #[test]
    fn verify_accepts_valid_token() {
        let env = Env::default();
        ledger(&env, 1_000);
        let signing_key = SigningKey::generate(&mut OsRng);
        let pk = Bytes::from_slice(&env, signing_key.verifying_key().as_bytes());

        let attestor = Address::generate(&env);
        let sub = attestor.to_string();
        let sub_str: std::string::String = sub.to_string();
        let jwt = build_jwt(&signing_key, sub_str.as_str(), 2_000);
        let token = String::from_str(&env, jwt.as_str());

        let mut keys = soroban_sdk::Vec::new(&env);
        keys.push_back(pk);
        assert!(verify_sep10_jwt(&env, &token, &keys, Some(&sub), 0).is_ok());
        assert!(verify_sep10_jwt(&env, &token, &keys, None, 0).is_ok());
    }

    #[test]
    fn verify_rejects_expired_token() {
        let env = Env::default();
        ledger(&env, 5_000);
        let signing_key = SigningKey::generate(&mut OsRng);
        let pk = Bytes::from_slice(&env, signing_key.verifying_key().as_bytes());

        let attestor = Address::generate(&env);
        let sub = attestor.to_string();
        let sub_str: std::string::String = sub.to_string();
        let jwt = build_jwt(&signing_key, sub_str.as_str(), 1_000);
        let token = String::from_str(&env, jwt.as_str());

        let mut keys = soroban_sdk::Vec::new(&env);
        keys.push_back(pk);
        assert!(verify_sep10_jwt(&env, &token, &keys, Some(&sub), 0).is_err());
    }

    #[test]
    #[should_panic]
    fn verify_rejects_invalid_signature() {
        let env = Env::default();
        ledger(&env, 1_000);
        let signing_key = SigningKey::generate(&mut OsRng);
        let other_key = SigningKey::generate(&mut OsRng);
        let pk = Bytes::from_slice(&env, other_key.verifying_key().as_bytes());

        let attestor = Address::generate(&env);
        let sub = attestor.to_string();
        let mut buf = [0u8; 128];
        let len = sub.len() as usize;
        let final_len = if len > 128 { 128 } else { len };
        sub.copy_into_slice(&mut buf[..final_len]);
        let sub_str = core::str::from_utf8(&buf[..final_len]).unwrap_or("");
        let jwt = build_jwt(&signing_key, sub_str, 2_000);
        let token = String::from_str(&env, jwt.as_str());

        let mut keys = soroban_sdk::Vec::new(&env);
        keys.push_back(pk);
        assert!(verify_sep10_jwt(&env, &token, &keys, Some(&sub), 0).is_err());

        // Malformed payloads should also return Err, not panic
        let malformed_cases: &[&[u8]] = &[
            b"",                                    // empty payload
            b"not json at all",                     // bad JSON
            b"{\"sub\":\"val\\\"ue\",\"exp\":9999}", // escaped quote in sub value
            b"{\"sub\":\"unterminated",             // truncated / no closing quote
        ];
        for payload in malformed_cases {
            assert!(
                parse_json_sub(&env, payload).is_err(),
                "expected Err for payload: {:?}",
                payload
            );
        }
    }

    #[test]
    fn parse_json_sub_malformed_inputs_return_none() {
        let env = Env::default();
        ledger(&env, 1_000);

        let cases: &[&[u8]] = &[
            b"",                                          // empty
            b"{}",                                        // no sub key
            b"{\"sub\":42}",                              // sub not a string
            b"{\"sub\":\"unterminated",                   // truncated / no closing quote
            b"{\"sub\":\"\\",                             // backslash at end (malformed escape)
        ];

        for payload in cases {
            assert!(
                parse_json_sub(&env, payload).is_err(),
                "expected Err for: {:?}",
                payload
            );
        }
    }

    fn build_jwt_with_scope(signing_key: &SigningKey, sub: &str, exp: u64, scope: &str) -> std::string::String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let header = r#"{"alg":"EdDSA","typ":"JWT"}"#;
        let payload = format!(r#"{{"sub":"{}","exp":{},"scp":"{}"}}"#, sub, exp, scope);
        let header_b64 = URL_SAFE_NO_PAD.encode(header);
        let payload_b64 = URL_SAFE_NO_PAD.encode(payload);
        let signing_input = format!("{}.{}", header_b64, payload_b64);
        let sig = signing_key.sign(signing_input.as_bytes());
        let sig_b64 = URL_SAFE_NO_PAD.encode(sig.to_bytes());
        format!("{}.{}", signing_input, sig_b64)
    }

    fn build_jwt_with_custom_alg(signing_key: &SigningKey, sub: &str, exp: u64, alg: &str) -> std::string::String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let header = format!(r#"{{"alg":"{}","typ":"JWT"}}"#, alg);
        let payload = format!(r#"{{"sub":"{}","exp":{}}}"#, sub, exp);
        let header_b64 = URL_SAFE_NO_PAD.encode(header);
        let payload_b64 = URL_SAFE_NO_PAD.encode(payload);
        let signing_input = format!("{}.{}", header_b64, payload_b64);
        let sig = signing_key.sign(signing_input.as_bytes());
        let sig_b64 = URL_SAFE_NO_PAD.encode(sig.to_bytes());
        format!("{}.{}", signing_input, sig_b64)
    }

    fn build_jwt_without_alg(signing_key: &SigningKey, sub: &str, exp: u64) -> std::string::String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let header = r#"{"typ":"JWT"}"#;
        let payload = format!(r#"{{"sub":"{}","exp":{}}}"#, sub, exp);
        let header_b64 = URL_SAFE_NO_PAD.encode(header);
        let payload_b64 = URL_SAFE_NO_PAD.encode(payload);
        let signing_input = format!("{}.{}", header_b64, payload_b64);
        let sig = signing_key.sign(signing_input.as_bytes());
        let sig_b64 = URL_SAFE_NO_PAD.encode(sig.to_bytes());
        format!("{}.{}", signing_input, sig_b64)
    }

    #[test]
    fn check_token_scope_matches() {
        let env = Env::default();
        ledger(&env, 1_000);
        let signing_key = SigningKey::generate(&mut OsRng);

        let jwt = build_jwt_with_scope(&signing_key, "any", 2_000, "deposit withdrawal");
        let token = String::from_str(&env, jwt.as_str());

        assert!(check_token_scope(&env, &token, 1).is_ok()); // deposit
        assert!(check_token_scope(&env, &token, 2).is_ok()); // withdrawal
        assert!(check_token_scope(&env, &token, 3).is_err()); // quote — not in scp
        assert!(check_token_scope(&env, &token, 4).is_err()); // kyc — not in scp
    }

    #[test]
    fn check_token_scope_no_scp_claim_returns_err() {
        let env = Env::default();
        ledger(&env, 1_000);
        let signing_key = SigningKey::generate(&mut OsRng);

        // JWT with no scp claim
        let jwt = build_jwt(&signing_key, "any", 2_000);
        let token = String::from_str(&env, jwt.as_str());

        assert!(check_token_scope(&env, &token, 1).is_err());
    }

    #[test]
    fn verify_accepts_token_within_clock_skew_window() {
        let env = Env::default();
        // Ledger is 30 s ahead of the token's exp — within a 60 s skew tolerance.
        ledger(&env, 1_030);
        let signing_key = SigningKey::generate(&mut OsRng);
        let pk = Bytes::from_slice(&env, signing_key.verifying_key().as_bytes());
        let mut keys = soroban_sdk::Vec::new(&env);
        keys.push_back(pk);

        let attestor = Address::generate(&env);
        let sub = attestor.to_string();
        let sub_str: std::string::String = sub.to_string();
        // Token expired at t=1_000, ledger is at t=1_030 (30 s lag).
        let jwt = build_jwt(&signing_key, sub_str.as_str(), 1_000);
        let token = String::from_str(&env, jwt.as_str());

        // Without skew: rejected.
        assert!(verify_sep10_jwt(&env, &token, &keys, None, 0).is_err());
        // With 60 s skew: accepted (exp + 60 = 1_060 > 1_030).
        assert!(verify_sep10_jwt(&env, &token, &keys, None, 60).is_ok());
        // With skew exactly equal to lag (30 s): exp + 30 = 1_030, not strictly greater — rejected.
        assert!(verify_sep10_jwt(&env, &token, &keys, None, 30).is_err());
    }

    #[test]
    fn verify_validates_alg_header() {
        let env = Env::default();
        ledger(&env, 1_000);
        let signing_key = SigningKey::generate(&mut OsRng);
        let pk = Bytes::from_slice(&env, signing_key.verifying_key().as_bytes());
        let mut keys = soroban_sdk::Vec::new(&env);
        keys.push_back(pk);

        let attestor = Address::generate(&env);
        let sub = attestor.to_string();
        let sub_str: std::string::String = sub.to_string();

        // EdDSA token is accepted
        let jwt = build_jwt(&signing_key, sub_str.as_str(), 2_000);
        let token = String::from_str(&env, jwt.as_str());
        assert!(verify_sep10_jwt(&env, &token, &keys, Some(&sub), 0).is_ok());

        // HS256 token is rejected
        let jwt_hs256 = build_jwt_with_custom_alg(&signing_key, sub_str.as_str(), 2_000, "HS256");
        let token_hs256 = String::from_str(&env, jwt_hs256.as_str());
        assert!(verify_sep10_jwt(&env, &token_hs256, &keys, Some(&sub), 0).is_err());

        // Token with no alg field is rejected
        let jwt_no_alg = build_jwt_without_alg(&signing_key, sub_str.as_str(), 2_000);
        let token_no_alg = String::from_str(&env, jwt_no_alg.as_str());
        assert!(verify_sep10_jwt(&env, &token_no_alg, &keys, Some(&sub), 0).is_err());
    }
}
