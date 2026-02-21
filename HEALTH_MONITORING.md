# Anchor Health Monitoring

AnchorKit now includes a health monitoring interface that tracks the operational status of registered anchors.

## Features

- **Latency Tracking**: Monitor response times in milliseconds
- **Failure Counting**: Track failed operations
- **Availability Metrics**: Percentage-based availability (0-100.00%)

## Data Structure

```rust
pub struct HealthStatus {
    pub anchor: Address,
    pub latency_ms: u64,
    pub failure_count: u32,
    pub availability_percent: u32, // 0-10000 (100.00%)
    pub last_check: u64,
}
```

## API Methods

### Update Health Status

Update the health metrics for an anchor. Only callable by the anchor itself.

```rust
pub fn update_health_status(
    env: Env,
    anchor: Address,
    latency_ms: u64,
    failure_count: u32,
    availability_percent: u32,
) -> Result<(), Error>
```

**Parameters:**
- `anchor`: The anchor's address
- `latency_ms`: Current latency in milliseconds
- `failure_count`: Total number of failures
- `availability_percent`: Availability as 0-10000 (e.g., 9950 = 99.50%)

**Errors:**
- `AttestorNotRegistered`: Anchor must be registered first
- `InvalidAnchorMetadata`: Availability percent exceeds 10000

### Get Health Status

Retrieve the current health status for an anchor.

```rust
pub fn get_health_status(env: Env, anchor: Address) -> Option<HealthStatus>
```

Returns `None` if no health data has been recorded for the anchor.

## Usage Example

```javascript
// Register an anchor first
await contract.register_attestor(anchorAddress);

// Update health metrics
await contract.update_health_status(
    anchorAddress,
    150,      // 150ms latency
    5,        // 5 failures
    9950      // 99.50% availability
);

// Query health status
const health = await contract.get_health_status(anchorAddress);
console.log(`Latency: ${health.latency_ms}ms`);
console.log(`Failures: ${health.failure_count}`);
console.log(`Availability: ${health.availability_percent / 100}%`);
console.log(`Last Check: ${health.last_check}`);
```

## Integration with Routing

Health status can be used alongside anchor metadata for intelligent routing decisions:

```javascript
// Get all anchors
const anchors = await contract.get_all_anchors();

// Filter by health metrics
const healthyAnchors = [];
for (const anchor of anchors) {
    const health = await contract.get_health_status(anchor);
    if (health && health.availability_percent > 9500 && health.latency_ms < 200) {
        healthyAnchors.push(anchor);
    }
}

// Route to the healthiest anchor
const routingResult = await contract.route_transaction({
    source_asset: "USDC",
    destination_asset: "BRL",
    amount: 1000,
    strategy: { BestRate: {} }
});
```

## Monitoring Best Practices

1. **Regular Updates**: Update health status periodically (e.g., every 5 minutes)
2. **Realistic Metrics**: Use actual operational data, not estimates
3. **Failure Tracking**: Increment failure count for any operation that doesn't complete successfully
4. **Latency Measurement**: Measure end-to-end response time for typical operations
5. **Availability Calculation**: Base on uptime over a rolling window (e.g., last 24 hours)

## Storage

Health status is stored in persistent storage with automatic TTL management. Each anchor's health data is stored independently and can be updated without affecting other anchors.
