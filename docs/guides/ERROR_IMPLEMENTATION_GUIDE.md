# AnchorKit Error Handling Implementation Guide

This guide shows how to implement error handling in AnchorKit modules using the custom error system.

## Quick Start

### 1. Import Error Types

```rust
use crate::errors::Error;
use crate::anchor_kit_error::{AnchorKitError, ErrorCode, ErrorCategory, ErrorSeverity};
```

### 2. Return Errors from Functions

```rust
pub fn validate_config(config: &ContractConfig) -> Result<(), Error> {
    if config.max_sessions == 0 {
        return Err(Error::InvalidConfig);
    }
    Ok(())
}
```

### 3. Use Rich Error Information

```rust
match validate_config(&config) {
    Ok(()) => println!("Config valid"),
    Err(error) => {
        let kit_error = AnchorKitError::from(error);
        eprintln!("Error {}: {}", kit_error.code_u32(), kit_error.name());
    }
}
```

## Pattern: Error Mapping

When converting external errors to AnchorKit errors:

```rust
use crate::error_mapping::{
    map_http_status_to_error,
    map_anchor_error_to_protocol,
    map_network_error_to_transport,
};

pub fn handle_http_response(status: u32) -> Result<Data, Error> {
    match status {
        200 => Ok(parse_response()),
        _ => Err(map_http_status_to_error(status)),
    }
}

pub fn handle_anchor_error(error_code: &str) -> Result<(), Error> {
    if error_code.is_empty() {
        Ok(())
    } else {
        Err(map_anchor_error_to_protocol(error_code))
    }
}
```

## Pattern: Validation with Errors

```rust
pub fn validate_quote(quote: &QuoteData) -> Result<(), Error> {
    // Check required fields
    if quote.rate == 0 {
        return Err(Error::InvalidQuote);
    }

    // Check business rules
    if quote.minimum_amount > quote.maximum_amount {
        return Err(Error::InvalidQuote);
    }

    // Check expiry
    if quote.valid_until < current_timestamp() {
        return Err(Error::StaleQuote);
    }

    Ok(())
}
```

## Pattern: Retry Logic

```rust
use crate::error_mapping::is_transport_error_retryable;

pub fn execute_with_retry<F, T>(
    mut operation: F,
    max_attempts: u32,
) -> Result<T, Error>
where
    F: FnMut() -> Result<T, Error>,
{
    let mut attempt = 0;
    loop {
        match operation() {
            Ok(result) => return Ok(result),
            Err(error) => {
                let kit_error = AnchorKitError::from(error);
                attempt += 1;

                // Check if retryable
                if !kit_error.is_retryable() || attempt >= max_attempts {
                    return Err(kit_error.base_error());
                }

                // Exponential backoff
                let delay_ms = 100 * 2_u64.pow(attempt - 1);
                // sleep(delay_ms);
            }
        }
    }
}
```

## Pattern: Error Classification

```rust
pub fn handle_error(error: Error) {
    let kit_error = AnchorKitError::from(error);

    // Route by category
    match kit_error.category() {
        ErrorCategory::Transport => {
            // Handle network/HTTP errors
            if kit_error.is_retryable() {
                // Retry with backoff
            }
        }
        ErrorCategory::Protocol => {
            // Handle Anchor protocol errors
            if kit_error.is_retryable() {
                // Retry with backoff
            }
        }
        ErrorCategory::Application => {
            // Handle business logic errors
            // Usually not retryable
        }
    }

    // Route by severity
    match kit_error.severity() {
        ErrorSeverity::Critical => {
            // Alert and stop
            eprintln!("CRITICAL: {}", kit_error.name());
        }
        ErrorSeverity::High => {
            // Log and handle carefully
            eprintln!("HIGH: {}", kit_error.name());
        }
        ErrorSeverity::Medium => {
            // Log and continue
            eprintln!("MEDIUM: {}", kit_error.name());
        }
        ErrorSeverity::Low => {
            // Retry or ignore
            eprintln!("LOW: {}", kit_error.name());
        }
    }
}
```

