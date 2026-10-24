# Anchor Info Discovery Service

The Anchor Info Discovery Service provides on-chain caching and validation of Stellar anchor metadata derived from `.well-known/stellar.toml` files.

> **Architecture Note**: AnchorKit runs inside Soroban WASM and does not perform outbound HTTP calls directly (as noted in [SEP24_INTERACTIVE.md](./SEP24_INTERACTIVE.md)). Off-chain callers fetch and parse the `stellar.toml` file, then submit the pre-parsed `StellarToml` struct to `fetch_anchor_info` to cache and validate it on-chain.

## Features

- **Validate and Cache stellar.toml**: Validate network passphrase, HTTPS transfer endpoints, and asset decimals, caching anchor metadata on-chain
- **Parse metadata**: Extract supported assets, fees, limits, and service endpoints off-chain
- **Cache with TTL**: Store parsed data in Soroban temporary storage with configurable time-to-live
- **Query capabilities**: Check asset support, fees, and limits programmatically on-chain

## Data Structures

### StellarToml

Complete representation of a stellar.toml file:

```rust
pub struct StellarToml {
    pub version: String,
    pub network_passphrase: String,
    pub accounts: Vec<String>,
    /// The SIGNING_KEY from stellar.toml, used for SEP-10 verification.
    /// `None` when the anchor does not publish a signing key.
    pub signing_key: Option<String>,
    pub currencies: Vec<AssetInfo>,
    /// Fiat currencies supported by this anchor (USD, EUR, etc.).
    pub fiat_currencies: Vec<FiatCurrency>,
    pub transfer_server: String,
    pub transfer_server_sep0024: String,
    pub kyc_server: String,
    pub web_auth_endpoint: String,
}
```

### AssetInfo

Detailed information about a supported asset:

```rust
pub struct AssetInfo {
    pub code: String,
    pub issuer: String,
    pub deposit_enabled: bool,
    pub withdrawal_enabled: bool,
    pub deposit_fee_fixed: u64,
    pub deposit_fee_percent: u32,
    pub withdrawal_fee_fixed: u64,
    pub withdrawal_fee_percent: u32,
    pub deposit_min_amount: u64,
    pub deposit_max_amount: u64,
    pub withdrawal_min_amount: u64,
    pub withdrawal_max_amount: u64,
    /// Number of decimal places for the asset (e.g. 7 for USDC on Stellar).
    /// Parsed from the `significant_decimals` field of stellar.toml; defaults to 7.
    pub decimals: u32,
}
```

## Contract Methods

### Fetch and Cache

```rust
pub fn fetch_anchor_info(
    env: Env,
    anchor: Address,
    toml_data: StellarToml,
    network_passphrase: String,
    ttl_override: Option<u64>,
)
```

Stores and caches pre-parsed `stellar.toml` metadata on-chain for the anchor after validating the network passphrase, HTTPS transfer server domain, and currency decimals. Requires anchor authorization (`anchor.require_auth()`).

**Parameters:**
- `anchor`: Address of the anchor (must authorize the transaction)
- `toml_data`: Pre-parsed `StellarToml` structure containing anchor metadata
- `network_passphrase`: Stellar network passphrase (must match the active network and `toml_data.network_passphrase`)
- `ttl_override`: Optional cache TTL in seconds (defaults to 3600 seconds if `None`, capped at `MAX_TTL_SECONDS`)

**Returns:**
- Returns `()` on success. Panics with `ErrorCode` on validation failure (e.g. `ErrorCode::ValidationError`, `ErrorCode::InvalidEndpointFormat`).

**Example:**
```rust
// Caller fetches/parses stellar.toml off-chain, then submits to contract:
contract.fetch_anchor_info(
    &anchor_addr,
    &toml_data,
    &String::from_str(&env, "Test SDF Network ; September 2015"),
    &Some(7200),
);
```

### Get Cached TOML

```rust
pub fn get_anchor_toml(
    env: Env,
    anchor: Address,
) -> Result<StellarToml, ErrorCode>
```

Retrieves cached `StellarToml` for an anchor.

**Returns:**
- `Ok(StellarToml)` if found and unexpired
- `Err(ErrorCode::CacheNotFound)` if no TOML has been cached for this anchor
- `Err(ErrorCode::CacheExpired)` if the cached entry has expired

**Example:**
```rust
let toml = contract.get_anchor_toml(&anchor_addr)?;
println!("Version: {}", toml.version);
```

### Refresh Cache

