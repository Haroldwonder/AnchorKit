# AnchorKit Documentation

Complete index of all AnchorKit documentation.

## Migration

| Document | Description |
|---|---|
| [MIGRATION.md](./MIGRATION.md) | Upgrade guide: 0.0.1 → 0.1.0 breaking changes and before/after examples |

## Feature Documentation

Reference docs for each feature exposed by the contract and SDK.

| Document | Description |
|---|---|
| [ANCHOR_INFO_DISCOVERY.md](./features/ANCHOR_INFO_DISCOVERY.md) | Fetch and parse stellar.toml, cache assets/fees/limits |
| [ANCHOR_ADAPTER.md](./features/ANCHOR_ADAPTER.md) | Unified anchor adapter interface |
| [METADATA_CACHE.md](./features/METADATA_CACHE.md) | TTL-based metadata and capabilities caching |
| [REQUEST_ID_PROPAGATION.md](./features/REQUEST_ID_PROPAGATION.md) | UUID per flow with tracing |
| [LOGGING.md](./features/LOGGING.md) | Logging system |
| [DOMAIN_VALIDATION.md](./features/DOMAIN_VALIDATION.md) | Domain validation |
| [ERROR_CODES_REFERENCE.md](./features/ERROR_CODES_REFERENCE.md) | Stable API error codes reference |
| [RETRY_BACKOFF.md](./features/RETRY_BACKOFF.md) | Retry and backoff strategies |
| [RESPONSE_NORMALIZATION.md](./features/RESPONSE_NORMALIZATION.md) | Response normalization |
| [TRANSACTION_STATE_TRACKER.md](./features/TRANSACTION_STATE_TRACKER.md) | Transaction state tracking |
| [WEBHOOK_MIDDLEWARE.md](./features/WEBHOOK_MIDDLEWARE.md) | Webhook middleware |
| [WEBHOOK_MONITOR.md](./features/WEBHOOK_MONITOR.md) | Webhook monitoring |
| [SEP10_AUTH.md](./features/SEP10_AUTH.md) | SEP-10 authentication |
| [SDK_CONFIG.md](./features/SDK_CONFIG.md) | SDK configuration |
| [STATUS_MONITOR.md](./features/STATUS_MONITOR.md) | Status monitoring |
| [SKELETON_LOADERS.md](./features/SKELETON_LOADERS.md) | UI skeleton loaders |
| [REQUEST_HISTORY_PANEL.md](./features/REQUEST_HISTORY_PANEL.md) | Request history panel |
| [RATE_LIMIT_PLAN.md](./features/RATE_LIMIT_PLAN.md) | Rate limiting |
| [INTERACTIVE_SUPPORT.md](./features/INTERACTIVE_SUPPORT.md) | Interactive support |

## Guides

How-to guides for contributors and operators.

| Document | Description |
|---|---|
| [DOCTOR_COMMAND.md](./guides/DOCTOR_COMMAND.md) | CLI doctor command and environment diagnostics |
| [CONTRIBUTING.md](./guides/CONTRIBUTING.md) | Contribution guidelines |
| [ERROR_IMPLEMENTATION_GUIDE.md](./guides/ERROR_IMPLEMENTATION_GUIDE.md) | Error handling implementation guide |
| [RETRY_QUICK_REFERENCE.md](./guides/RETRY_QUICK_REFERENCE.md) | Retry quick reference |
| [DOMAIN_VALIDATOR_QUICK_REF.md](./guides/DOMAIN_VALIDATOR_QUICK_REF.md) | Domain validator quick reference |
| [NORMALIZATION_QUICK_REF.md](./guides/NORMALIZATION_QUICK_REF.md) | Normalization quick reference |
| [SNAPSHOT_CONVENTIONS.md](./guides/SNAPSHOT_CONVENTIONS.md) | How Soroban snapshot paths are derived and how to keep test_snapshots/ clean |

## Internal Notes

Implementation summaries and CI notes for maintainers.

See [`docs/internal/`](./internal/) for archived implementation notes, pipeline summaries, and CI status records.

## Wallet Connector

- [WALLET_CONNECTOR_README.md](./WALLET_CONNECTOR_README.md) - Wallet connector component
