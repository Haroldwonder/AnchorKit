# AnchorKit Threat Model

This document describes the trust assumptions, attack surfaces, in-code mitigations, and known gaps for AnchorKit. It complements the reporting process in [SECURITY.md](../SECURITY.md) and the design notes in [docs/features/AUTHORIZATION_MODEL.md](./features/AUTHORIZATION_MODEL.md) and [docs/features/SEP10_AUTH.md](./features/SEP10_AUTH.md).

## 1. Trust Assumptions

### Admin
- The admin key is trusted to configure the contract (attestors, anchors, thresholds, pause state).
- Admin actions are assumed to be authorized and intentional; the contract enforces *who* may act, not *whether* the action is wise.
- Compromise of the admin key is treated as a full compromise of contract configuration.

### Attestors
- Attestors are trusted to sign only truthful attestations about the subjects they are authorized for.
- The contract verifies attestor signatures and authorization, but cannot verify the real-world truth of an attestation.
- A colluding quorum of attestors can produce false attestations; this is a governance risk, not a code bug.

### Anchors
- Anchors (SEP-1/SEP-10/SEP-24/SEP-31/SEP-38 participants) are trusted to operate their own endpoints and keys.
- The contract trusts anchor-provided identifiers only after signature and authorization checks.

### RPC / Ledger
- The Soroban RPC and the Stellar ledger are trusted to provide correct state and ordering.
- Clients are responsible for submitting transactions with fresh, valid ledger bounds; the contract enforces replay protection independently of the RPC.

## 2. Attack Surfaces

### Replay
- Re-submitting a previously valid signed payload or transaction.
- Cross-contract or cross-network replay of attestations.

### Signature Forgery
- Forging attestor or admin signatures.
- Substituting keys or bypassing signature verification paths.

### Quote Manipulation
- Tampering with quoted rates, fees, or amounts between signing and settlement.
- Submitting stale or out-of-order quotes.

### Storage Griefing
- Inflating persistent/temporary storage to exhaust rent or block legitimate writes.
- Repeated writes to force unbounded growth of stored records.

### SEP-10 Phishing
- A malicious service impersonating an anchor to harvest SEP-10 signed challenges.
- Replaying a SEP-10 challenge/JWT against a different domain or anchor.

## 3. Mitigations in the Code

- **Authorization tiers** — Admin-only, self-only, and public functions are separated and enforced via `require_auth` and admin checks (see `docs/features/AUTHORIZATION_MODEL.md`).
- **Replay protection** — Signed payloads include nonces/ledger bounds and are validated before state changes; consumed nonces are recorded to prevent reuse.
- **Signature verification** — Attestor and admin signatures are verified against registered keys before any state mutation; unknown or unauthorized signers are rejected with stable error codes.
- **Quote integrity** — Quotes are bound to their signed payload (subject, amount, timestamp) and validated on submission; stale quotes are rejected.
- **Storage hygiene** — Writes are gated by authorization and keyed deterministically to limit unbounded growth; temporary storage is used where persistence is not required.
- **SEP-10 verification** — JWT verification in `src/sep10_jwt.rs` validates issuer, audience/domain, and expiry to resist phishing and cross-domain replay.
- **Stable error codes** — Auth and validation failures return stable codes (see `docs/features/ERROR_CODES_REFERENCE.md`) so integrators can distinguish rejection reasons.

## 4. Known Gaps

- The contract cannot validate the real-world truth of attestations; trust in attestors is a governance assumption.
- Admin key compromise is not mitigated on-chain; key management (multisig, rotation) is the operator's responsibility.
- Storage griefing is bounded by authorization but not by hard per-account quotas; operators should monitor rent and storage growth.
- SEP-10 phishing resistance depends on clients validating the anchor domain and JWT audience; the contract only verifies the JWT it receives.
- Dependency and platform vulnerabilities are out of scope here; see [SECURITY.md](../SECURITY.md) for reporting.

## 5. Reporting

To report a vulnerability, follow the process in [SECURITY.md](../SECURITY.md). Do not open public issues for security problems.
