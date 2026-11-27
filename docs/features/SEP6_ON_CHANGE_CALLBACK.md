# SEP-6 `on_change_callback` Support

## Overview

SEP-6 deposit and withdraw requests accept an optional `on_change_callback`
parameter.  When present, the anchor POSTs status-change notifications to that
URL every time the transaction moves to a new state.  AnchorKit now surfaces
this parameter on both the raw input structs and the normalized response types.

## Fields added

| Struct | Field | Type |
|---|---|---|
| `RawDepositResponse` | `on_change_callback` | `Option<String>` |
| `RawWithdrawalResponse` | `on_change_callback` | `Option<String>` |
| `DepositResponse` | `on_change_callback` | `Option<String>` |
| `WithdrawalResponse` | `on_change_callback` | `Option<String>` |

## Validation

`on_change_callback` is validated by the existing
`domain_validator::validate_anchor_domain` function, which enforces:

- Must start with `https://` (HTTP is rejected).
- Must have a valid, multi-label hostname (no `localhost`, no bare IPs).
- Maximum URL length of 2 048 characters.
- No control characters or path-traversal sequences.

If the value fails validation, `initiate_deposit` / `initiate_withdrawal`
returns `Err` with `ErrorCode::ValidationError` and a message of
`"on_change_callback must be a valid HTTPS URL"`.

Passing `None` (or omitting the field entirely) is always valid — the field is
optional by design so existing callers require no changes.

## Usage

```rust
use anchorkit::sep6::{initiate_deposit, RawDepositResponse};

let raw = RawDepositResponse {
    transaction_id: "txn-001".to_string(),
    how: "Send to bank account 1234".to_string(),
    extra_info: None,
    min_amount: Some(10),
    max_amount: Some(10_000),
    fee_fixed: Some(1),
    fee_percent: None,
    status: Some("incomplete".to_string()),
    depositor_account: None,
    claimable_balance_supported: false,
    // Supply your HTTPS endpoint; None means no callback.
    on_change_callback: Some("https://my-service.example.com/sep6/notify".to_string()),
};

let deposit = initiate_deposit(raw, "USDC")?;

// The validated URL is available on the normalized response.
if let Some(url) = &deposit.on_change_callback {
    println!("Anchor will POST status updates to: {url}");
}
```

The same pattern applies to `initiate_withdrawal` / `RawWithdrawalResponse`.

## Verifying the callback signature

Anchors sign their callback POSTs so receivers can confirm authenticity.  The
standard signing scheme (described in [SEP-6 §8][sep6-spec]) works as follows:

1. **Retrieve the anchor's signing key** — fetch `stellar.toml` from the
   anchor's home domain and read the `SIGNING_KEY` field (a Stellar G-address).

2. **Reconstruct the signed payload** — the anchor signs a canonicalized string
   of the form `<timestamp>.<body-hash>` where:
   - `<timestamp>` is the value of the `X-Stellar-Signature-Timestamp` HTTP
     request header (Unix seconds as a decimal string).
   - `<body-hash>` is the lowercase hex-encoded SHA-256 digest of the raw
     request body.

3. **Verify the Ed25519 signature** — the `X-Stellar-Signature` header carries
   a base64url-encoded Ed25519 signature over the payload above, produced with
   the anchor's signing key.  Verify it with any Ed25519 library (e.g.
   `ed25519-dalek`):

   ```rust
   use ed25519_dalek::{Signature, VerifyingKey, Verifier};

   fn verify_callback(
       verifying_key: &VerifyingKey,
       timestamp_header: &str,  // X-Stellar-Signature-Timestamp
       body: &[u8],
       signature_header: &str,  // X-Stellar-Signature (base64url, no padding)
   ) -> bool {
       use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
       use sha2::{Digest, Sha256};

       let body_hash = hex::encode(Sha256::digest(body));
       let payload = format!("{}.{}", timestamp_header, body_hash);

       let Ok(sig_bytes) = URL_SAFE_NO_PAD.decode(signature_header) else {
           return false;
       };
       let Ok(signature) = Signature::from_slice(&sig_bytes) else {
           return false;
       };
       verifying_key.verify(payload.as_bytes(), &signature).is_ok()
   }
   ```

4. **Reject stale callbacks** — compare the timestamp to your server clock and
   reject any callback where `|now - timestamp| > 300` seconds to prevent
   replay attacks.

> **Note:** AnchorKit validates that the callback URL is a well-formed HTTPS
> endpoint before it is stored.  Signature verification happens at your HTTP
> server layer when the anchor actually delivers a POST; it is outside the scope
> of AnchorKit's request-builder layer.

[sep6-spec]: https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0006.md