```rust
pub fn refresh_anchor_info(
    env: Env,
    anchor: Address,
    force: bool,
)
```

Refreshes or clears cached metadata for an anchor. Requires anchor authorization (`anchor.require_auth()`).

**Parameters:**
- `anchor`: Address of the anchor (must authorize the transaction)
- `force`: If `true`, removes the cached TOML immediately; if `false`, removes it only if expired.

**Example:**
```rust
// Evict expired cache or force invalidation
contract.refresh_anchor_info(&anchor_addr, &true);
```

### Query Supported Assets

```rust
pub fn get_anchor_assets(
    env: Env,
    anchor: Address,
) -> Result<Vec<String>, ErrorCode>
```

Returns list of asset codes supported by the anchor.

**Example:**
```rust
let assets = contract.get_anchor_assets(&anchor_addr)?;
// Returns: ["USDC", "XLM", "BTC"]
```

### Query Supported Fiat Currencies

```rust
pub fn get_anchor_currencies(
    env: Env,
    anchor: Address,
) -> Result<Vec<FiatCurrency>, ErrorCode>
```

Returns list of fiat currencies supported by the anchor.

### Get Asset Details

```rust
pub fn get_anchor_asset_info(
    env: Env,
    anchor: Address,
    asset_code: String,
) -> Result<AssetInfo, ErrorCode>
```

Retrieves complete information about a specific asset.

**Example:**
```rust
let usdc = String::from_str(&env, "USDC");
let info = contract.get_anchor_asset_info(&anchor_addr, &usdc)?;
println!("Issuer: {}", info.issuer);
println!("Deposit enabled: {}", info.deposit_enabled);
```

### Query Limits

```rust
// Deposit limits
pub fn get_anchor_deposit_limits(
    env: Env,
    anchor: Address,
    asset_code: String,
) -> Result<(u64, u64), ErrorCode>

// Withdrawal limits
pub fn get_anchor_withdrawal_limits(
    env: Env,
    anchor: Address,
    asset_code: String,
) -> Result<(u64, u64), ErrorCode>
```

Returns `(min, max)` limits for deposits or withdrawals.

**Example:**
```rust
let usdc = String::from_str(&env, "USDC");
let (min, max) = contract.get_anchor_deposit_limits(&anchor_addr, &usdc)?;
println!("Deposit range: {} - {}", min, max);
```

### Query Fees

```rust
// Deposit fees
pub fn get_anchor_deposit_fees(
    env: Env,
    anchor: Address,
    asset_code: String,
) -> Result<(u64, u32), ErrorCode>

// Withdrawal fees
pub fn get_anchor_withdrawal_fees(
    env: Env,
    anchor: Address,
    asset_code: String,
) -> Result<(u64, u32), ErrorCode>
```

Returns `(fixed_fee, percent_fee)` for deposits or withdrawals.

**Example:**
```rust
let usdc = String::from_str(&env, "USDC");
let (fixed, percent) = contract.get_anchor_deposit_fees(&anchor_addr, &usdc)?;
println!("Fee: {} + {}%", fixed, percent);
```

### Check Service Support

```rust
// Check deposit support
pub fn anchor_supports_deposits(
    env: Env,
    anchor: Address,
    asset_code: String,
) -> Result<bool, ErrorCode>

// Check withdrawal support
pub fn anchor_supports_withdrawals(
    env: Env,
    anchor: Address,
    asset_code: String,
) -> Result<bool, ErrorCode>
```

**Example:**
```rust
let usdc = String::from_str(&env, "USDC");
if contract.anchor_supports_deposits(&anchor_addr, &usdc)? {
    println!("Deposits supported for USDC");
}
```

## Usage Examples

### Complete Workflow

```rust
use soroban_sdk::{Env, String, Address};

// 1. Fetch & parse stellar.toml off-chain, then cache on-chain
let network_passphrase = String::from_str(&env, "Test SDF Network ; September 2015");
contract.fetch_anchor_info(&anchor, &toml_data, &network_passphrase, &None);

// 2. List supported assets
let assets = contract.get_anchor_assets(&anchor)?;
for asset in assets.iter() {
    println!("Supported: {}", asset);
}

// 3. Check specific asset details
let usdc = String::from_str(&env, "USDC");
let info = contract.get_anchor_asset_info(&anchor, &usdc)?;

// 4. Validate transaction parameters
let (min, max) = contract.get_anchor_deposit_limits(&anchor, &usdc)?;
let amount = 5000;
if amount >= min && amount <= max {
    // Proceed with deposit
}

// 5. Calculate fees
let (fixed, percent) = contract.get_anchor_deposit_fees(&anchor, &usdc)?;
let total_fee = fixed + (amount * percent as u64 / 10000);
```

