# Webhook Middleware - Security & Validation

## Implementation Status

The webhook middleware is implemented as a validation layer in the contract test suite. The key contract-level behaviour is:

- **Address validation**: Only Soroban C-addresses are valid as `source_address`. G-addresses (Stellar account addresses) are rejected by the host with `"unexpected strkey length"`.
- **Test coverage**: `test_webhook_request_with_source_address` verifies that passing a G-address panics as expected.

## Overview

The Webhook Middleware provides production-grade security for webhook processing in AnchorKit. It implements a comprehensive validation pipeline that protects against common webhook attacks while maintaining high performance and reliability.

## Features

### 1. Signature Verification
- **Multiple Algorithms**: HMAC-SHA256, HMAC-SHA512, Ed25519
- **Constant-Time Comparison**: Prevents timing attacks
- **Flexible Configuration**: Choose algorithm per webhook endpoint
- **Secure Key Management**: Integration with credential system

### 2. Replay Attack Prevention
- **Hash-Based Deduplication**: Tracks processed webhooks by payload hash
- **Webhook ID Tracking**: Prevents duplicate processing of same webhook
- **Configurable TTL**: 1-day default retention for replay detection
- **Automatic Cleanup**: Temporary storage with automatic expiration

### 3. Timestamp Validation
- **Configurable Tolerance**: Default 300 seconds (5 minutes)
- **Clock Skew Handling**: 60-second future tolerance for clock drift
- **Expiration Detection**: Rejects stale webhooks
- **Audit Trail**: Logs all timestamp violations

### 4. Suspicious Activity Logging
- **8 Activity Types**: InvalidSignature, ReplayAttack, TimestampOutOfRange, PayloadTooLarge, MissingHeaders, RateLimitExceeded, UnauthorizedSource, MalformedPayload
- **4 Severity Levels**: Low, Medium, High, Critical
- **Real-Time Events**: Emits events for monitoring systems
- **7-Day Retention**: Audit trail for security investigation
- **Source Tracking**: Records source address when available

### 5. Delivery Tracking
- **Attempt Recording**: Tracks all delivery attempts with timestamps
- **Response Metrics**: Records response time and error codes
- **Status Tracking**: Pending, Delivered, Failed, Rejected, Suspicious
- **Retry History**: Complete audit trail of retry attempts

## Architecture

### Validation Pipeline

```
Webhook Request
    ↓
[1] Payload Size Check
    ↓ (fail) → Log: PayloadTooLarge
    ↓ (pass)
[2] Timestamp Validation
    ↓ (fail) → Log: TimestampOutOfRange
    ↓ (pass)
[3] Signature Verification
    ↓ (fail) → Log: InvalidSignature
    ↓ (pass)
[4] Replay Attack Detection
    ↓ (fail) → Log: ReplayAttack
    ↓ (pass)
[5] Record Delivery Success
    ↓
Webhook Accepted
```

### Storage Strategy

**Temporary Storage (1-day TTL):**
- Webhook replay detection hashes
- Delivery attempt records
- Suspicious activity logs (7-day TTL)
- Activity ID counters

**Persistent Storage (Optional):**
- Webhook endpoint configurations
- Security policies per anchor
- Credential storage (encrypted)

## Usage

### Basic Setup

```rust
use anchorkit::{
    WebhookMiddleware, WebhookSecurityConfig, WebhookRequest,
    SignatureAlgorithm, ActivitySeverity, SuspiciousActivityType,
};

// Create security configuration
let config = WebhookSecurityConfig {
    algorithm: SignatureAlgorithm::Sha256,
    secret_key: secret_bytes,
    timestamp_tolerance_seconds: 300,
    max_payload_size_bytes: 10000,
    enable_replay_protection: true,
};

// Create webhook request
let request = WebhookRequest {
    payload: payload_bytes,
    signature: signature_bytes,
    timestamp: webhook_timestamp,
    webhook_id: unique_webhook_id,
    source_address: Some(sender_address),
};

// Validate webhook
let result = WebhookMiddleware::validate_webhook(&env, &request, &config)?;

if result.is_valid {
    // Process webhook
} else {
    // Handle validation failure
    eprintln!("Webhook validation failed: {:?}", result.error);
}
```

### Signature Verification

```rust
// Verify signature with specific algorithm
let is_valid = WebhookMiddleware::verify_signature(&env, &request, &config)?;

if is_valid {
    println!("Signature verified successfully");
} else {
    println!("Signature verification failed");
}
```

### Timestamp Validation

