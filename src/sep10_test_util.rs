#![cfg(test)]
extern crate alloc;
use alloc::format;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ed25519_dalek::{Signer, SigningKey};
use soroban_sdk::{Address, Bytes, Env, String};

use crate::contract::AnchorKitContractClient;

pub fn build_sep10_jwt(signing_key: &SigningKey, sub: &str, exp: u64) -> alloc::string::String {
    let header = r#"{"alg":"EdDSA","typ":"JWT"}"#;
    let payload = format!(r#"{{"sub":"{}","exp":{}}}"#, sub, exp);
    let header_b64 = URL_SAFE_NO_PAD.encode(header);
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload);
    let signing_input = format!("{}.{}", header_b64, payload_b64);
    let sig = signing_key.sign(signing_input.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig.to_bytes());
    format!("{}.{}", signing_input, sig_b64)
}

/// Build a SEP-10 JWT that includes an `scp` (scope) claim.
///
/// `scope` is a space-separated list of service names (e.g. `"deposit withdrawal"`).
pub fn build_sep10_jwt_with_scope(
    signing_key: &SigningKey,
    sub: &str,
    exp: u64,
    scope: &str,
) -> alloc::string::String {
    let header = r#"{"alg":"EdDSA","typ":"JWT"}"#;
    let payload = format!(r#"{{"sub":"{}","exp":{},"scp":"{}"}}"#, sub, exp, scope);
    let header_b64 = URL_SAFE_NO_PAD.encode(header);
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload);
    let signing_input = format!("{}.{}", header_b64, payload_b64);
    let sig = signing_key.sign(signing_input.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig.to_bytes());
    format!("{}.{}", signing_input, sig_b64)
}

/// Build a SEP-10 JWT that includes a `memo` claim.
///
/// `memo` is an optional string for customer/account identification.
pub fn build_sep10_jwt_with_memo(
    signing_key: &SigningKey,
    sub: &str,
    exp: u64,
    memo: Option<&str>,
) -> alloc::string::String {
    let header = r#"{"alg":"EdDSA","typ":"JWT"}"#;
    let payload = if let Some(m) = memo {
        format!(r#"{{"sub":"{}","exp":{},"memo":"{}"}}"#, sub, exp, m)
    } else {
        format!(r#"{{"sub":"{}","exp":{},"memo":null}}"#, sub, exp)
    };
    let header_b64 = URL_SAFE_NO_PAD.encode(header);
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload);
    let signing_input = format!("{}.{}", header_b64, payload_b64);
    let sig = signing_key.sign(signing_input.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig.to_bytes());
    format!("{}.{}", signing_input, sig_b64)
}

/// Build a SEP-10 JWT that includes a `client_domain` claim.
///
/// `client_domain` is an optional domain string for domain verification.
pub fn build_sep10_jwt_with_client_domain(
    signing_key: &SigningKey,
    sub: &str,
    exp: u64,
    client_domain: Option<&str>,
) -> alloc::string::String {
    let header = r#"{"alg":"EdDSA","typ":"JWT"}"#;
    let payload = if let Some(d) = client_domain {
        format!(r#"{{"sub":"{}","exp":{},"client_domain":"{}"}}"#, sub, exp, d)
    } else {
        format!(r#"{{"sub":"{}","exp":{},"client_domain":null}}"#, sub, exp)
    };
    let header_b64 = URL_SAFE_NO_PAD.encode(header);
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload);
    let signing_input = format!("{}.{}", header_b64, payload_b64);
    let sig = signing_key.sign(signing_input.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig.to_bytes());
    format!("{}.{}", signing_input, sig_b64)
}

/// Sign a payload hash with the given signing key, returning a 64-byte signature as Bytes.
pub fn sign_payload(env: &Env, signing_key: &SigningKey, payload_hash: &Bytes) -> Bytes {
    let mut hash_arr = [0u8; 32];
    payload_hash.copy_into_slice(&mut hash_arr);
    let sig = signing_key.sign(&hash_arr);
    Bytes::from_slice(env, &sig.to_bytes())
}
pub fn register_attestor_with_sep10(
    env: &Env,
    client: &AnchorKitContractClient,
    attestor: &Address,
    sep10_issuer: &Address,
    signing_key: &SigningKey,
) {
    let pk = Bytes::from_slice(env, signing_key.verifying_key().as_bytes());
    client.set_sep10_jwt_verifying_key(sep10_issuer, &pk);

    let sub = attestor.to_string();
    let mut buf = [0u8; 128];
    let len = sub.len() as usize;
    let final_len = if len > 128 { 128 } else { len };
    sub.copy_into_slice(&mut buf[..final_len]);
    let sub_str = core::str::from_utf8(&buf[..final_len]).unwrap_or("");
    
    let exp = env.ledger().timestamp().saturating_add(86_400);
    let jwt = build_sep10_jwt(signing_key, sub_str, exp);
    let token = String::from_str(env, jwt.as_str());
    client.register_attestor(attestor, &token, sep10_issuer);
}
