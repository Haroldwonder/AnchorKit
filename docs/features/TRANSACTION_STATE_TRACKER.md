# Transaction State Tracker Implementation

## Overview

The Transaction State Tracker is a comprehensive system for managing and tracking the lifecycle of transactions within the AnchorKit smart contract. It provides a robust mechanism to track transactions through four distinct states: Pending, In-Progress, Completed, and Failed.

> **⚠️ Off-Chain Utility Only**
> `TransactionStateTracker` is an **off-chain SDK utility** and is **not backed by on-chain Soroban storage**.
> It operates entirely in memory and cannot be queried via contract methods. Use this tracker in your client SDK
> to maintain local transaction state during its lifecycle without storing state on the blockchain.

## Features

### Core States

The Transaction State Tracker supports four transaction states:

1. **Pending** - Initial state when a transaction is created
2. **In-Progress** - State when the transaction processing has started
3. **Completed** - State when the transaction has been successfully processed
4. **Failed** - State when the transaction has failed with an error message

### State Transitions

```
                    ┌─────────────────┐
                    │    PENDING      │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │  IN_PROGRESS    │
                    └────┬────────┬───┘
                         │        │
         ┌───────────────┘        └───────────────┐
         │                                         │
    ┌────▼────────┐                       ┌───────▼──────┐
    │  COMPLETED  │                       │    FAILED    │
    └─────────────┘                       └──────────────┘
```

### Storage Options

- **Development Mode**: Uses in-memory cache (`Vec<TransactionStateRecord>`)
- **Production Mode**: Designed for database persistence (implementation ready for DB integration)

## API Reference

### TransactionState Enum

```rust
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum TransactionState {
    Pending = 1,
    InProgress = 2,
    Completed = 3,
    Failed = 4,
}
```

### TransactionStateRecord Struct

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionStateRecord {
    pub transaction_id: u64,
    pub state: TransactionState,
    pub initiator: Address,
    pub timestamp: u64,
    pub last_updated: u64,
    pub error_message: Option<String>,
}
```

### TransactionStateTracker Methods

#### Initialization
```rust
let mut tracker = TransactionStateTracker::new(is_dev_mode);
```

#### Create Transaction
```rust
tracker.create_transaction(transaction_id, initiator, env)?;
```
- Creates a new transaction in Pending state
- Returns: `TransactionStateRecord`

#### Update States
```rust
// Move to In-Progress
tracker.start_transaction(transaction_id, env)?;

// Mark as Completed
tracker.complete_transaction(transaction_id, env)?;

// Mark as Failed with error message
tracker.fail_transaction(transaction_id, error_message, env)?;
```

#### Query Operations
```rust
// Get specific transaction state
tracker.get_transaction_state(transaction_id, env)?;

// Get all transactions in a specific state
tracker.get_transactions_by_state(state)?;

// Get all transactions
tracker.get_all_transactions()?;

// Get cache size (dev mode only)
tracker.cache_size();

// Clear cache (dev mode only)
tracker.clear_cache()?;
```

## Usage Example

### Basic Usage

```rust
use anchorkit::{TransactionStateTracker, TransactionState};
use soroban_sdk::Env;