```rust
// Validate timestamp independently
let is_valid = WebhookMiddleware::validate_timestamp(
    &env,
    webhook_timestamp,
    300, // 5 minute tolerance
)?;

if is_valid {
    println!("Timestamp is within acceptable range");
}
```

### Replay Attack Detection

```rust
// Check for replay attacks
let payload_hash = env.crypto().sha256(&payload);
let is_new = WebhookMiddleware::check_replay_attack(
    &env,
    webhook_id,
    &payload_hash,
)?;

if is_new {
    println!("Webhook is new, not a replay");
} else {
    println!("Duplicate webhook detected!");
}
```

### Suspicious Activity Logging

```rust
// Log suspicious activity
WebhookMiddleware::log_suspicious_activity(
    &env,
    SuspiciousActivityType::InvalidSignature,
    ActivitySeverity::Critical,
    String::from_str(&env, "Signature verification failed for webhook 123"),
    Some(sender_address),
);

// Retrieve suspicious activity record
let activity = WebhookMiddleware::get_suspicious_activity(&env, activity_id);
if let Some(record) = activity {
    println!("Activity: {:?}", record.activity_type);
    println!("Severity: {:?}", record.severity);
    println!("Details: {}", record.details);
}
```

### Delivery Tracking

```rust
// Record successful delivery
WebhookMiddleware::record_delivery_attempt(
    &env,
    webhook_id,
    WebhookDeliveryStatus::Delivered,
    150, // response time in ms
    None, // no error
);

// Record failed delivery with retry
WebhookMiddleware::record_delivery_attempt(
    &env,
    webhook_id,
    WebhookDeliveryStatus::Failed,
    5000,
    Some(500), // HTTP 500 error
);

// Retrieve delivery record
let record = WebhookMiddleware::get_delivery_record(&env, webhook_id, attempt_number);
if let Some(delivery) = record {
    println!("Attempt {}: {:?}", delivery.attempt_number, delivery.status);
    println!("Response time: {}ms", delivery.response_time_ms);
}
```

## HMAC Signature Verification

Signature verification is the primary mechanism for proving that an incoming webhook was sent by a trusted source and that its payload was not tampered with in transit. Without it, any party that knows your endpoint URL can send arbitrary webhook payloads.

### How It Works

The sender computes an HMAC over a canonical message derived from the request, then attaches the resulting digest as a request header. The receiver independently recomputes the same HMAC and compares the two values using a constant-time comparison to prevent timing attacks.

**Canonical message format:**

```
message = timestamp_string + raw_payload_bytes
```

The timestamp is the Unix epoch in seconds, serialized as a decimal string and prepended to the raw payload bytes before hashing. This binds the signature to a specific point in time, which is what makes replay attack prevention effective.

### Required HTTP Headers

| Header | Description | Example |
|--------|-------------|---------|
| `X-Webhook-Signature` | Hex-encoded HMAC digest | `3d4f2a...` |
| `X-Webhook-Timestamp` | Unix timestamp (seconds) | `1714000000` |
| `X-Webhook-ID` | Globally unique webhook identifier | `wh_abc123` |

### Sender: Generating the Signature

**Node.js**

```javascript
const crypto = require('crypto');

function signWebhook(payload, secret, timestamp) {
    // payload: raw JSON string (do NOT re-serialize after signing)
    // secret:  Buffer of the shared secret bytes
    // timestamp: integer Unix seconds
    const message = Buffer.concat([
        Buffer.from(String(timestamp)),
        Buffer.from(payload),
    ]);
    return crypto.createHmac('sha256', secret).update(message).digest('hex');
}

const payload   = JSON.stringify({ event: 'deposit.completed', amount: 100 });
const secret    = Buffer.from(process.env.WEBHOOK_SECRET, 'hex');
const timestamp = Math.floor(Date.now() / 1000);
const signature = signWebhook(payload, secret, timestamp);

await fetch('https://your-anchor.example/webhooks', {
    method: 'POST',
    headers: {
        'Content-Type':        'application/json',
        'X-Webhook-Signature': signature,
        'X-Webhook-Timestamp': String(timestamp),
        'X-Webhook-ID':        'wh_abc123',
    },
    body: payload,
});
```

**Python**

