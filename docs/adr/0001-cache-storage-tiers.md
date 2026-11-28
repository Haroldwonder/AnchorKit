# 0001. Cache storage tiers (persistent vs temporary)

- **Status:** Accepted
- **Date:** 2026-11-23

## Context

Soroban offers three storage tiers: `instance`, `persistent` and `temporary`.
They differ in rent cost and in what happens when an entry's TTL lapses:
persistent entries are archived and can be restored, temporary entries are
deleted for good.

AnchorKit stores two kinds of data:

- **Authoritative state** - attestor registrations, attestation records,
  replay-protection flags for payload hashes, revocation flags, SEP-10 keys,
  endpoints and supported services.
- **Derived / cached data** - anchor metadata, capabilities, and `stellar.toml`
  content that can always be re-fetched from the anchor.

## Decision

- Authoritative state uses **persistent** storage, and its TTL is extended
  (`PERSISTENT_TTL`) on write.
- Caches (`StorageKey::MetadataCache`, `StorageKey::CapabilitiesCache`,
  `StorageKey::TomlCache`) and tracing spans use **temporary** storage, with a
  TTL matching the caller-supplied cache TTL.
- Reading a missing cache entry returns `CacheNotFound`; reading an entry whose
  logical TTL has passed returns `CacheExpired`. Callers re-fetch and re-cache
  in both cases.

## Alternatives considered

- **Everything persistent:** simple, but caches would pay persistent rent and
  stale entries would be archived rather than dropped.
- **Everything temporary:** cheap, but losing a replay flag or attestation when
  its TTL lapses would break replay protection and audit history.

## Consequences

- Cache misses are normal and must be handled by callers (see the
  [troubleshooting guide](../guides/TROUBLESHOOTING.md#cacheexpired)).
- Replay protection stays sound over the contract's lifetime because payload
  hash flags never silently disappear.
- New storage keys must document their tier in `src/storage.rs`.
