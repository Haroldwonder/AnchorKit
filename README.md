# AnchorKit

[![UI Tests](https://github.com/Haroldwonder/AnchorKit/actions/workflows/ui-tests.yml/badge.svg)](https://github.com/Haroldwonder/AnchorKit/actions/workflows/ui-tests.yml)

AnchorKit is a Soroban-native toolkit for anchoring off-chain attestations to Stellar. It enables smart contracts to verify real-world events such as KYC approvals, payment confirmations, and signed claims in a trust-minimized way.


## Features

- Attestation management with replay attack protection
- Attestor registration and revocation
- Endpoint configuration for attestors
- Service capability discovery (deposits, withdrawals, quotes, KYC)
- **Anchor Info Discovery** (fetch and parse stellar.toml, cache assets/fees/limits)
- **Health monitoring** (latency, failures, availability)
- **Metadata caching** (TTL-based with manual refresh)
- **Request ID propagation** (UUID per flow with tracing)
- Event emission for all state changes
- Comprehensive error handling with stable error codes

## Supported Services

Anchors can configure which services they support:

- **Deposits**: Accept incoming deposits from users
- **Withdrawals**: Process withdrawal requests
- **Quotes**: Provide exchange rate quotes
- **KYC**: Perform Know Your Customer verification

## Usage Example

```rust
// Initialize the contract
contract.initialize(&admin);

// Register an attestor/anchor
contract.register_attestor(&anchor);

// Configure supported services for the anchor
let mut services = Vec::new(&env);
services.push_back(ServiceType::Deposits);
services.push_back(ServiceType::Withdrawals);
services.push_back(ServiceType::KYC);
contract.configure_services(&anchor, &services);

// Query supported services
let supported = contract.get_supported_services(&anchor);

// Check if a specific service is supported
if contract.supports_service(&anchor, &ServiceType::Deposits) {
    // Process deposit
}

// NEW: Compute payload hash for off-chain matching (matches on-chain deterministic_hash exactly)
let subject = Address::generate(&env);
let timestamp: u64 = env.ledger().timestamp();
let payload_data = Bytes::from_slice(&env, b"kyc_approved");
let payload_hash = contract.compute_payload_hash_public(&env, subject, timestamp, payload_data);

// Use same inputs on-chain to verify attestation matches expected hash
let expected_hash = deterministic_hash::compute_payload_hash(&env, &subject, timestamp, &payload_data);
assert_eq!(payload_hash, expected_hash);
```

## CLI Example

See complete deposit/withdraw workflow:

```bash
# Run bash demo
./examples/cli_example.sh

# Or run Rust example
cargo run --example cli_example
```

See **[docs/guides/DOCTOR_COMMAND.md](./docs/guides/DOCTOR_COMMAND.md)** for CLI documentation.

## Key Features

- **Attestation Management**: Register attestors, submit and retrieve attestations
- **Endpoint Configuration**: Manage attestor endpoints for off-chain integration
- **Unified Anchor Adapter**: Consistent API for multiple anchor integrations
- **Session Management**: Group operations into logical sessions for traceability
- **Audit Trail**: Complete immutable record of all operations
- **Reproducibility**: Deterministic operation replay for verification
- **Replay Protection**: Multi-level protection against unauthorized replays
- **Secure Credential Management**: Runtime credential injection with automatic rotation

## New: Session Traceability & Reproducibility

AnchorKit now includes comprehensive session management and operation tracing to ensure all anchor interactions are **reproducible** and **traceable**.

### What This Means

- **Every operation is logged** with complete context (who, what, when, result)
- **Sessions group related operations** for logical organization
- **Audit trail is immutable** for compliance and verification
- **Operations can be replayed** deterministically for reproducibility
- **Replay attacks are prevented** through nonce-based protection

### Quick Example

```javascript
// Create a session
const sessionId = await contract.create_session(userAddress);

// Perform operations within the session
const attestationId = await contract.submit_attestation_with_session(
    sessionId,
    issuer,
    subject,
    timestamp,
    payloadHash,
    signature
);

// Verify session completeness
const operationCount = await contract.get_session_operation_count(sessionId);

// Retrieve audit logs
const auditLog = await contract.get_audit_log(0);
```

## Documentation

### Getting Started
- **[QUICK_START.md](./QUICK_START.md)** - Quick reference guide with examples
- **[CHANGELOG.md](./CHANGELOG.md)** - Version history and changes

### Feature Documentation
- **[docs/features/ANCHOR_INFO_DISCOVERY.md](./docs/features/ANCHOR_INFO_DISCOVERY.md)** - Anchor info discovery service (stellar.toml)
- **[docs/features/ANCHOR_ADAPTER.md](./docs/features/ANCHOR_ADAPTER.md)** - Unified anchor adapter interface
- **[docs/features/METADATA_CACHE.md](./docs/features/METADATA_CACHE.md)** - Metadata and capabilities caching
- **[docs/features/REQUEST_ID_PROPAGATION.md](./docs/features/REQUEST_ID_PROPAGATION.md)** - Request ID tracking and tracing
- **[docs/features/LOGGING.md](./docs/features/LOGGING.md)** - Logging system
- **[docs/features/DOMAIN_VALIDATION.md](./docs/features/DOMAIN_VALIDATION.md)** - Domain validation
- **[docs/features/ERROR_CODES_REFERENCE.md](./docs/features/ERROR_CODES_REFERENCE.md)** - API error codes reference
- **[docs/features/RETRY_BACKOFF.md](./docs/features/RETRY_BACKOFF.md)** - Retry and backoff strategies
- **[docs/features/WEBHOOK_MIDDLEWARE.md](./docs/features/WEBHOOK_MIDDLEWARE.md)** - Webhook middleware
- **[docs/features/WEBHOOK_MONITOR.md](./docs/features/WEBHOOK_MONITOR.md)** - Webhook monitoring
- **[docs/features/TRANSACTION_STATE_TRACKER.md](./docs/features/TRANSACTION_STATE_TRACKER.md)** - Transaction state tracking
- **[docs/features/SEP10_AUTH.md](./docs/features/SEP10_AUTH.md)** - SEP-10 authentication
- **[docs/features/SDK_CONFIG.md](./docs/features/SDK_CONFIG.md)** - SDK configuration
- **[docs/features/STATUS_MONITOR.md](./docs/features/STATUS_MONITOR.md)** - Status monitoring
- **[docs/features/ROUTING_STRATEGY.md](./docs/features/ROUTING_STRATEGY.md)** - Routing strategy selection (`route_transaction`)

### Guides
- **[docs/guides/DOCTOR_COMMAND.md](./docs/guides/DOCTOR_COMMAND.md)** - CLI doctor command and environment diagnostics
- **[docs/guides/CONTRIBUTING.md](./docs/guides/CONTRIBUTING.md)** - Contribution guidelines
- **[docs/guides/ERROR_IMPLEMENTATION_GUIDE.md](./docs/guides/ERROR_IMPLEMENTATION_GUIDE.md)** - Error handling implementation guide
- **[docs/guides/RETRY_QUICK_REFERENCE.md](./docs/guides/RETRY_QUICK_REFERENCE.md)** - Retry quick reference

### Full Index
See **[docs/README.md](./docs/README.md)** for the complete documentation index.

## New API Methods

### Session Management
- `create_session(initiator)` - Create new session
- `get_session(session_id)` - Get session details
- `get_session_operation_count(session_id)` - Get operation count
- `get_audit_log(log_id)` - Get audit log entry

### Session-Aware Operations
- `submit_attestation_with_session(...)` - Submit attestation with logging
- `register_attestor_with_session(...)` - Register attestor with logging
- `revoke_attestor_with_session(...)` - Revoke attestor with logging

## New Data Structures

- `InteractionSession` - Represents a session with metadata
- `OperationContext` - Captures operation details
- `AuditLog` - Complete audit entry

## New Events

- `SessionCreated` - Emitted when session is created
- `OperationLogged` - Emitted when operation is logged

## Platform Support

AnchorKit is designed to work seamlessly across all major platforms:

- ✅ **Linux** (Ubuntu, Debian, Fedora, etc.)
- ✅ **macOS** (Intel and Apple Silicon)
- ✅ **Windows** (10/11 with PowerShell)

### Cross-Platform Features

- **Path Handling**: All file operations use platform-agnostic APIs (`std::path::Path` in Rust, `pathlib.Path` in Python)
- **Scripts**: Both bash (Unix) and PowerShell (Windows) versions provided
- **Testing**: Comprehensive cross-platform test suite included
- **CI/CD**: Automated testing on Linux, macOS, and Windows

### Platform-Specific Setup

- **Linux/macOS**: See main setup instructions below
- **Windows**: See [WINDOWS_SETUP.md](./WINDOWS_SETUP.md) for detailed Windows-specific guide

## Building

### Linux/macOS

```bash
cargo build --release
```

### Windows

```powershell
cargo build --release
```

For detailed Windows setup instructions, including IDE configuration and troubleshooting, see [WINDOWS_SETUP.md](./WINDOWS_SETUP.md).

## CLI Usage

AnchorKit now includes a comprehensive CLI tool for interacting with the smart contract. Each command includes helpful examples and clear descriptions.

### Getting Help

View all available commands:
```bash
anchorkit --help
```

Get detailed help for any command:
```bash
anchorkit deploy --help
anchorkit register --help
```

### Common Workflows

#### 1. Build and Deploy
```bash
# Build the contract
anchorkit build --release

# Deploy to testnet
anchorkit deploy --network testnet

# Initialize with admin account
anchorkit init --admin GADMIN123...
```

#### 2. Register an Attestor
```bash
# Basic registration
anchorkit register --address GANCHOR123...

# Register with services
anchorkit register --address GANCHOR123... \
  --services deposits,withdrawals,kyc \
  --endpoint https://anchor.example.com
```

#### 3. Submit Attestations
```bash
# Submit attestation
anchorkit attest --subject GUSER123... --payload-hash abc123...

# Submit with session tracking
anchorkit attest --subject GUSER123... \
  --payload-hash abc123... \
  --session session-001
```

#### 4. Monitor Health
```bash
# Check all attestors
anchorkit health

# Monitor specific attestor
anchorkit health --attestor GANCHOR123... --watch --interval 30
```

### Available Commands

- `build` - Build the smart contract
- `deploy` - Deploy to Stellar network
- `init` - Initialize contract with admin
- `register` - Register new attestor
- `attest` - Submit attestation
- `query` - Query attestation by ID
- `health` - Check attestor health
- `test` - Run contract tests
- `validate` - Validate configuration files
- `doctor` - Run environment diagnostics

Each command includes:
- Clear description of when to use it
- Real-world usage examples
- All available options and flags
- Network selection support

### Environment Diagnostics

The `doctor` command helps troubleshoot environment setup issues:

```bash
# Check your development environment
anchorkit doctor
```

The doctor command checks:
- ✅ Rust toolchain installation
- ✅ WASM target availability
- ✅ Wallet configuration
- ✅ RPC endpoint connectivity
- ✅ Config file validity
- ✅ Network connectivity

See **[docs/guides/DOCTOR_COMMAND.md](./docs/guides/DOCTOR_COMMAND.md)** for complete documentation.

## Testing

The contract includes comprehensive tests for all functionality, including cross-platform compatibility:

### Linux/macOS
```bash
# Run all tests
cargo test

# Run cross-platform path tests
cargo test cross_platform

# Run with verbose output
cargo test --verbose
```

### Windows
```powershell
# Run all tests
cargo test

# Run cross-platform path tests
cargo test cross_platform

# Run with verbose output
cargo test --verbose
```

### Configuration Validation

#### Linux/macOS
```bash
# Validate all configurations
./validate_all.sh

# Pre-deployment validation
./pre_deploy_validate.sh
```

#### Windows
```powershell
# Validate all configurations
.\validate_all.ps1

# Pre-deployment validation
.\pre_deploy_validate.ps1
```

## Backward Compatibility

All existing methods remain unchanged. Session features are opt-in, allowing gradual adoption.

## Use Cases

### Compliance & Audit
- Complete audit trail for regulatory compliance
- Immutable operation records
- Actor tracking for accountability

### Reproducibility
- Deterministic operation replay
- Session-based operation grouping
- Complete context preservation

### Security
- Replay attack prevention
- Multi-level protection
- Nonce-based verification

## Architecture

AnchorKit consists of:

- **Core Contract** (`src/lib.rs`) - Main contract logic
- **Storage Layer** (`src/storage.rs`) - Persistent data management
- **Event System** (`src/events.rs`) - Event definitions and publishing
- **Type System** (`src/types.rs`) - Data structures
- **Error Handling** (`src/errors.rs`) - Error codes and definitions

## Security

- Stable error codes (100-120) for API compatibility
- Replay protection at multiple levels
- Immutable audit logs
- Authorization checks on all operations
- Complete operation context for verification

## Performance

- Efficient storage with TTL management
- Minimal event data
- Sequential IDs (no hash lookups)
- Optimized for Soroban constraints

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

For questions or issues:
1. Check the documentation files
2. Review the API specification
3. Examine the test cases in `src/lib.rs`