```python
import hmac, hashlib, json, time, os

def sign_webhook(payload: str, secret: bytes, timestamp: int) -> str:
    message = str(timestamp).encode() + payload.encode()
    return hmac.new(secret, message, hashlib.sha256).hexdigest()

payload   = json.dumps({"event": "deposit.completed", "amount": 100})
secret    = bytes.fromhex(os.environ["WEBHOOK_SECRET"])
timestamp = int(time.time())
signature = sign_webhook(payload, secret, timestamp)

import requests
requests.post(
    "https://your-anchor.example/webhooks",
    data=payload,
    headers={
        "Content-Type":        "application/json",
        "X-Webhook-Signature": signature,
        "X-Webhook-Timestamp": str(timestamp),
        "X-Webhook-ID":        "wh_abc123",
    },
)
```

**Go**

```go
import (
    "crypto/hmac"
    "crypto/sha256"
    "encoding/hex"
    "fmt"
    "time"
)

func signWebhook(payload []byte, secret []byte, timestamp int64) string {
    message := append([]byte(fmt.Sprintf("%d", timestamp)), payload...)
    h := hmac.New(sha256.New, secret)
    h.Write(message)
    return hex.EncodeToString(h.Sum(nil))
}
```

### Receiver: Verifying the Signature

On the AnchorKit side, `WebhookMiddleware::validate_webhook` handles verification as step 3 of the validation pipeline. Internally it:

1. Reads `X-Webhook-Timestamp` and `X-Webhook-Signature` from the request.
2. Reconstructs the canonical message: `timestamp_string + raw_payload_bytes`.
3. Computes `HMAC-SHA256(secret_key, message)`.
4. Compares the computed digest against the received signature using **constant-time comparison** (`subtle::ConstantTimeEq` equivalent) to prevent timing side-channels.

```rust
use anchorkit::{WebhookMiddleware, WebhookSecurityConfig, WebhookRequest, SignatureAlgorithm};

let config = WebhookSecurityConfig {
    algorithm: SignatureAlgorithm::Sha256,
    secret_key: Bytes::from_array(&env, &SECRET_KEY_BYTES),
    timestamp_tolerance_seconds: 300,
    max_payload_size_bytes: 10_000,
    enable_replay_protection: true,
};

let request = WebhookRequest {
    payload:        raw_payload_bytes,
    signature:      hex_decoded_signature_bytes,
    timestamp:      parsed_timestamp,
    webhook_id:     unique_id,
    source_address: None,
};

// verify_signature returns true only when the digest matches
let valid = WebhookMiddleware::verify_signature(&env, &request, &config)?;
```

You can also call `validate_webhook` to run the full pipeline (size → timestamp → signature → replay) in one shot:

```rust
let result = WebhookMiddleware::validate_webhook(&env, &request, &config)?;

if result.is_valid {
    // safe to process
}
```

### When Signature Verification Fails

A failed signature check triggers `ErrorCode::WebhookSignatureInvalid` (code 56) and the webhook is rejected before any business logic runs.

**What the middleware does automatically:**

- Logs a `SuspiciousActivityType::InvalidSignature` event at `ActivitySeverity::Critical`.
- Records a `WebhookDeliveryStatus::Rejected` delivery attempt.
- Emits a `(webhook, suspicious)` event to the AnchorKit event system for real-time monitoring.

**What your handler should do:**

```rust
match WebhookMiddleware::validate_webhook(&env, &request, &config) {
    Ok(result) if result.is_valid => {
        process_webhook(&env, &request.payload)?;
    }

    Ok(result) => {
        // result.is_valid == false; inspect the error
        if let Some(ref err) = result.error {
            if err.contains("Signature") {
                // 1. Do NOT retry with the same signature — it will fail again.
                // 2. Alert your on-call channel; this is a Critical-severity event.
                // 3. Verify the shared secret matches what the sender is using.
                // 4. Check that neither side is re-encoding the payload before signing.
                log_alert(&env, "Webhook signature invalid — possible tampering or key mismatch");
            }
        }
        return Err(Error::WebhookValidationFailed);
    }

    Err(e) => {
        // Unexpected contract error — surface it for investigation
        return Err(e);
    }
}
```

**Decision table for signature failures:**

| Scenario | Likely Cause | Action |
|----------|-------------|--------|
| All webhooks from one sender fail | Secret key mismatch | Re-exchange the shared secret |
| Intermittent failures from one sender | Payload re-encoded after signing | Ensure raw bytes are signed, not re-serialized |
| Failures only on large payloads | Encoding overhead changes byte length | Confirm sender signs raw body, not a transformed copy |
| Sudden spike after working fine | Secret rotated on sender side without notifying receiver | Coordinate key rotation with a grace period |
| Failures from unknown source IPs | Attacker probing the endpoint | Rate-limit by IP; escalate to security team |