## Pattern: Error Propagation

```rust
pub fn high_level_operation() -> Result<Data, Error> {
    // Propagate errors with ?
    let config = load_config()?;
    validate_config(&config)?;
    let data = fetch_data(&config)?;
    process_data(&data)?;
    Ok(data)
}

// Caller can use rich error information
match high_level_operation() {
    Ok(data) => println!("Success"),
    Err(error) => {
        let kit_error = AnchorKitError::from(error);
        eprintln!("Operation failed: {} ({})", 
            kit_error.name(), 
            kit_error.category().as_str()
        );
    }
}
```

## Pattern: Conditional Error Handling

```rust
pub fn process_with_fallback(primary: &str, fallback: &str) -> Result<Data, Error> {
    match fetch_data(primary) {
        Ok(data) => Ok(data),
        Err(error) => {
            let kit_error = AnchorKitError::from(error);

            // Retry on transport errors
            if kit_error.is_transport_error() && kit_error.is_retryable() {
                return fetch_data(fallback);
            }

            // Fail on non-retryable errors
            Err(kit_error.base_error())
        }
    }
}
```

## Pattern: Error Context

```rust
pub struct OperationContext {
    pub operation_id: u64,
    pub timestamp: u64,
    pub user: Address,
}

pub fn execute_operation(
    context: &OperationContext,
    operation: Operation,
) -> Result<(), Error> {
    match perform_operation(&operation) {
        Ok(()) => Ok(()),
        Err(error) => {
            let kit_error = AnchorKitError::from(error);
            
            // Log with context
            eprintln!(
                "Operation {} failed at {}: {} (code: {})",
                context.operation_id,
                context.timestamp,
                kit_error.name(),
                kit_error.code_u32()
            );

            Err(kit_error.base_error())
        }
    }
}
```

## Pattern: Error Response API

```rust
use soroban_sdk::String;

pub fn get_error_response(error: Error) -> String {
    let kit_error = AnchorKitError::from(error);
    let response = kit_error.response;

    // Format as JSON-like string
    format!(
        "{{\"code\":{},\"name\":\"{}\",\"category\":\"{}\",\"severity\":\"{}\",\"retryable\":{}}}",
        response.code,
        response.name,
        response.category,
        response.severity,
        response.retryable
    )
}
```

