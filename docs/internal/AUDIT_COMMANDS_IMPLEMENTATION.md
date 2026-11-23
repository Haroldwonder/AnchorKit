# Audit Commands Implementation Guide

This document provides technical implementation details for integrating `anchorkit audit get` and `anchorkit audit list` commands with the Soroban on-chain contract.

## Architecture

### Command Structure

```
anchorkit audit get <LOG_ID>
  └─ run_audit_get(log_id)
     ├─ fetch_audit_log_entry(log_id)
     │  └─ RPC call: contract.get_audit_log(log_id)
     └─ print_audit_entry(entry)

anchorkit audit list --session <ID> [--from N] [--to M] [--format F] [--pretty]
  └─ run_audit_list(session, from, to, format, pretty)
     ├─ fetch_audit_logs_by_session(session, from, to)
     │  └─ Loop: RPC call: contract.get_audit_log_range(from_id, to_id)
     │     └─ Client-side filter: keep entries where session_id == target
     └─ Format output (text/json/csv)
```

## Implementation Tasks

### Phase 1: RPC Client Infrastructure

**File**: `src/lib.rs` or new `src/rpc.rs`

Create RPC client wrapper:

```rust
/// RPC client for Soroban contract interaction
pub struct SorobanClient {
    rpc_url: String,
    wallet: StellarWallet,
    contract_id: Address,
}

impl SorobanClient {
    pub fn new(rpc_url: &str, wallet: StellarWallet) -> Result<Self> {
        Ok(SorobanClient {
            rpc_url: rpc_url.to_string(),
            wallet,
            contract_id: load_contract_id()?,
        })
    }

    /// Invoke contract method and return result
    pub async fn invoke_contract<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        args: &[SorobanContractArg],
    ) -> Result<T> {
        // 1. Build transaction envelope
        // 2. Sign with wallet
        // 3. POST to RPC /simulateTransaction
        // 4. Parse result
        // 5. Return deserialized value
    }
}

pub struct StellarWallet {
    secret_key: String,
}

impl StellarWallet {
    pub fn from_env() -> Result<Self> {
        // Load from STELLAR_SECRET_KEY or SOROBAN_IDENTITY
    }

    pub fn sign(&self, tx_envelope: &TransactionEnvelope) -> Signature {
        // Sign using ed25519-dalek
    }
}
```

### Phase 2: Contract Method Wrappers

**File**: `src/contract_client.rs` or integrate into `src/rpc.rs`

Create typed wrappers for contract methods:

```rust
pub struct AuditContractClient {
    client: SorobanClient,
}

impl AuditContractClient {
    pub async fn get_audit_log(&self, log_id: u64) -> Result<AuditLog> {
        self.client.invoke_contract(
            "get_audit_log",
            &[SorobanContractArg::U64(log_id)],
        ).await
    }

    pub async fn get_audit_log_range(
        &self,
        from_id: u64,
        to_id: u64,
    ) -> Result<Vec<AuditLog>> {
        self.client.invoke_contract(
            "get_audit_log_range",
            &[
                SorobanContractArg::U64(from_id),
                SorobanContractArg::U64(to_id),
            ],
        ).await
    }
}
```

### Phase 3: Soroban RPC Integration

**Resources**:
- Soroban RPC Spec: https://soroban.stellar.org/docs/learn/interacting
- RPC Methods: POST `/simulateTransaction`

**Request Structure**:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "simulateTransaction",
  "params": {
    "transaction": "<base64-encoded TransactionEnvelope>",
    "resourceLeeway": 15
  }
}
```

**Implementation in Rust**:

```rust
use reqwest::Client;
use soroban_sdk::xdr::{TransactionEnvelope, SorobanTransactionData};