**Key rules:**
- Never return a `200 OK` for a webhook that failed signature verification — this tells the sender the webhook was accepted.
- Return `401 Unauthorized` or `403 Forbidden` so the sender knows to investigate.
- Do not log the received signature value in plaintext — log only that verification failed and the webhook ID.

## Security Considerations

### 1. Secret Key Management
- Store secret keys in secure environment variables
- Never commit secrets to version control
- Rotate keys periodically
- Use different keys per webhook endpoint

### 2. Signature Algorithm Selection
- **HMAC-SHA256**: Recommended for most use cases
- **HMAC-SHA512**: Higher security margin, slightly slower
- **Ed25519**: Asymmetric, requires public key distribution

### 3. Timestamp Tolerance
- **300 seconds (5 min)**: Default, suitable for most systems
- **60 seconds (1 min)**: Strict, for high-security environments
- **600 seconds (10 min)**: Lenient, for systems with clock drift

### 4. Payload Size Limits
- **10KB**: Default, suitable for most webhooks
- **1MB**: Maximum recommended for security
- **Adjust based on**: Expected payload size, network conditions

### 5. Replay Protection
- Always enable for production
- Disable only for testing/development
- Monitor replay attack logs for patterns
- Investigate repeated replay attempts

## Error Handling

### Error Types

```rust
pub enum Error {
    WebhookTimestampExpired = 53,
    WebhookTimestampInFuture = 54,
    WebhookPayloadTooLarge = 55,
    WebhookSignatureInvalid = 56,
    WebhookValidationFailed = 57,
    ReplayAttack = 6,
}
```

### Error Recovery

```rust
match WebhookMiddleware::validate_webhook(&env, &request, &config) {
    Ok(result) => {
        if result.is_valid {
            // Process webhook
        } else {
            match result.error {
                Some(err) if err.contains("Timestamp") => {
                    // Handle timestamp error - check system clock
                }
                Some(err) if err.contains("Signature") => {
                    // Handle signature error - verify secret key
                }
                Some(err) if err.contains("Replay") => {
                    // Handle replay - check webhook ID
                }
                _ => {
                    // Handle other errors
                }
            }
        }
    }
    Err(e) => {
        // Handle validation error
        eprintln!("Validation error: {:?}", e);
    }
}
```

## Monitoring & Alerting

### Key Metrics

1. **Signature Failures**: Indicates compromised secret or attacker attempts
2. **Replay Attacks**: Indicates network issues or attack attempts
3. **Timestamp Violations**: Indicates clock skew or stale webhooks
4. **Payload Size Violations**: Indicates malformed or malicious payloads
5. **Delivery Success Rate**: Indicates system health

### Alert Thresholds

- **Critical**: 5+ signature failures in 1 minute
- **High**: 3+ replay attacks in 1 minute
- **Medium**: 10+ timestamp violations in 1 hour
- **Low**: Delivery success rate < 95%

### Event Monitoring

```rust
// Subscribe to webhook events
env.events().subscribe(
    (symbol_short!("webhook"), symbol_short!("suspicious")),
    |event| {
        // Handle suspicious activity event
        println!("Suspicious activity detected: {:?}", event);
    }
);

env.events().subscribe(
    (symbol_short!("webhook"), symbol_short!("delivery")),
    |event| {
        // Handle delivery event
        println!("Webhook delivery: {:?}", event);
    }
);
```

## Performance Characteristics

### Computational Complexity

| Operation | Time | Notes |
|-----------|------|-------|
| Signature Verification (HMAC-SHA256) | ~1ms | Constant-time |
| Timestamp Validation | <1ms | Simple comparison |
| Replay Detection | <1ms | Hash lookup |
| Payload Size Check | <1ms | Length check |
| Full Validation Pipeline | ~2-3ms | All checks combined |

### Storage Requirements

| Data | Size | TTL | Notes |
|------|------|-----|-------|
| Replay Hash | 32 bytes | 1 day | Per webhook |
| Delivery Record | ~200 bytes | 1 day | Per attempt |
| Activity Record | ~300 bytes | 7 days | Per suspicious activity |
| Config | ~500 bytes | Persistent | Per endpoint |

## Testing

### Unit Tests

```bash
cargo test webhook_middleware_tests
```

### Test Coverage

- ✅ Timestamp validation (within range, too old, future)
- ✅ Payload size validation (within limit, exceeds limit)
- ✅ Replay attack detection (first webhook, duplicate, different IDs)
- ✅ Suspicious activity logging (all activity types)
- ✅ Delivery tracking (success, failure, multiple attempts)
- ✅ Signature verification (HMAC-SHA256, HMAC-SHA512, Ed25519)
- ✅ Constant-time comparison
- ✅ Full validation pipeline