## Pattern: Testing Error Handling

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error() {
        let invalid_config = ContractConfig {
            max_sessions: 0,
            ..Default::default()
        };

        let result = validate_config(&invalid_config);
        assert!(result.is_err());

        let error = result.unwrap_err();
        let kit_error = AnchorKitError::from(error);
        assert_eq!(kit_error.code_u32(), 1901);
        assert_eq!(kit_error.name(), "InvalidConfig");
        assert!(!kit_error.is_retryable());
    }

    #[test]
    fn test_retryable_error() {
        let error = Error::TransportTimeout;
        let kit_error = AnchorKitError::from(error);

        assert!(kit_error.is_retryable());
        assert!(kit_error.is_transport_error());
        assert_eq!(kit_error.severity(), ErrorSeverity::Low);
    }

    #[test]
    fn test_error_response_format() {
        let error = Error::SessionNotFound;
        let kit_error = AnchorKitError::from(error);
        let response = kit_error.response;

        assert_eq!(response.code, 1601);
        assert_eq!(response.category, "application");
        assert!(response.retryable);
    }
}
```

## Common Error Scenarios

### Scenario 1: Invalid Input

```rust
pub fn submit_quote(quote: QuoteData) -> Result<u64, Error> {
    // Validate input
    if quote.rate == 0 {
        return Err(Error::InvalidQuote);  // 1701
    }

    // Process
    Ok(quote_id)
}
```

**Error Code**: 1701 (InvalidQuote)  
**Category**: Application  
**Severity**: Medium  
**Retryable**: No  

### Scenario 2: Network Timeout

```rust
pub fn fetch_anchor_info(url: &str) -> Result<AnchorInfo, Error> {
    match http_get(url) {
        Ok(response) => parse_response(response),
        Err(NetworkError::Timeout) => Err(Error::TransportTimeout),  // 2202
        Err(_) => Err(Error::TransportError),  // 2201
    }
}
```

**Error Code**: 2202 (TransportTimeout)  
**Category**: Transport  
**Severity**: Low  
**Retryable**: Yes  

### Scenario 3: Rate Limited

```rust
pub fn submit_request() -> Result<(), Error> {
    if rate_limiter.is_limited() {
        return Err(Error::ProtocolRateLimitExceeded);  // 2303
    }
    Ok(())
}
```

**Error Code**: 2303 (ProtocolRateLimitExceeded)  
**Category**: Protocol  
**Severity**: Low  
**Retryable**: Yes  

### Scenario 4: Compliance Check Failed

```rust
pub fn validate_transaction(intent: &TransactionIntent) -> Result<(), Error> {
    if !compliance_check(intent) {
        return Err(Error::ComplianceNotMet);  // 1802
    }
    Ok(())
}
```

**Error Code**: 1802 (ComplianceNotMet)  
**Category**: Application  
**Severity**: Critical  
**Retryable**: No  

### Scenario 5: Configuration Error

```rust
pub fn initialize(config: ContractConfig) -> Result<(), Error> {
    if config.max_sessions == 0 {
        return Err(Error::InvalidConfig);  // 1901
    }
    Ok(())
}
```

**Error Code**: 1901 (InvalidConfig)  
**Category**: Application  
**Severity**: Medium  
**Retryable**: No  

## Best Practices Checklist

- [ ] Use specific error codes, not generic ones
- [ ] Check retryability before implementing retry logic
- [ ] Log error severity for prioritization
- [ ] Classify errors to determine handling strategy
- [ ] Preserve error context when propagating
- [ ] Test error paths as thoroughly as success paths
- [ ] Document error conditions in function contracts
- [ ] Use standardized response format for APIs
- [ ] Implement exponential backoff for retries
- [ ] Alert on critical errors
- [ ] Monitor error rates by code and category

## Error Handling Checklist for New Functions

When implementing a new function:

1. **Identify possible errors**
   - Input validation errors
   - State errors
   - External service errors
   - Business logic errors

2. **Map to error codes**
   - Use existing codes when possible
   - Follow semantic ranges
   - Document error conditions

3. **Implement error handling**
   - Return appropriate error codes
   - Preserve error context
   - Use error mapping for external errors

4. **Test error paths**
   - Test each error condition
   - Verify error codes
   - Check retryability classification

5. **Document errors**
   - List possible errors in function docs
   - Explain error conditions
   - Provide recovery guidance

## Migration from Old Error Handling

### Before

```rust
pub fn old_function() -> Result<Data, Error> {
    match operation() {
        Ok(data) => Ok(data),
        Err(e) => Err(e),  // No context
    }
}
```

### After

```rust
pub fn new_function() -> Result<Data, Error> {
    match operation() {
        Ok(data) => Ok(data),
        Err(e) => {
            let kit_error = AnchorKitError::from(e);
            eprintln!("Error {}: {}", kit_error.code_u32(), kit_error.name());
            Err(kit_error.base_error())
        }
    }
}
```

## Summary

The AnchorKit error handling system provides:

✅ Standardized error codes with semantic meaning  
✅ Consistent response format across all operations  
✅ Rich error classification (category, severity, retryability)  
✅ Error mapping from external to internal errors  
✅ Full backward compatibility with base Error enum  
✅ Comprehensive testing infrastructure  

Use these patterns to implement robust, maintainable error handling throughout AnchorKit.