fn handle_transaction(env: &Env, user: Address) {
    let mut tracker = TransactionStateTracker::new(true); // dev mode
    
    // Create a transaction
    let record = tracker.create_transaction(1, user, env)
        .expect("Failed to create transaction");
    assert_eq!(record.state, TransactionState::Pending);
    
    // Start processing
    let record = tracker.start_transaction(1, env)
        .expect("Failed to start transaction");
    assert_eq!(record.state, TransactionState::InProgress);
    
    // Process and complete
    let record = tracker.complete_transaction(1, env)
        .expect("Failed to complete transaction");
    assert_eq!(record.state, TransactionState::Completed);
}
```

### Handling Failures

```rust
fn handle_failed_transaction(env: &Env, user: Address) {
    let mut tracker = TransactionStateTracker::new(true);
    
    tracker.create_transaction(1, user, env).ok();
    tracker.start_transaction(1, env).ok();
    
    // Handle failure
    let error_msg = String::from_slice(env, "Insufficient balance".as_bytes());
    let record = tracker.fail_transaction(1, error_msg, env)
        .expect("Failed to mark transaction as failed");
    
    assert_eq!(record.state, TransactionState::Failed);
    assert!(record.error_message.is_some());
}
```

### Querying Transactions

```rust
fn monitor_transactions(tracker: &TransactionStateTracker) {
    // Get all pending transactions
    let pending = tracker.get_transactions_by_state(TransactionState::Pending)
        .expect("Failed to query pending transactions");
    
    // Get all in-progress transactions
    let in_progress = tracker.get_transactions_by_state(TransactionState::InProgress)
        .expect("Failed to query in-progress transactions");
    
    // Get all failed transactions for error handling
    let failed = tracker.get_transactions_by_state(TransactionState::Failed)
        .expect("Failed to query failed transactions");
}
```

## Implementation Details

### Memory Cache (Dev Mode)

- Transactions are stored in a `Vec<TransactionStateRecord>`
- Useful for development and testing
- Can be cleared with `clear_cache()`
- Access with O(n) complexity for lookups

### Database Mode (Production)

- Framework prepared for database integration
- In production mode, data would be persisted to permanent storage
- Supports error handling for database operations
- Ready to implement with:
  - Soroban persistent storage
  - External database integration
  - Distributed cache (Redis, etc.)

## State Management Features

### Timestamp Tracking

- **timestamp**: Initial creation time
- **last_updated**: Latest state change time
- Automatically tracked for audit trails

### Error Handling

- Failed transactions can store detailed error messages
- Error messages preserved in state record
- Useful for debugging and user notification

### Initiator Tracking

- Every transaction tracks the initiator address
- Enables per-user transaction filtering
- Supports multi-actor scenarios

## Testing

The implementation includes comprehensive tests covering:

1. State transitions (Pending → In-Progress → Completed/Failed)
2. Error cases (transaction not found, production mode constraints)
3. State queries by single ID and batch by state
4. Cache management and lifecycle
5. Timestamp accuracy
6. Multi-transaction isolation

Run tests with:
```bash
cargo test --lib transaction_state_tracker
```

## Integration Points

The Transaction State Tracker integrates with:

- **Storage Module**: Extended with transaction state storage keys
- **Types Module**: Defines shared state enums and structures
- **Error Handling**: Uses AnchorKit error types
- **Session Management**: Can be used to track session-level transactions

## Future Enhancements

1. **Database Persistence**: Implement Soroban persistent storage backend
2. **Event Emission**: Emit events on state transitions
3. **TTL Management**: Auto-cleanup of old transactions
4. **Batch Operations**: Bulk state updates for efficiency
5. **Advanced Queries**: Complex filtering and aggregation
6. **Rate Limiting**: Throttle transaction creation
7. **History Tracking**: Full audit trail of state changes

## Migration Guide

To add Transaction State Tracker to your AnchorKit deployment:

1. Update to the latest AnchorKit version
2. Import the module: `use anchorkit::TransactionStateTracker;`
3. Initialize tracker: `let tracker = TransactionStateTracker::new(is_dev_mode);`
4. Use the API for transaction lifecycle management

## Performance Considerations

- **Dev Mode**: O(n) for lookups, O(1) for append
- **Production Mode**: Depends on storage backend implementation
- **Cache Size**: No built-in limits; clear periodically if needed
- **Memory Usage**: ~100-200 bytes per transaction record

## Security Notes

- Transactions are immutable once created (state transitions only)
- Initiator address prevents spoofing
- Error messages are logged but should not contain sensitive data
- Production mode with database backend should use encryption at rest
