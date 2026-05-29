# AnchorKit Error Codes Reference

Quick lookup table for all error codes and their properties.

## Migration Note (contiguous renumbering)

Error codes were previously non-contiguous (1-19, then 48-54, with `NotInitialized` at 101).
They have been renumbered to the contiguous range **1-26**. If you match on raw numeric
values, update your mappings using the table below:

| Old code | New code | Name                     |
|----------|----------|--------------------------|
| 1-19     | 1-19     | unchanged                |
| 48       | 20       | CacheExpired             |
| 49       | 21       | CacheNotFound            |
| 51       | 22       | AuditLogMaxSizeInvalid   |
| 52       | 23       | UnauthorizedProposeAdmin |
| 53       | 24       | NoPendingAdmin           |
| 54       | 25       | NotPendingAdmin          |
| 101      | 26       | NotInitialized           |

## Legacy Raw Error Codes (48-54)

| Code | Name                     | Description |
|------|--------------------------|-------------|
| 48   | CacheExpired             | Cache entry has expired |
| 49   | CacheNotFound            | Cache entry not found |
| 51   | AuditLogMaxSizeInvalid   | `max_audit_log_size` must be at least 1 |
| 52   | UnauthorizedProposeAdmin | A pending admin proposal already exists |
| 53   | NoPendingAdmin           | No pending admin transfer found |
| 54   | NotPendingAdmin          | Caller is not the pending admin |

## On-Chain Error Codes (1-26)

| Code | Name                     | Severity | Retryable |
|------|--------------------------|----------|-----------|
| 1    | AlreadyInitialized       | Medium   | No        |
| 2    | AttestorAlreadyRegistered | Medium  | No        |
| 3    | AttestorNotRegistered    | Medium   | No        |
| 4    | UnauthorizedAttestor     | High     | No        |
| 5    | InvalidTimestamp         | Medium   | No        |
| 6    | ReplayAttack             | Critical | No        |
| 7    | InvalidQuote             | Medium   | No        |
| 8    | InvalidServiceType       | Medium   | No        |
| 9    | InvalidTransactionIntent | Medium   | No        |
| 10   | StaleQuote               | Low      | Yes       |
| 11   | ComplianceNotMet         | Critical | No        |
| 12   | InvalidEndpointFormat    | Medium   | No        |
| 13   | NoQuotesAvailable        | Low      | Yes       |
| 14   | ServicesNotConfigured    | Medium   | Yes       |
| 15   | ValidationError          | Medium   | No        |
| 16   | RateLimitExceeded        | Medium   | No        |
| 17   | AttestationNotFound      | Medium   | Yes       |
| 18   | InvalidSep10Token        | High     | No        |
| 19   | StorageCorrupted         | High     | No        |
| 20   | CacheExpired             | Low      | Yes       |
| 21   | CacheNotFound            | Low      | Yes       |
| 22   | AuditLogMaxSizeInvalid   | Medium   | No        |
| 23   | UnauthorizedProposeAdmin | High     | No        |
| 24   | NoPendingAdmin           | Medium   | No        |
| 25   | NotPendingAdmin          | Medium   | No        |
| 26   | NotInitialized           | Medium   | No        |
