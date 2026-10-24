# SEP-24 Interactive Deposit/Withdrawal Guide

AnchorKit currently provides a complete SEP-6 non-interactive deposit/withdrawal layer. This guide explains how to extend AnchorKit to support **SEP-24 interactive flows**, which open a popup/iframe where users complete the anchor's own UI before the transaction is submitted on-chain.

---

## Table of Contents

1. [SEP-24 vs SEP-6: Key Differences](#sep-24-vs-sep-6-key-differences)
2. [Prerequisites](#prerequisites)
3. [SEP-24 Flow Overview](#sep-24-flow-overview)
4. [Step 1 – Authenticate with SEP-10](#step-1--authenticate-with-sep-10)
5. [Step 2 – Initiate the Interactive Session](#step-2--initiate-the-interactive-session)
6. [Step 3 – Open the Interactive Popup](#step-3--open-the-interactive-popup)
7. [Step 4 – Poll Transaction Status](#step-4--poll-transaction-status)
8. [Step 5 – Finalise On-Chain](#step-5--finalise-on-chain)
9. [Integrating with AnchorKit's On-Chain Contract](#integrating-with-anchorkits-on-chain-contract)
10. [Error Handling](#error-handling)
11. [Example: Full Deposit Flow](#example-full-deposit-flow)

---

## SEP-24 vs SEP-6: Key Differences

| Feature | SEP-6 (non-interactive) | SEP-24 (interactive) |
|---------|------------------------|----------------------|
| User experience | API calls only | Popup/iframe UI hosted by anchor |
| KYC collection | Caller-driven | Anchor-driven in the popup |
| Status polling | `/transaction` endpoint | `/transaction` endpoint (same) |
| JWT scope | `sep6:deposit` / `sep6:withdraw` | `sep24:deposit` / `sep24:withdraw` |
| Stellar endpoint | `transfer_server` | `transfer_server_sep0024` in stellar.toml |

Both protocols share the same SEP-10 authentication and the same on-chain attestation submission via AnchorKit.

---

## Prerequisites

Before implementing SEP-24 flows:

1. The anchor's stellar.toml must include a `TRANSFER_SERVER_SEP0024` field.
2. AnchorKit must have a cached toml entry for the anchor (`fetch_anchor_info`).
3. A valid SEP-10 JWT scoped for `sep24:deposit` or `sep24:withdraw` must be obtained.

---

## SEP-24 Flow Overview

```
Client                       Anchor SEP-24 Server              AnchorKit Contract
  |                                   |                                 |
  |-- POST /deposit or /withdraw ----->|                                 |
  |<-- { id, url } -------------------|                                 |
  |                                   |                                 |
  |-- open popup(url) ---------------->|                                 |
  |   (user fills KYC in popup)        |                                 |
  |<-- postMessage("completed") -------|                                 |
  |                                   |                                 |
  |-- GET /transaction?id=<id> ------->|                                 |
  |<-- { status: "pending_external" }--|                                 |
  |                                   |                                 |
  |-- (send Stellar payment) --------->|                                 |
  |                                   |                                 |
  |-- GET /transaction?id=<id> ------->|                                 |
  |<-- { status: "completed" } --------|                                 |
  |                                   |                                 |
  |-- submit_attestation() ------------------------------------------>|
  |<-- attestation_id <-----------------------------------------------|
```

---

## Step 1 – Authenticate with SEP-10

Obtain a SEP-10 JWT with the SEP-24 scope. This is off-chain but the verifying key must be registered in AnchorKit first.

```rust
// Register the anchor's SEP-10 verifying key (admin, one-time setup)
client.upsert_sep10_verifying_key(&issuer, &ed25519_public_key_bytes);
```

Obtain the JWT using your SEP-10 client library and then verify it on-chain before using it in further calls:

```rust
client.verify_sep10_token_for_service(&jwt_token, &issuer, &SERVICE_DEPOSITS);
```

---

## Step 2 – Initiate the Interactive Session

Call the anchor's `POST /deposit` (or `/withdraw`) endpoint with the scoped JWT. The anchor returns a transaction `id` and an interactive `url`.

```javascript
// JavaScript / off-chain client
const response = await fetch(`${sep24Server}/deposit`, {
  method: "POST",
  headers: {
    "Authorization": `Bearer ${jwtToken}`,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    asset_code: "USDC",
    account: userStellarAddress,
    amount: "100.00",
  }),
});
const { id: transactionId, url: interactiveUrl } = await response.json();
```

Retrieve the `transfer_server_sep0024` URL from the AnchorKit-cached stellar.toml:

```rust
let toml: StellarToml = client.get_anchor_toml(&anchor);
// toml.transfer_server_sep0024 contains the base URL
```

---

## Step 3 – Open the Interactive Popup

Open `interactiveUrl` in a popup or iframe. Listen for the `postMessage` completion signal defined by SEP-24:

```javascript
const popup = window.open(interactiveUrl, "sep24", "width=600,height=800");

window.addEventListener("message", (event) => {
  // Validate origin against the anchor domain
  if (event.origin !== anchorOrigin) return;

  if (event.data?.type === "sep24_done") {
    popup.close();
    pollTransactionStatus(transactionId);
  }
});
```

For mobile or non-browser clients, redirect the user to `interactiveUrl` and resume polling after the user returns to your app via a deep-link callback URL.

---

## Step 4 – Poll Transaction Status

Poll `GET /transaction?id=<transactionId>` until the status reaches `completed`, `error`, or a terminal state requiring user action.

```javascript
async function pollTransactionStatus(txId) {
  while (true) {
    const res = await fetch(`${sep24Server}/transaction?id=${txId}`, {
      headers: { "Authorization": `Bearer ${jwtToken}` },
    });
    const { transaction } = await res.json();

    if (transaction.status === "completed") {
      return transaction; // ready to attest
    }
    if (["error", "refunded", "expired"].includes(transaction.status)) {
      throw new Error(`Transaction ${txId} ended with status: ${transaction.status}`);
    }
    // pending_external, pending_user_transfer_start, etc. — keep polling
    await new Promise(r => setTimeout(r, 5000));
  }
}
```

---

## Step 5 – Finalise On-Chain

Once the anchor reports `status: completed`, submit the attestation to AnchorKit to record the completed interactive flow on-chain.

```rust
// Compute the payload hash from the transaction details
let payload_data = Bytes::from_slice(&env, transaction_id_bytes);
let hash = client.compute_payload_hash_public(&subject, &completed_at_timestamp, &payload_data);

// Submit the attestation
let attestation_id = client.submit_attestation(
    &attestor,
    &subject,
    &hash,
    &completed_at_timestamp,
    &nonce,
);
```

For session-aware tracing, wrap the submission in a session:

```rust
let session_id = client.create_session(&initiator);
let attestation_id = client.submit_attestation_with_session(
    &session_id,
    &attestor,
    &subject,
    &hash,
    &completed_at_timestamp,
    &nonce,
);
```

---

## Integrating with AnchorKit's On-Chain Contract

AnchorKit does not itself make outbound HTTP calls (it runs inside Soroban WASM). The interactive flow happens off-chain. The on-chain contract's role is to:

1. **Verify the anchor identity** — `is_attestor`, `get_endpoint`, `verify_sep10_token`
2. **Record the completed interaction** — `submit_attestation` or `submit_attestation_with_session`
3. **Provide an immutable audit trail** — `get_audit_log`, `get_session`

For capability detection before initiating a flow, use:

```rust
// Check that the anchor supports deposits before starting
if client.anchor_supports_deposits(&anchor, &String::from_str(&env, "USDC")) {
    // proceed with SEP-24 deposit
}
```

---

## Error Handling

| Scenario | AnchorKit error | HTTP side |
|----------|----------------|-----------|
| JWT expired before submission | `InvalidSep10Token (18)` | Re-authenticate with SEP-10 |
| Anchor not registered | `AttestorNotRegistered (3)` | Call `register_attestor` first |
| Duplicate nonce | `ReplayAttack (6)` | Generate a fresh nonce |
| TOML not cached | `CacheNotFound (49)` | Call `fetch_anchor_info` first |
| Anchor auto-deactivated | `UnauthorizedAttestor (4)` | Check health; use `update_health_status` |

---

## Example: Full Deposit Flow

```rust
// ---- On-chain setup (admin, one-time) ----
client.upsert_sep10_verifying_key(&issuer, &ed25519_key);
client.fetch_anchor_info(&anchor, &stellar_toml, &None);

// ---- Off-chain: initiate SEP-24 deposit ----
// 1. GET /.well-known/stellar.toml → parse TRANSFER_SERVER_SEP0024
// 2. POST /auth  → obtain SEP-10 challenge
// 3. Sign challenge → POST /auth → receive JWT
// 4. POST /deposit { asset_code, account, amount } → receive { id, url }
// 5. Open popup(url) → user completes KYC
// 6. Poll GET /transaction?id until status == "completed"

// ---- On-chain: record completed interaction ----
let session_id = client.create_session(&user_address);

let payload = Bytes::from_slice(&env, completed_tx_id.as_bytes());
let hash = client.compute_payload_hash_public(&user_address, &completed_at, &payload);

let attestation_id = client.submit_attestation_with_session(
    &session_id,
    &anchor,
    &user_address,
    &hash,
    &completed_at,
    &unique_nonce,
);

// Verify the record
let log = client.get_audit_log(&attestation_id);
```

---

For SEP-10 setup details, see [`docs/features/SEP10_AUTH.md`](./SEP10_AUTH.md).  
For anchor info discovery, see [`docs/features/ANCHOR_INFO_DISCOVERY.md`](./ANCHOR_INFO_DISCOVERY.md).  
For the full API reference, see [`docs/API_REFERENCE.md`](../API_REFERENCE.md).
