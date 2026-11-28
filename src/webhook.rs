//! Signature verification for SEP-6/24/31 anchor callbacks (#1573).
//!
//! Anchors sign callbacks with their stellar.toml `SIGNING_KEY`. The
//! `Signature` header is a base64-encoded ed25519 signature over the
//! concatenation of the decimal `Timestamp` header (unix seconds) and the raw
//! request body. Callbacks older than `max_age` must be rejected.

extern crate std;

use alloc::format;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::sep6::signing_key_bytes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackError {
    MissingHeader,
    InvalidTimestamp,
    Expired,
    InvalidSigningKey,
    InvalidSignature,
}

impl core::fmt::Display for CallbackError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::MissingHeader => "missing Signature or Timestamp header",
            Self::InvalidTimestamp => "invalid Timestamp header",
            Self::Expired => "callback timestamp is outside the allowed age",
            Self::InvalidSigningKey => "invalid stellar.toml SIGNING_KEY",
            Self::InvalidSignature => "callback signature does not match signing key",
        };
        f.write_str(message)
    }
}

impl std::error::Error for CallbackError {}

fn header<'a>(headers: &'a [(&'a str, &'a str)], name: &str) -> Option<&'a str> {
    headers.iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| *value)
}

/// Verify a callback using the current wall clock as "now".
pub fn verify_anchor_callback(
    headers: &[(&str, &str)],
    body: &[u8],
    signing_key: &str,
    max_age: u64,
) -> Result<(), CallbackError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    verify_anchor_callback_at(headers, body, signing_key, max_age, now)
}

/// Same verification with an explicit `now`, for deterministic tests/replays.
pub fn verify_anchor_callback_at(
    headers: &[(&str, &str)],
    body: &[u8],
    signing_key: &str,
    max_age: u64,
    now: u64,
) -> Result<(), CallbackError> {
    let timestamp = header(headers, "Timestamp").ok_or(CallbackError::MissingHeader)?;
    let signature = header(headers, "Signature").ok_or(CallbackError::MissingHeader)?;
    let timestamp: u64 = timestamp.parse().map_err(|_| CallbackError::InvalidTimestamp)?;
    let age = now.checked_sub(timestamp).ok_or(CallbackError::Expired)?;
    if age > max_age {
        return Err(CallbackError::Expired);
    }

    let key_bytes = signing_key_bytes(signing_key).ok_or(CallbackError::InvalidSigningKey)?;
    let verifying_key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|_| CallbackError::InvalidSigningKey)?;
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature.trim())
        .map_err(|_| CallbackError::InvalidSignature)?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| CallbackError::InvalidSignature)?;

    let mut signed = format!("{}", timestamp).into_bytes();
    signed.extend_from_slice(body);
    verifying_key.verify(&signed, &signature)
        .map_err(|_| CallbackError::InvalidSignature)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/anchor_callback.json");

    #[test]
    fn verifies_fixture_callback() {
        let fixture: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
        let signing_key = fixture["signing_key"].as_str().unwrap();
        let body = fixture["body"].as_str().unwrap().as_bytes();
        let timestamp = fixture["timestamp"].as_str().unwrap();
        let signature = fixture["signature"].as_str().unwrap();
        let headers = [("signature", signature), ("timestamp", timestamp)];

        assert_eq!(verify_anchor_callback_at(&headers, body, signing_key, 300, 1_700_000_060), Ok(()));
        assert_eq!(
            verify_anchor_callback_at(&headers, body, signing_key, 60, 1_700_000_100),
            Err(CallbackError::Expired)
        );
        assert_eq!(
            verify_anchor_callback_at(&headers, b"tampered", signing_key, 300, 1_700_000_060),
            Err(CallbackError::InvalidSignature)
        );
        assert_eq!(
            verify_anchor_callback_at(&headers, body, "GCARGMU3VAUYHDCZEJSRPZPGRZFX764FYRWZSPWQCJ6TBQXZQCWXP44A", 300, 1_700_000_060),
            Err(CallbackError::InvalidSigningKey)
        );
        assert_eq!(
            verify_anchor_callback_at(&[("Timestamp", timestamp)], body, signing_key, 300, 1_700_000_060),
            Err(CallbackError::MissingHeader)
        );
    }
}
