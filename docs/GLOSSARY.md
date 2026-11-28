# Glossary

Terms used across AnchorKit's contract, CLI, SDK and docs.

| Term | Definition |
|------|------------|
| **Anchor** | A Stellar service that moves value between the Stellar network and off-chain rails (bank transfers, cash, other chains). AnchorKit registers anchors, stores their metadata and services, and routes requests to them. |
| **Attestor** | An address authorised by the contract admin to submit attestations. Registered with `register_attestor`, removed with `revoke_attestor` (see [ADR 0002](./adr/0002-revocation-model.md)). |
| **Attestation** | An on-chain record signed by an attestor asserting a fact about a subject (e.g. KYC passed, deposit received). Stored by ID in persistent storage with issuer, subject, timestamp and payload hash. |
| **Payload hash** | The SHA-256 hash of the attested off-chain payload. Only the hash goes on-chain; it is also the key for replay protection. |
| **SEP-1** | `stellar.toml`: the file an anchor publishes at `/.well-known/stellar.toml` describing its endpoints, assets and signing keys. AnchorKit caches it (`TomlCache`). |
| **SEP-6** | Programmatic (non-interactive) deposit and withdrawal API. |
| **SEP-10** | Stellar Web Authentication: a challenge transaction signed by the client, exchanged for a JWT. AnchorKit verifies SEP-10 JWTs against registered keys (`InvalidSep10Token` on failure). |
| **SEP-12** | KYC API for submitting and querying customer information. |
| **SEP-24** | Interactive deposit and withdrawal through an anchor-hosted web flow. |
| **SEP-31** | Cross-border payments between two anchors (sending and receiving). |
| **SEP-38** | Anchor RFQ API: firm and indicative quotes for asset exchange. |
| **Quote** | A price offer from an anchor for converting one asset to another, with an expiry. Expired quotes fail with `StaleQuote`. |
| **Routing strategy** | The rule used to pick an anchor for a request (e.g. best rate, lowest fee, fastest, health-weighted). See [ROUTING_STRATEGY.md](./features/ROUTING_STRATEGY.md). Unknown strategies fail with `InvalidStrategy`. |
| **Session** | A grouped, auditable sequence of operations by one initiator, created with a session ID and nonce. Operations within a session are logged for replay and audit. Errors: `SessionNotFound`, `SessionExpired`, `UnauthorizedSession`. |
| **Health score** | A 0-100 score per anchor derived from latency, failure rate and availability, used by routing and the status monitor. See [ANCHOR_HEALTH_SCORE.md](./features/ANCHOR_HEALTH_SCORE.md). |
| **Replay window** | The period during which a submitted payload hash is remembered; resubmitting the same hash fails with `ReplayAttack`. Replay flags live in persistent storage. |
| **Clock skew** | The allowed difference between an attestation's timestamp and the ledger timestamp. Timestamps that are zero, too far in the future, or too old fail with `InvalidTimestamp`. |

See also: [ARCHITECTURE.md](./ARCHITECTURE.md), [Troubleshooting](./guides/TROUBLESHOOTING.md), [Architecture decisions](./adr/README.md).
