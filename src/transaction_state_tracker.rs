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
}

impl TransactionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionState::Pending => "pending",
            TransactionState::InProgress => "in_progress",
            TransactionState::Completed => "completed",
            TransactionState::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(TransactionState::Pending),
            "in_progress" => Some(TransactionState::InProgress),
            "completed" => Some(TransactionState::Completed),
            "failed" => Some(TransactionState::Failed),
            _ => None,
        }
    }
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
}

/// Transaction state tracker
#[derive(Clone)]
#[allow(dead_code)]
pub struct TransactionStateTracker {
    cache: alloc::vec::Vec<TransactionStateRecord>,
    is_dev_mode: bool,
}

#[allow(dead_code)]
impl TransactionStateTracker {
    /// Create a new transaction state tracker
    pub fn new(is_dev_mode: bool) -> Self {
        TransactionStateTracker {
            cache: alloc::vec::Vec::new(),
            is_dev_mode,
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
        };

        if self.is_dev_mode {
            self.cache.push(record.clone());
        }

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

        if self.is_dev_mode {
            // Search and update in cache
            for record in self.cache.iter_mut() {
                if record.transaction_id == transaction_id {
                    record.state = new_state;
                    record.last_updated = current_time;
                    record.error_message = error_message;
                    return Ok(record.clone());
                }
            }
            Err(String::from_str(
                env,
                "Transaction not found in cache",
            ))
        } else {
            let dummy_address = Address::from_string(&String::from_str(env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"));
            let record = TransactionStateRecord {
                transaction_id,
                state: new_state,
                initiator: dummy_address,
                timestamp: current_time,
                last_updated: current_time,
                error_message,
            };
            Ok(record)
        }
    }

    /// Get transaction state by ID
    pub fn get_transaction_state(
        &self,
        transaction_id: u64,
        _env: &Env,
    ) -> Result<Option<TransactionStateRecord>, String> {
        if self.is_dev_mode {
            for record in self.cache.iter() {
                if record.transaction_id == transaction_id {
                    return Ok(Some(record.clone()));
                }
            }
            Ok(None)
        } else {
            // In production, this would query the DB
            Ok(None)
        }
    }

    /// Get all transactions in a specific state
    pub fn get_transactions_by_state(
        &self,
        state: TransactionState,
    ) -> Result<alloc::vec::Vec<TransactionStateRecord>, String> {
        if self.is_dev_mode {
            let mut result = alloc::vec::Vec::new();
            for record in self.cache.iter() {
                if record.state == state {
                    result.push(record.clone());
                }
            }
            Ok(result)
        } else {
            // In production, this would query the DB
            Ok(alloc::vec::Vec::new())
        }
    }

    /// Get all transactions
    pub fn get_all_transactions(&self) -> Result<alloc::vec::Vec<TransactionStateRecord>, String> {
        if self.is_dev_mode {
            Ok(self.cache.clone())
        } else {
            // In production, this would query the DB
            Ok(alloc::vec::Vec::new())
        }
    }

    /// Clear all cached transactions (dev mode only)
    pub fn clear_cache(&mut self) -> Result<(), String> {
        if self.is_dev_mode {
            self.cache = alloc::vec::Vec::new();
            Ok(())
        } else {
            Err(String::from_str(&Env::default(), "Cannot clear cache in production mode"))
        }
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
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
        let mut tracker = TransactionStateTracker::new(true);
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
        let mut tracker = TransactionStateTracker::new(true);
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
        let mut tracker = TransactionStateTracker::new(true);
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
        let mut tracker = TransactionStateTracker::new(true);
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
        let mut tracker = TransactionStateTracker::new(true);
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
        let mut tracker = TransactionStateTracker::new(true);
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
        let mut tracker = TransactionStateTracker::new(true);
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
        let mut tracker = TransactionStateTracker::new(true);
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        tracker.create_transaction(2, initiator.clone(), &env).ok();

        assert_eq!(tracker.cache_size(), 2);
    }

    #[test]
    fn test_clear_cache() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new(true);
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        let clear_result = tracker.clear_cache();

        assert!(clear_result.is_ok());
        assert_eq!(tracker.cache_size(), 0);
    }
}
