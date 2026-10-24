# AnchorKit Error Codes Reference

Quick lookup table for all error codes and their properties.

## Error Codes

Error codes are non-contiguous. See the table below for the current values.

> **Reserved / deprecated codes:** Some variants are kept only for ABI stability and are
> never raised by the contract. Their discriminants remain reserved and must not be
> reused. These are marked **(reserved)** below. Integrators should match on the
> canonical code instead (e.g. `PendingAdminAlreadyExists` (52) rather than
> `AdminTransferPending` (123)).

| Code | Name                     | Description |
|------|--------------------------|-------------|
| 1    | AlreadyInitialized       | Contract is already initialized |
| 2    | AttestorAlreadyRegistered | Attestor is already registered |
| 3    | AttestorNotRegistered    | Attestor is not registered |
| 4    | UnauthorizedAttestor     | Attestor is not authorized |
| 5    | InvalidTimestamp         | Timestamp is invalid |
| 6    | ReplayAttack             | Replay attack detected |
| 7    | InvalidQuote             | Quote is invalid |
| 8    | InvalidServiceType       | Service type is invalid |
| 9    | InvalidTransactionIntent | Transaction intent is invalid |
| 10   | StaleQuote               | Quote has expired |
| 11   | ComplianceNotMet         | Compliance requirements not met |
| 12   | InvalidEndpointFormat    | Endpoint format is invalid |
| 13   | NoQuotesAvailable        | No quotes are available |
| 14   | ServicesNotConfigured    | Services are not configured |
| 15   | ValidationError          | Input validation failed |
| 16   | RateLimitExceeded        | Rate limit exceeded |
| 17   | AttestationNotFound      | Attestation not found |
| 18   | InvalidSep10Token        | SEP-10 JWT is missing, expired, or invalid |
| 19   | StorageCorrupted         | On-chain storage entry is corrupted or unreadable |
| 26   | NotInitialized           | Contract is not initialized |
| 48   | CacheExpired             | Cache entry has expired |
| 49   | CacheNotFound            | Cache entry not found |
| 51   | AuditLogMaxSizeInvalid   | `max_audit_log_size` must be at least 1 |
| 52   | PendingAdminAlreadyExists | An admin transfer is already pending |
| 53   | NoPendingAdmin           | No pending admin transfer found |
| 54   | NotPendingAdmin          | Caller is not the pending admin **(reserved, never raised)** |
| 55   | SessionNotFound          | Session not found |
| 56   | SessionExpired           | Session has expired |
| 57   | MissingSigningKey        | Anchor TOML does not publish a signing key |
| 58   | UnauthorizedProposeAdmin | Only admin can propose new admin **(reserved, never raised)** |
| 59   | InvalidStrategy          | Routing strategy symbol is not recognized |
| 60   | AttestationLimitReached  | Attestation ID counter has reached its maximum value |
| 61   | AttestorCapExceeded      | Maximum number of attestors has been reached (also raised when the cached-anchor list is full) |
| 62   | PathTraversalDetected    | URL contains a path traversal sequence |
| 63   | InvalidAmount            | Amount is outside the allowed min/max range for this asset |
| 64   | AttestationRevoked       | Attestation has been revoked |
| 121  | AttestationExpired       | Attestation has expired |
| 122  | ContractPaused           | Contract is paused |
| 123  | AdminTransferPending     | Admin transfer is already pending **(reserved, duplicates 52; never raised)** |