pub async fn simulate_transaction(
    rpc_url: &str,
    tx_envelope: &TransactionEnvelope,
) -> Result<SimulateTransactionResponse> {
    let client = Client::new();
    
    let envelope_xdr = tx_envelope.to_xdr(soroban_sdk::xdr::Limits::none())?;
    let envelope_b64 = base64::encode(&envelope_xdr);
    
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "simulateTransaction",
        "params": {
            "transaction": envelope_b64,
            "resourceLeeway": 15
        }
    });
    
    let response: SimulateTransactionResponse = client
        .post(rpc_url)
        .json(&request)
        .send()
        .await?
        .json()
        .await?;
    
    Ok(response)
}
```

### Phase 4: Transaction Building

**Key Steps**:

1. **Get Latest Ledger**:
   ```rust
   pub async fn get_latest_ledger(rpc_url: &str) -> Result<u32> {
       let response: GetLatestLedgerResponse = rpc_call(rpc_url, "getLedgerEntries", json!({})).await?;
       Ok(response.sequence)
   }
   ```

2. **Build Contract Invocation**:
   ```rust
   pub fn build_contract_invoke(
       contract_id: &Address,
       method: &str,
       args: &[ScVal],
       source_account: &Account,
       fee: u32,
   ) -> Transaction {
       let op = InvokeHostFunction {
           host_function: HostFunction::InvokeContract(InvokeContractArgs {
               contract_address: contract_id.clone(),
               function_name: ScSymbol(method.as_bytes().to_vec()),
               args: args.to_vec(),
           }),
           auth: vec![],  // No auth required for read-only queries
       };
       
       // Build transaction with operation
       // Sign with wallet
   }
   ```

3. **Extend Resource Fees**:
   ```rust
   pub async fn prepare_transaction_for_simulation(
       tx: &mut Transaction,
       rpc_url: &str,
   ) -> Result<()> {
       // Call getTransactionBuilder
       // Set fees and resource requirements
   }
   ```

### Phase 5: CLI Integration (Existing)

**File**: `src/bin/anchorkit.rs`

Already implemented:
- `run_audit_get(log_id)` - Entry point
- `run_audit_list(...)` - Entry point
- `fetch_audit_log_entry()` - Placeholder
- `fetch_audit_logs_by_session()` - Placeholder
- Formatting functions

**Next Steps**: Replace placeholders with actual RPC calls

### Phase 6: Error Handling

Create audit-specific error types:

```rust
#[derive(Debug)]
pub enum AuditError {
    RpcError(String),
    NotFound(u64),
    ContractError(String),
    InvalidSession(u64),
    NetworkError(String),
    InvalidFormat(String),
    ParseError(String),
}

impl std::fmt::Display for AuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditError::NotFound(log_id) => write!(f, "audit log entry {} not found", log_id),
            AuditError::InvalidSession(session_id) => write!(f, "invalid session {}", session_id),
            // ...
        }
    }
}

impl std::error::Error for AuditError {}
```

### Phase 7: Testing Strategy

Unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_audit_log_entry() {
        // Mock RPC response and verify parsing
    }

    #[test]
    fn test_session_filtering() {
        // Verify session_id filtering logic
    }

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(86400), "1d 00h 00m 00s");
    }

    #[test]
    fn test_csv_escaping() {
        // Verify CSV quote escaping
    }

    #[tokio::test]
    async fn test_audit_get_integration() {
        // Integration test with mock RPC server
    }

    #[tokio::test]
    async fn test_audit_list_pagination() {
        // Test pagination logic
    }
}
```

Integration tests:

```bash
# Before running, ensure testnet RPC is available
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org:443 \
  cargo test --test audit_integration -- --nocapture
```

## Environment Configuration

### Required Environment Variables

```bash
# RPC endpoint (required)
export ANCHORKIT_RPC_URL=https://soroban-testnet.stellar.org:443

# Wallet (one of the following)
export STELLAR_SECRET_KEY=SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
# OR
export SOROBAN_IDENTITY=default  # Uses ~/.config/soroban/identity/{name}
```

### Optional Configuration

```bash
# Contract ID (can be loaded from Cargo.toml or hardcoded)
export ANCHORKIT_CONTRACT_ID=CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX

# RPC request timeout (seconds)
export ANCHORKIT_RPC_TIMEOUT=30

# Pagination size (max 100)
export ANCHORKIT_AUDIT_PAGE_SIZE=100

# Cache audit logs locally (path)
export ANCHORKIT_AUDIT_CACHE=~/.anchorkit/audit_cache.json
```

## Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
# Existing
soroban-sdk = "21.7.0"
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# New for RPC integration
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
tokio = { version = "1.35", features = ["full"] }
base64 = "0.22"
ed25519-dalek = "2"
thiserror = "1.0"
log = "0.4"
env_logger = "0.11"

[dev-dependencies]
mockito = "1.2"  # Mock HTTP for testing
```

## Phased Rollout

### MVP (Phase 1-3)

- ✅ CLI structure (already implemented)
- [ ] Basic RPC client
- [ ] Contract method wrappers
- [ ] Minimal error handling

**Status**: Ready for implementation

### V1.0 (Phase 4-5)

- [ ] Transaction building
- [ ] Full error handling
- [ ] CLI integration
- [ ] Basic testing

**Timeline**: 2-3 weeks

### V1.1 (Phase 6-7)

- [ ] Comprehensive test coverage
- [ ] Integration testing
- [ ] Performance optimization
- [ ] Documentation

**Timeline**: 1-2 weeks

## Testing Against Testnet

### Setup

```bash
# 1. Get testnet RPC
export SOROBAN_RPC_URL=https://soroban-testnet.stellar.org:443

# 2. Create testnet identity
soroban config identity generate --global my-audit-test

# 3. Fund account (via friendbot or testnet faucet)
curl "https://friendbot.stellar.org?addr=GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"

# 4. Deploy test contract
soroban contract deploy --wasm wasm/anchorkit.wasm ...
```

### Run Commands

```bash
# Single entry
SOROBAN_RPC_URL=... cargo run --bin anchorkit -- audit get 42

# List session
SOROBAN_RPC_URL=... cargo run --bin anchorkit -- audit list --session 5

# Export to file
SOROBAN_RPC_URL=... cargo run --bin anchorkit -- audit list --session 5 --format json > audit.json
```

## Debugging

Enable logging:

```bash
RUST_LOG=debug SOROBAN_RPC_URL=... cargo run --bin anchorkit -- audit list --session 5
```

Common issues:

1. **"Contract not found"**: Verify contract ID matches RPC network
2. **"Invalid signature"**: Check wallet credentials and network match
3. **"Timeout"**: Increase `ANCHORKIT_RPC_TIMEOUT` or check RPC endpoint
4. **"Not Found"**: Log ID may be pruned or invalid

## Performance Optimization

### Caching

```rust
pub struct AuditCache {
    entries: HashMap<u64, AuditLogEntry>,
    session_ranges: HashMap<u64, (u64, u64)>,  // session -> (min_id, max_id)
    ttl: Duration,
}

impl AuditCache {
    pub fn get(&self, log_id: u64) -> Option<&AuditLogEntry> {
        if self.is_expired() {
            return None;
        }
        self.entries.get(&log_id)
    }
}
```

### Parallel Requests

```rust
pub async fn fetch_audit_logs_parallel(
    client: &AuditContractClient,
    ranges: Vec<(u64, u64)>,  // [(from1, to1), (from2, to2), ...]
) -> Result<Vec<AuditLog>> {
    let futures: Vec<_> = ranges
        .into_iter()
        .map(|(from, to)| client.get_audit_log_range(from, to))
        .collect();
    
    let results = futures::future::try_join_all(futures).await?;
    Ok(results.into_iter().flatten().collect())
}
```

## Security Considerations

1. **Wallet Security**:
   - Never log secret keys
   - Use environment variables, not CLI args
   - Consider hardware wallet integration

2. **RPC Validation**:
   - Validate RPC URL scheme (HTTPS only for production)
   - Certificate validation for TLS
   - Timeout to prevent hanging

3. **Data Validation**:
   - Verify returned data matches contract schema
   - Check signatures on critical operations
   - Sanitize user input before RPC calls

4. **Rate Limiting**:
   - Respect RPC rate limits
   - Implement exponential backoff
   - Cache results locally

## Future Enhancements

1. **Audit Subscribe**: Real-time audit log notifications
2. **Audit Analytics**: Aggregation and trend analysis
3. **Audit Verify**: Merkle proof verification
4. **Audit Retention**: Archive old logs to IPFS/S3
5. **Audit Dashboard**: Web UI for visualization
