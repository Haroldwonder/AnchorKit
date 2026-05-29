//! Transaction state tracking for off-chain SDK usage.
//!
//! This module provides [`TransactionStateTracker`], an **off-chain only** utility
//! that tracks transaction state transitions in memory. It is **not backed by
//! on-chain Soroban storage** and cannot be queried via contract methods.
//!
//! Use this tracker in your client SDK to maintain local state of transaction
//! lifecycle without storing state on the blockchain.

use soroban_sdk::{contracttype, Address, Env, String};

/// Transaction states for the state tracker
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum TransactionState {
    Pending = 1,
    InProgress = 2,
    Completed = 3,
    Failed = 4,
    /// Unrecognized state string from a future or unknown protocol version
    Unknown = 5,
}

impl TransactionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionState::Pending => "pending",
            TransactionState::InProgress => "in_progress",
            TransactionState::Completed => "completed",
            TransactionState::Failed => "failed",
            TransactionState::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(TransactionState::Pending),
            "in_progress" => Some(TransactionState::InProgress),
            "completed" => Some(TransactionState::Completed),
            "failed" => Some(TransactionState::Failed),
            _ => Some(TransactionState::Unknown),
        }
    }
}

/// Transaction state transition record
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateTransition {
    pub state: TransactionState,
    pub timestamp: u64,
}

/// Transaction state record
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionStateRecord {
    pub transaction_id: u64,
    pub state: TransactionState,
    pub initiator: Address,
    pub timestamp: u64,
    pub last_updated: u64,
    pub error_message: Option<String>,
    pub history: soroban_sdk::Vec<StateTransition>,
}

/// Off-chain transaction state tracker.
///
/// This type operates entirely in memory and is **not backed by on-chain storage**.
/// It is suitable for client-side SDK usage to track transaction state locally
/// during its lifecycle, but state is not persisted to or queryable from the
/// Soroban contract.
#[derive(Clone)]
#[allow(dead_code)]
pub struct TransactionStateTracker {
    cache: alloc::vec::Vec<TransactionStateRecord>,
    /// Per-state counters indexed by TransactionState discriminant (1-based).
    /// Index 0 is unused; indices 1-5 map to Pending/InProgress/Completed/Failed/Unknown.
    state_counts: [u64; 6],
}

#[allow(dead_code)]
impl TransactionStateTracker {
    /// Create a new transaction state tracker
    pub fn new() -> Self {
        TransactionStateTracker {
            cache: alloc::vec::Vec::new(),
            state_counts: [0u64; 6],
        }
    }

    /// Create a transaction with pending state
    pub fn create_transaction(
        &mut self,
        transaction_id: u64,
        initiator: Address,
        env: &Env,
    ) -> Result<TransactionStateRecord, String> {
        let current_time = env.ledger().timestamp();

        let record = TransactionStateRecord {
            transaction_id,
            state: TransactionState::Pending,
            initiator,
            timestamp: current_time,
            last_updated: current_time,
            error_message: None,
            history: {
                let mut h = soroban_sdk::Vec::new(env);
                h.push_back(StateTransition {
                    state: TransactionState::Pending,
                    timestamp: current_time,
                });
                h
            },
        };

        self.cache.push(record.clone());
        self.state_counts[TransactionState::Pending as usize] += 1;

        Ok(record)
    }

    /// Update transaction state to in-progress
    pub fn start_transaction(
        &mut self,
        transaction_id: u64,
        env: &Env,
    ) -> Result<TransactionStateRecord, String> {
        self.update_state(transaction_id, TransactionState::InProgress, None, env)
    }

    /// Mark transaction as completed
    pub fn complete_transaction(
        &mut self,
        transaction_id: u64,
        env: &Env,
    ) -> Result<TransactionStateRecord, String> {
        self.update_state(transaction_id, TransactionState::Completed, None, env)
    }

    /// Mark transaction as failed
    pub fn fail_transaction(
        &mut self,
        transaction_id: u64,
        error_message: String,
        env: &Env,
    ) -> Result<TransactionStateRecord, String> {
        self.update_state(
            transaction_id,
            TransactionState::Failed,
            Some(error_message),
            env,
        )
    }

    /// Update transaction state
    fn update_state(
        &mut self,
        transaction_id: u64,
        new_state: TransactionState,
        error_message: Option<String>,
        env: &Env,
    ) -> Result<TransactionStateRecord, String> {
        let current_time = env.ledger().timestamp();

        // Search and update in cache
        for record in self.cache.iter_mut() {
            if record.transaction_id == transaction_id {
                let old_state = record.state;
                record.state = new_state;
                record.last_updated = current_time;
                if new_state == TransactionState::Failed {
                    record.error_message = error_message.clone();
                }
                record.error_message = error_message;
                record.history.push_back(StateTransition {
                    state: new_state,
                    timestamp: current_time,
                });
                // Update state counts
                if self.state_counts[old_state as usize] > 0 {
                    self.state_counts[old_state as usize] -= 1;
                }
                self.state_counts[new_state as usize] += 1;
                return Ok(record.clone());
            }
        }
        Err(String::from_str(
            env,
            "Transaction not found in cache",
        ))
    }

    /// Get transaction state by ID
    pub fn get_transaction_state(
        &self,
        transaction_id: u64,
        _env: &Env,
    ) -> Result<Option<TransactionStateRecord>, String> {
        for record in self.cache.iter() {
            if record.transaction_id == transaction_id {
                return Ok(Some(record.clone()));
            }
        }
        Ok(None)
    }