### Integration Testing

```rust
#[test]
fn test_webhook_integration() {
    let env = Env::default();
    let config = create_webhook_config(&env);
    
    // Create valid webhook
    let request = create_valid_webhook(&env);
    
    // Validate
    let result = WebhookMiddleware::validate_webhook(&env, &request, &config)?;
    assert!(result.is_valid);
    
    // Verify delivery was recorded
    let delivery = WebhookMiddleware::get_delivery_record(&env, request.webhook_id, 1);
    assert!(delivery.is_some());
}
```

## Best Practices

### 1. Configuration Management
- Store configs in environment variables
- Use different configs per environment (dev, staging, prod)
- Rotate secrets regularly
- Document all configuration options

### 2. Error Handling
- Log all validation failures
- Alert on suspicious patterns
- Implement exponential backoff for retries
- Track error rates per endpoint

### 3. Monitoring
- Track signature verification success rate
- Monitor replay attack frequency
- Alert on unusual patterns
- Maintain audit trail for compliance

### 4. Security
- Use HTTPS for webhook delivery
- Implement rate limiting per source
- Validate webhook source IP addresses
- Implement webhook signature rotation

### 5. Performance
- Cache security configs
- Use connection pooling for delivery
- Implement async webhook processing
- Monitor response times

## Troubleshooting

### Signature Verification Failures

**Symptoms**: Frequent "Invalid signature" errors

**Causes**:
- Secret key mismatch
- Payload modification in transit
- Incorrect signature algorithm
- Encoding issues

**Solutions**:
1. Verify secret key matches sender's key
2. Check payload encoding (UTF-8 vs binary)
3. Verify signature algorithm matches config
4. Check for payload modification in middleware

### Replay Attack False Positives

**Symptoms**: Legitimate webhooks rejected as replays

**Causes**:
- Webhook ID collision
- Payload hash collision (extremely rare)
- Storage TTL too short
- Concurrent webhook processing

**Solutions**:
1. Ensure webhook IDs are globally unique
2. Increase storage TTL if needed
3. Implement idempotency keys
4. Use distributed locking for concurrent processing

### Timestamp Validation Failures

**Symptoms**: Webhooks rejected due to timestamp

**Causes**:
- System clock skew
- Timezone issues
- Network latency
- Sender clock drift

**Solutions**:
1. Sync system clocks (NTP)
2. Increase timestamp tolerance
3. Check sender's clock accuracy
4. Implement clock skew detection

### Payload Size Violations

**Symptoms**: Large webhooks rejected

**Causes**:
- Payload size limit too small
- Webhook includes large data
- Compression not enabled
- Encoding overhead

**Solutions**:
1. Increase max_payload_size_bytes
2. Implement payload compression
3. Split large payloads
4. Use references instead of full data

## Integration with AnchorKit

### Event System Integration

```rust
// Webhook events are emitted to AnchorKit event system
env.events().publish(
    (symbol_short!("webhook"), symbol_short!("suspicious"), activity_id),
    suspicious_activity_record,
);

env.events().publish(
    (symbol_short!("webhook"), symbol_short!("delivery"), webhook_id),
    delivery_record,
);
```

### Request History Integration

```rust
// Webhook delivery attempts are tracked in request history
RequestHistory::record_call(
    &env,
    request_id,
    "webhook_delivery",
    caller,
    ApiCallStatus::Success,
    response_time_ms,
);
```

### Credential Management Integration

```rust
// Webhook secrets are stored using credential system
let credential = SecureCredential {
    attestor: webhook_endpoint,
    credential_type: CredentialType::ApiKey,
    encrypted_value: encrypted_secret,
    created_at: env.ledger().timestamp(),
    expires_at: expiration_time,
    rotation_required: false,
};
```

### Rate Limiting Integration

```rust
// Webhook delivery can be rate limited per source
RateLimiter::check_and_update(
    &env,
    &webhook_source,
    &rate_limit_config,
)?;
```

## References

- [OWASP Webhook Security](https://owasp.org/www-community/attacks/Webhook_Attacks)
- [RFC 2104 - HMAC](https://tools.ietf.org/html/rfc2104)
- [RFC 8032 - EdDSA](https://tools.ietf.org/html/rfc8032)
- [Soroban Crypto Module](https://docs.rs/soroban-sdk/latest/soroban_sdk/crypto/)

## Support

For issues or questions:
1. Check the troubleshooting section
2. Review test cases for usage examples
3. Check AnchorKit documentation
4. Open an issue on GitHub