## Cache Management

### Default TTL

The default cache TTL is 3600 seconds (1 hour). This can be customized per fetch:

```rust
// Cache for 2 hours
contract.fetch_anchor_info(&anchor, &toml_data, &network_passphrase, &Some(7200));

// Cache for 30 minutes
contract.fetch_anchor_info(&anchor, &toml_data, &network_passphrase, &Some(1800));
```

### Manual Refresh / Invalidation

Force cache eviction when anchor metadata changes:

```rust
contract.refresh_anchor_info(&anchor, &true);
```

### Cache Expiration Handling

When cache expires, queries return `ErrorCode::CacheExpired`. Handle this by re-submitting fresh metadata:

```rust
match contract.get_anchor_toml(&anchor) {
    Ok(toml) => {
        // Use cached data
    }
    Err(ErrorCode::CacheExpired) => {
        // Re-fetch off-chain and update contract cache
        contract.fetch_anchor_info(&anchor, &fresh_toml, &network_passphrase, &None);
    }
    Err(e) => return Err(e),
}
```

## Error Handling

### Common Error Codes

- `ErrorCode::CacheNotFound`: No cached data for anchor (call `fetch_anchor_info` first)
- `ErrorCode::CacheExpired`: Cached data expired (call `fetch_anchor_info` with refreshed metadata)
- `ErrorCode::ValidationError`: Network passphrase mismatch, invalid passphrase length, or currency decimals > 18
- `ErrorCode::InvalidEndpointFormat`: Transfer server URL is invalid or not HTTPS

## Off-Chain Integration Architecture

AnchorKit runs inside Soroban WASM and cannot make outbound HTTP requests. Discovery operates in two stages:

1. **Off-Chain Client/SDK**:
   - Queries `https://<domain>/.well-known/stellar.toml` via HTTP
   - Parses the TOML content into the `StellarToml` struct
2. **On-Chain AnchorKit Contract**:
   - Accepts pre-parsed `StellarToml` in `fetch_anchor_info`
   - Verifies caller authority (`anchor.require_auth()`)
   - Validates that `network_passphrase` matches the known Stellar network (Mainnet or Testnet) and matches `toml_data.network_passphrase`
   - Verifies `transfer_server` endpoint via domain validation
   - Validates decimals for all assets
   - Stores metadata in Soroban temporary storage with TTL

## API Summary

| Method | Auth Required | Returns | Purpose |
|--------|---------------|---------|---------|
| `fetch_anchor_info` | Anchor | `()` | Validate and cache pre-parsed TOML on-chain |
| `get_anchor_toml` | None | `Result<StellarToml, ErrorCode>` | Get cached TOML |
| `refresh_anchor_info` | Anchor | `()` | Refresh or invalidate cached TOML |
| `get_anchor_assets` | None | `Result<Vec<String>, ErrorCode>` | List supported asset codes |
| `get_anchor_currencies` | None | `Result<Vec<FiatCurrency>, ErrorCode>` | List supported fiat currencies |
| `get_anchor_asset_info` | None | `Result<AssetInfo, ErrorCode>` | Asset details |
| `get_anchor_deposit_limits` | None | `Result<(u64, u64), ErrorCode>` | Deposit min/max |
| `get_anchor_withdrawal_limits` | None | `Result<(u64, u64), ErrorCode>` | Withdrawal min/max |
| `get_anchor_deposit_fees` | None | `Result<(u64, u32), ErrorCode>` | Deposit fees |
| `get_anchor_withdrawal_fees` | None | `Result<(u64, u32), ErrorCode>` | Withdrawal fees |
| `anchor_supports_deposits` | None | `Result<bool, ErrorCode>` | Check deposit support |
| `anchor_supports_withdrawals` | None | `Result<bool, ErrorCode>` | Check withdrawal support |

## Security

- **Anchor Authorization**: `fetch_anchor_info` and `refresh_anchor_info` require `anchor.require_auth()`
- **Network Passphrase Validation**: Ensures the TOML belongs to the correct Stellar network before caching
- **HTTPS Enforcement**: Transfer server endpoint is validated to ensure HTTPS-only communication
- **Asset Decimal Bounds**: Enforces that asset decimals do not exceed 18