    /// Get transaction history by ID
    pub fn get_transaction_history(
        &self,
        transaction_id: u64,
        _env: &Env,
    ) -> Result<soroban_sdk::Vec<StateTransition>, String> {
        for record in self.cache.iter() {
            if record.transaction_id == transaction_id {
                return Ok(record.history.clone());
            }
        }
        Err(String::from_str(_env, "Transaction not found"))
    }


    /// Get all transactions in a specific state
    pub fn get_transactions_by_state(
        &self,
        state: TransactionState,
    ) -> Result<alloc::vec::Vec<TransactionStateRecord>, String> {
        let mut result = alloc::vec::Vec::new();
        for record in self.cache.iter() {
            if record.state == state {
                result.push(record.clone());
            }
        }
        Ok(result)
    }

    /// Get all transactions
    pub fn get_all_transactions(&self) -> Result<alloc::vec::Vec<TransactionStateRecord>, String> {
        Ok(self.cache.clone())
    }

    /// Clear all cached transactions.
    /// Requires admin authorization.
    pub fn clear_cache(&mut self, admin: &Address, _env: &Env) -> Result<(), String> {
        admin.require_auth();
        self.cache = alloc::vec::Vec::new();
        self.state_counts = [0u64; 6];
        Ok(())
    }

    /// Get cache size — O(1)
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Get the count of transactions in a given state — O(1).
    /// More efficient than `get_transactions_by_state(...).len()`.
    pub fn get_transaction_count_by_state(&self, state: TransactionState) -> u64 {
        self.state_counts[state as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;
    use soroban_sdk::testutils::Address;

    #[test]
    fn test_create_transaction() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new();
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        let result = tracker.create_transaction(1, initiator.clone(), &env);
        assert!(result.is_ok());

        let record = result.unwrap();
        assert_eq!(record.transaction_id, 1);
        assert_eq!(record.state, TransactionState::Pending);
        assert_eq!(record.initiator, initiator);
    }

    #[test]
    fn test_start_transaction() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new();
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        let result = tracker.start_transaction(1, &env);

        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.state, TransactionState::InProgress);
    }

    #[test]
    fn test_complete_transaction() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new();
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        tracker.start_transaction(1, &env).ok();
        let result = tracker.complete_transaction(1, &env);

        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.state, TransactionState::Completed);
    }

    #[test]
    fn test_fail_transaction() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new();
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        let error_msg = String::from_str(&env, "Test error");
        let result = tracker.fail_transaction(1, error_msg, &env);

        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.state, TransactionState::Failed);
        assert!(record.error_message.is_some());
    }

    #[test]
    fn test_get_transaction_state() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new();
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        let result = tracker.get_transaction_state(1, &env);

        assert!(result.is_ok());
        let state = result.unwrap();
        assert!(state.is_some());
        assert_eq!(state.unwrap().state, TransactionState::Pending);
    }

    #[test]
    fn test_get_transactions_by_state() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new();
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        tracker.create_transaction(2, initiator.clone(), &env).ok();
        tracker.start_transaction(1, &env).ok();

        let result = tracker.get_transactions_by_state(TransactionState::Pending);
        assert!(result.is_ok());
        let transactions = result.unwrap();
        assert_eq!(transactions.len(), 1);
    }

    #[test]
    fn test_get_all_transactions() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new();
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        tracker.create_transaction(2, initiator.clone(), &env).ok();

        let result = tracker.get_all_transactions();
        assert!(result.is_ok());
        let transactions = result.unwrap();
        assert_eq!(transactions.len(), 2);
    }

    #[test]
    fn test_cache_size() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new();
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        tracker.create_transaction(2, initiator.clone(), &env).ok();

        assert_eq!(tracker.cache_size(), 2);
    }

    #[test]
    fn test_clear_cache() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new();
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        let clear_result = tracker.clear_cache(&initiator, &env);

        assert!(clear_result.is_ok());
        assert_eq!(tracker.cache_size(), 0);
    }

    #[test]
    fn test_transaction_history_lifecycle() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new();
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        tracker.start_transaction(1, &env).ok();
        tracker.complete_transaction(1, &env).ok();

        let result = tracker.get_transaction_history(1, &env);
        assert!(result.is_ok());
        let history = result.unwrap();

        assert_eq!(history.len(), 3);
        assert_eq!(history.get(0).unwrap().state, TransactionState::Pending);
        assert_eq!(history.get(1).unwrap().state, TransactionState::InProgress);
        assert_eq!(history.get(2).unwrap().state, TransactionState::Completed);
    }

    #[test]
    fn test_unknown_state_counter() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new();

        // Verify that Unknown state (index 5) is accessible without panic
        let count = tracker.get_transaction_count_by_state(TransactionState::Unknown);
        assert_eq!(count, 0);

        // Directly increment the Unknown state counter to simulate a transition
        // This verifies the array is large enough
        tracker.state_counts[TransactionState::Unknown as usize] = 1;
        assert_eq!(tracker.get_transaction_count_by_state(TransactionState::Unknown), 1);
    }

    #[test]
    #[should_panic]
    fn test_clear_cache_requires_admin_auth() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new();
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let different_admin = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator, &env).ok();
        assert_eq!(tracker.cache_size(), 1);

        // This should panic because different_admin has not authorized this call
        tracker.clear_cache(&different_admin, &env).ok();
    }
}
