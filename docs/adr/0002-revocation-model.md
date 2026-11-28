# 0002. Revocation model (soft-revoke)

- **Status:** Accepted
- **Date:** 2026-11-23

## Context

Attestors and individual attestations sometimes need to be withdrawn: a key is
compromised, an attestor leaves, or a single attestation was issued in error.
Attestations are referenced by ID from off-chain systems and audit trails, so
removing them would break history and verification of past decisions.

## Decision

AnchorKit uses **soft revocation**: records are flagged, never deleted.

- `revoke_attestor(attestor)` removes the attestor's active registration (so it
  can no longer submit attestations, failing with `AttestorNotRegistered` /
  `UnauthorizedAttestor`) and sets the persistent flag
  `StorageKey::AttestorRevoked(attestor)`. Historical attestations stay readable
  and surface `issuer_revoked = true`.
- `revoke_attestation(attestor, id)` may only be called by the original issuer
  (`UnauthorizedAttestor` otherwise; `AttestationNotFound` for unknown IDs). It
  sets the persistent flag `StorageKey::AttestationRevoked(id)`. The record stays readable; the
  `AttestationRevoked` error code is reserved for operations that must reject it.
- Re-registering an attestor clears `AttestorRevoked` so new attestations are not
  flagged.
- Each revocation emits an event (`attestor/revoked`, `attest/revoked`) so
  indexers can react.

## Alternatives considered

- **Hard delete:** removes history and lets replay flags or IDs be ambiguous.
- **Cascading revoke of every attestation by an attestor:** O(n) writes per
  revocation and exceeds ledger limits for prolific attestors; the lazy
  `issuer_revoked` flag gives the same information at read time.

## Consequences

- Readers must check revocation flags, not just existence.
- Storage grows monotonically; flags are persistent and TTL-extended.
