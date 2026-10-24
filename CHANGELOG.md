# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Anchor Health Score**: New `get_anchor_health_score` function that computes a 0-100 health score from cached metadata, combining uptime (40%), reputation (35%), and settlement speed (25%) into a single metric
- Comprehensive test suite for health score calculation with 12 test cases covering various scenarios
- Feature documentation for anchor health score (docs/features/ANCHOR_HEALTH_SCORE.md)
- Quick reference guide for health score usage (docs/guides/HEALTH_SCORE_QUICK_REF.md)
- Example script demonstrating health score evaluation (examples/health_score_example.sh)
- Comprehensive CONTRIBUTING.md with contributor guidelines
- MIT LICENSE file at repository root
- Unit tests for domain_validator edge cases (Unicode/IDN, IP addresses, trailing slashes, length boundaries)
- License field to Cargo.toml

### Fixed
<!-- Bug fixes go here -->

### Breaking Changes
<!-- Breaking changes go here -->

### Changed
- Updated README.md license section to reference LICENSE file

## [0.1.0] - 2024-01-01

### Added
- **Attestation Management**: Register attestors, submit and retrieve attestations with replay attack protection
- **Endpoint Configuration**: Manage attestor endpoints for off-chain integration
- **Unified Anchor Adapter**: Consistent API for multiple anchor integrations
- **Session Management**: Group operations into logical sessions for traceability
- **Audit Trail**: Complete immutable record of all operations
- **Reproducibility**: Deterministic operation replay for verification
- **Replay Protection**: Multi-level protection against unauthorized replays
- **Secure Credential Management**: Runtime credential injection with automatic rotation
- **Domain Validation**: HTTPS-only domain validation with comprehensive edge case handling
- **Transaction State Tracker**: Track and manage transaction states
- **Response Validator**: Validate anchor responses with error handling
- **Service Capability Discovery**: Discover anchor services (deposits, withdrawals, quotes, KYC)
- **Anchor Info Discovery**: Fetch and parse stellar.toml, cache assets/fees/limits
- **Health Monitoring**: Monitor anchor latency, failures, and availability
- **Metadata Caching**: TTL-based caching with manual refresh
- **Request ID Propagation**: UUID per flow with tracing
- **Event Emission**: Emit events for all state changes
- **Comprehensive Error Handling**: Stable error codes (1-19, 26, 48-64, 121-123) for API compatibility
- **SEP-10 JWT Authentication**: Secure JWT-based authentication for Stellar protocol compliance
- **Webhook Monitoring**: Real-time monitoring and tracking of webhook delivery and processing
- **Rate Limiting**: Configurable rate limiting to protect services from overload
- **Retry/Backoff Mechanisms**: Intelligent retry strategies with exponential backoff for resilience
- **Response Validation**: Validate responses against expected schemas and formats
- **Routing Strategy Selection**: Flexible routing strategies for anchor selection and failover
- **CLI Tool**: Command-line interface for contract interaction
- **Cross-Platform Support**: Linux, macOS, and Windows compatibility
- **Configuration Validation**: Validate configuration files with JSON schema
- **Environment Diagnostics**: Doctor command for troubleshooting setup issues

### Security
- Stable error codes for API compatibility
- Replay protection at multiple levels
- Immutable audit logs
- Authorization checks on all operations
- Complete operation context for verification

### Performance
- Efficient storage with TTL management
- Minimal event data
- Sequential IDs (no hash lookups)
- Optimized for Soroban constraints

## [0.0.1] - 2023-12-01

### Added
- Initial project structure
- Basic contract skeleton
- Core type definitions
- Error handling framework
