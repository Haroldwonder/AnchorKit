# SEP-6 `on_change_callback`

SEP-6 deposit and withdrawal requests may include an `on_change_callback` URL.
When a transaction's `status` changes, the anchor POSTs the full
`/transaction` response object to that URL instead of the wallet having to
poll. AnchorKit exposes the parameter on both the raw request inputs and the
normalized responses.

## Using the parameter

`on_change_callback` is an `Option<String>`. `None` (the default for existing
callers) means no callback is registered and polling continues as before.

```rust
use anchorkit::sep6::{initiate_deposit, initiate_withdrawal,
                      RawDepositResponse, RawWithdrawalResponse};

// Deposit: add the callback URL to the raw request passed to the anchor.
let mut deposit = RawDepositResponse {
    transaction_id: "txn-123".into(),
    how: "Send funds to bank account 1234".into(),
    extra_info: None,
    min_amount: Some(10),
    max_amount: Some(10_000),
    fee_fixed: None,
    fee_percent: None,
    status: None,
    depositor_account: None,
    claimable_balance_supported: false,
    on_change_callback: Some("https://wallet.example.com/sep6/callback".into()),
};
let deposit = initiate_deposit(deposit, "USDC").unwrap();
assert_eq!(
    deposit.on_change_callback.as_deref(),
    Some("https://wallet.example.com/sep6/callback")
);

// Withdrawal: the field works the same way.
let mut withdrawal = RawWithdrawalResponse {
    transaction_id: "txn-456".into(),
    account_id: "GAAACAQDAQCQMBYIBEFAWDANBYHRAEISCMKBKFQXDAMRUGY4DUPB7JZX".into(),
    dest_account_id: None,
    memo: None,
    memo_type: None,
    min_amount: None,
    max_amount: None,
    fee_fixed: None,
    fee_percent: None,
    status: None,
    on_change_callback: Some("https://wallet.example.com/sep6/callback".into()),
};
let withdrawal = initiate_withdrawal(withdrawal, "USDC").unwrap();
assert!(withdrawal.on_change_callback.is_some());
```

The validated URL is propagated unchanged to `DepositResponse` /
`WithdrawalResponse`, so callers can confirm which callback they registered.

## Validation rules

The field is validated with the same `validate_anchor_domain` helper used for
anchor endpoints. A present value must:

- use the `https` scheme (uppercase `HTTPS://` is accepted; `http://` is not);
- have a real hostname — `localhost`, `127.0.0.1`, bare IPs and single-label
  hosts are rejected;
- be at most 2048 characters and contain no path-traversal sequences.

An invalid value returns `ErrorCode::ValidationError` with
`Invalid on_change_callback: must be a valid HTTPS URL` as the message and the
offending URL as context. `None` is always accepted. Note that the validator
checks the URL's *shape* — it does not make a network request to the callback
endpoint.

## Verifying the callback signature

An anchor MUST sign every callback; do not trust an unsigned POST. Follow the
[SEP-6 callback signature specification](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0006.md#callback-signature):

1. Read the `SIGNING_KEY` from the anchor's `stellar.toml`. This is an Ed25519
   **public** key (`G...`); the anchor signs with the matching private key.
2. Get the signature header: `Signature: t=<unix-timestamp>, s=<base64-signature>`.
   Accept the deprecated `X-Stellar-Signature` header with the same format for
   older anchors.
3. Check freshness: reject the request if `now - t` exceeds your tolerance
   (1–2 minutes) or if `t` is in the future.
4. Base64-decode `s` into 64 raw signature bytes.
5. Read the **raw** request body bytes (do not re-serialize the JSON).
6. Build the signed payload by joining, with literal `.` separators:

   ```text
   <timestamp>.<wallet-host>.<raw-body>
   ```

   `<wallet-host>` is the host of the callback URL you registered (including
   the port when non-default), e.g. `wallet.example.com`. The timestamp is the
   decimal string `t` from the header, not a freshly generated one.
7. Verify the Ed25519 signature over that payload with the anchor's
   `SIGNING_KEY`. Reject the callback if verification fails.
8. Record `(t, s)` for a short period and reject duplicates to stop a valid
   callback from being replayed within the freshness window.

### Example (Node.js)

```js
const crypto = require('crypto');
const stellar = require('@stellar/stellar-sdk');

const ANCHOR_SIGNING_KEY = process.env.ANCHOR_SIGNING_KEY; // from stellar.toml
const CALLBACK_HOST = 'wallet.example.com';
const seen = new Set(); // use a shared, expiring store in production

function verifyCallback(rawBody, header, now = Math.floor(Date.now() / 1000)) {
  const match = /^t=(\d+),\s*s=([A-Za-z0-9+/=]+)$/.exec(header);
  if (!match) return false;
  const [, t, s] = match;
  const age = now - Number(t);
  if (age < 0 || age > 120 || seen.has(`${t}:${s}`)) return false;

  const payload = Buffer.concat([
    Buffer.from(`${t}.${CALLBACK_HOST}.`), // note the trailing dot
    rawBody,
  ]);
  const signature = Buffer.from(s, 'base64');
  const key = stellar.Keypair.fromPublicKey(ANCHOR_SIGNING_KEY);
  const ok = key.verify(payload, signature);
  if (ok) seen.add(`${t}:${s}`);
  return ok;
}

// Express: keep the raw body for signature verification.
app.use('/sep6/callback', express.raw({ type: 'application/json' }));
app.post('/sep6/callback', (req, res) => {
  const header = req.get('Signature') ?? req.get('X-Stellar-Signature');
  if (!header || !verifyCallback(req.body, header)) {
    return res.sendStatus(401);
  }
  const transaction = JSON.parse(req.body.toString('utf8'));
  // Handle transaction.id / transaction.status here.
  return res.sendStatus(200);
});
```

### Example (Rust)

```rust,ignore
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use stellar_strkey::ed25519::PublicKey; // decode the G... SIGNING_KEY

fn verify_callback(raw_body: &[u8], t: &str, s: &str, signing_key: &str) -> bool {
    // Also check the timestamp freshness and replay cache before calling this.
    let Ok(sig_bytes) = base64::decode(s) else { return false; };
    let Ok(signature) = Signature::from_slice(&sig_bytes) else { return false; };
    let Ok(public) = PublicKey::from_string(signing_key) else { return false; };
    let Ok(vk) = VerifyingKey::from_bytes(&public.0) else { return false; };

    let mut payload = format!("{t}.wallet.example.com.").into_bytes();
    payload.extend_from_slice(raw_body);
    vk.verify(&payload, &signature).is_ok()
}
```

> Always verify against the `SIGNING_KEY` you fetched from the anchor's
> authenticated `stellar.toml`, and keep the raw body bytes: JSON
> re-serialization (key order, whitespace, number formatting) changes the
> signed payload and will cause valid callbacks to fail verification.

## References

- [SEP-6: Deposit and Withdrawal API](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0006.md)
- [SEP-1: `stellar.toml` `SIGNING_KEY`](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0001.md)
