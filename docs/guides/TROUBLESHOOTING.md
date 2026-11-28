# Troubleshooting by ErrorCode

Every contract error is an `ErrorCode` from [`src/errors.rs`](../../src/errors.rs). Find the code (name or number) below for its likely cause and fix.

> CI runs `scripts/check_troubleshooting.py`, which fails if any `ErrorCode` variant is missing here. When adding a variant, add a section to this file.

See also: [ERROR_CODES_REFERENCE.md](../features/ERROR_CODES_REFERENCE.md), [GLOSSARY.md](../GLOSSARY.md).

| Code | Name |
|------|------|
| 1 | [`AlreadyInitialized`](#alreadyinitialized) |
| 2 | [`AttestorAlreadyRegistered`](#attestoralreadyregistered) |
| 3 | [`AttestorNotRegistered`](#attestornotregistered) |
| 4 | [`UnauthorizedAttestor`](#unauthorizedattestor) |
| 5 | [`InvalidTimestamp`](#invalidtimestamp) |
| 6 | [`ReplayAttack`](#replayattack) |
| 7 | [`InvalidQuote`](#invalidquote) |
| 8 | [`InvalidServiceType`](#invalidservicetype) |
| 9 | [`InvalidTransactionIntent`](#invalidtransactionintent) |
| 10 | [`StaleQuote`](#stalequote) |
| 11 | [`ComplianceNotMet`](#compliancenotmet) |
| 12 | [`InvalidEndpointFormat`](#invalidendpointformat) |
| 13 | [`NoQuotesAvailable`](#noquotesavailable) |
| 14 | [`ServicesNotConfigured`](#servicesnotconfigured) |
| 15 | [`ValidationError`](#validationerror) |
| 16 | [`RateLimitExceeded`](#ratelimitexceeded) |
| 17 | [`AttestationNotFound`](#attestationnotfound) |
| 18 | [`InvalidSep10Token`](#invalidsep10token) |
| 19 | [`StorageCorrupted`](#storagecorrupted) |
| 26 | [`NotInitialized`](#notinitialized) |
| 48 | [`CacheExpired`](#cacheexpired) |
| 49 | [`CacheNotFound`](#cachenotfound) |
| 51 | [`AuditLogMaxSizeInvalid`](#auditlogmaxsizeinvalid) |
| 52 | [`PendingAdminAlreadyExists`](#pendingadminalreadyexists) |
| 53 | [`NoPendingAdmin`](#nopendingadmin) |
| 54 | [`NotPendingAdmin`](#notpendingadmin) |
| 55 | [`SessionNotFound`](#sessionnotfound) |
| 56 | [`SessionExpired`](#sessionexpired) |
| 57 | [`MissingSigningKey`](#missingsigningkey) |
| 58 | [`UnauthorizedProposeAdmin`](#unauthorizedproposeadmin) |
| 59 | [`InvalidStrategy`](#invalidstrategy) |
| 60 | [`AttestationLimitReached`](#attestationlimitreached) |
| 61 | [`AttestorCapExceeded`](#attestorcapexceeded) |
| 62 | [`PathTraversalDetected`](#pathtraversaldetected) |
| 63 | [`InvalidAmount`](#invalidamount) |
| 64 | [`AttestationRevoked`](#attestationrevoked) |
| 65 | [`UnauthorizedSession`](#unauthorizedsession) |
| 121 | [`AttestationExpired`](#attestationexpired) |
| 122 | [`ContractPaused`](#contractpaused) |
| 123 | [`AdminTransferPending`](#admintransferpending) |
| 124 | [`ServerError`](#servererror) |
| 125 | [`TransportTimeout`](#transporttimeout) |
| 126 | [`TransactionNotFound`](#transactionnotfound) |
| 127 | [`QuoteNotFound`](#quotenotfound) |
| 128 | [`EndpointNotSet`](#endpointnotset) |
| 129 | [`AuditLogNotFound`](#auditlognotfound) |

## AlreadyInitialized

**Code:** 1

**Cause:** `initialize` was called on a contract that already has an admin.

**Fix:** Skip initialization; use admin functions (`propose_admin`) to change configuration.

## AttestorAlreadyRegistered

**Code:** 2

**Cause:** `register_attestor` was called for an address that is already active.

**Fix:** Check `is_attestor` before registering.

## AttestorNotRegistered

**Code:** 3

**Cause:** The address is not (or no longer) a registered attestor.

**Fix:** Register it with `register_attestor`, or check whether it was revoked.

## UnauthorizedAttestor

**Code:** 4

**Cause:** The caller is not allowed to perform this action (not an attestor, or not the issuer of the attestation).

**Fix:** Sign with the registered attestor / original issuer key.

## InvalidTimestamp

**Code:** 5

**Cause:** The timestamp is zero, too far in the future, or older than the allowed clock skew.

**Fix:** Use the current ledger time; check the host clock (NTP) and the configured skew.

## ReplayAttack

**Code:** 6

**Cause:** The payload hash was already submitted within the replay window.

**Fix:** Do not resubmit; generate a new payload (e.g. include a nonce) if a new attestation is intended.

## InvalidQuote

**Code:** 7

**Cause:** Quote fields are malformed (zero rate, inverted min/max, bad assets).

**Fix:** Validate quote input before submitting.

## InvalidServiceType

**Code:** 8

**Cause:** An unknown service type was supplied.

**Fix:** Use one of the supported service types (deposits, withdrawals, quotes, KYC).

## InvalidTransactionIntent

**Code:** 9

**Cause:** The transaction intent is incomplete or inconsistent.

**Fix:** Check amount, assets and direction in the intent.

## StaleQuote

**Code:** 10

**Cause:** The quote's expiry has passed.

**Fix:** Request a fresh quote and retry.

## ComplianceNotMet

**Code:** 11

**Cause:** KYC/compliance requirements for this anchor or jurisdiction are not satisfied.

**Fix:** Complete SEP-12 KYC or choose an anchor whose policy allows the request.

## InvalidEndpointFormat

**Code:** 12

**Cause:** The endpoint URL is not a valid HTTPS URL.

**Fix:** Use an `https://` URL with a valid host and no credentials.

## NoQuotesAvailable

**Code:** 13

**Cause:** No anchor returned a matching quote.

**Fix:** Widen the routing criteria, or check that anchors support the asset pair.

## ServicesNotConfigured

**Code:** 14

**Cause:** The anchor has no supported services configured.

**Fix:** Call `configure_services` for the anchor before routing to it.

## ValidationError

**Code:** 15

**Cause:** Generic input validation failure.

**Fix:** Inspect the error message/context for the failing field.

## RateLimitExceeded

**Code:** 16

**Cause:** Too many operations from this caller within the rate-limit window.

**Fix:** Back off and retry later, or have the admin raise the limit.

## AttestationNotFound

**Code:** 17

**Cause:** No attestation exists for the given ID.

**Fix:** Check the ID; use pagination APIs to list a subject's attestations.

## InvalidSep10Token

**Code:** 18

**Cause:** The SEP-10 JWT is missing, expired, malformed, or signed by an unregistered key.

**Fix:** Re-run SEP-10 authentication and ensure the anchor's signing key is registered.

## StorageCorrupted

**Code:** 19

**Cause:** A storage entry could not be decoded.

**Fix:** Report it; restore archived entries or redeploy/migrate per MIGRATION.md.

## NotInitialized

**Code:** 26

**Cause:** The contract has not been initialized.

**Fix:** Call `initialize` with an admin address.

## CacheExpired

**Code:** 48

**Cause:** A cached metadata/capabilities/toml entry exceeded its TTL.

**Fix:** Re-fetch from the anchor and re-cache. This is expected; see ADR 0001.

## CacheNotFound

**Code:** 49

**Cause:** No cache entry exists (never cached, or evicted from temporary storage).

**Fix:** Fetch from the anchor and cache it.

## AuditLogMaxSizeInvalid

**Code:** 51

**Cause:** `max_audit_log_size` was set to 0.

**Fix:** Use a value of at least 1.

## PendingAdminAlreadyExists

**Code:** 52

**Cause:** An admin transfer is already pending.

**Fix:** Accept or cancel the pending transfer first.

## NoPendingAdmin

**Code:** 53

**Cause:** There is no pending admin transfer to accept or cancel.

**Fix:** Call `propose_admin` first.

## NotPendingAdmin

**Code:** 54

**Cause:** The caller is not the proposed admin.

**Fix:** Accept the transfer from the proposed admin address.

## SessionNotFound

**Code:** 55

**Cause:** The session ID does not exist.

**Fix:** Create a session first, or check the ID.

## SessionExpired

**Code:** 56

**Cause:** The session's lifetime has elapsed.

**Fix:** Create a new session.

## MissingSigningKey

**Code:** 57

**Cause:** No attestation signing key is registered for the issuer.

**Fix:** Register a signing key for the issuer.

## UnauthorizedProposeAdmin

**Code:** 58

**Cause:** Only the current admin can propose a new admin.

**Fix:** Sign with the current admin key.

## InvalidStrategy

**Code:** 59

**Cause:** An unknown routing strategy was supplied.

**Fix:** Use a supported strategy (see ROUTING_STRATEGY.md).

## AttestationLimitReached

**Code:** 60

**Cause:** The subject has reached the maximum number of attestations.

**Fix:** Revoke unused attestations or raise the configured limit.

## AttestorCapExceeded

**Code:** 61

**Cause:** The maximum number of attestors is registered.

**Fix:** Revoke an inactive attestor or raise the cap.

## PathTraversalDetected

**Code:** 62

**Cause:** A path or endpoint contains `..` or similar traversal segments.

**Fix:** Remove traversal segments from the input.

## InvalidAmount

**Code:** 63

**Cause:** Amount is zero, negative, or out of range.

**Fix:** Pass a positive amount within limits.

## AttestationRevoked

**Code:** 64

**Cause:** The attestation was revoked by its issuer.

**Fix:** Request a new attestation; see ADR 0002.

## UnauthorizedSession

**Code:** 65

**Cause:** The caller does not own the session.

**Fix:** Use the session initiator's address.

## AttestationExpired

**Code:** 121

**Cause:** The attestation is past its expiry.

**Fix:** Request a fresh attestation.

## ContractPaused

**Code:** 122

**Cause:** The admin has paused the contract.

**Fix:** Wait for the admin to unpause.

## AdminTransferPending

**Code:** 123

**Cause:** The action is blocked while an admin transfer is pending.

**Fix:** Complete or cancel the transfer.

## ServerError

**Code:** 124

**Cause:** The anchor returned a 5xx error.

**Fix:** Retry with backoff (see RETRY_BACKOFF.md); check anchor status.

## TransportTimeout

**Code:** 125

**Cause:** The request to the anchor timed out.

**Fix:** Retry with backoff; check network connectivity and anchor health.

## TransactionNotFound

**Code:** 126

**Cause:** The anchor has no transaction with this ID.

**Fix:** Check the transaction ID and anchor.

## QuoteNotFound

**Code:** 127

**Cause:** No quote exists with this ID.

**Fix:** Request a new quote.

## EndpointNotSet

**Code:** 128

**Cause:** No endpoint is configured for the attestor/anchor.

**Fix:** Call `set_endpoint`.

## AuditLogNotFound

**Code:** 129

**Cause:** No audit log entry exists for the given ID.

**Fix:** Check the ID or session.
