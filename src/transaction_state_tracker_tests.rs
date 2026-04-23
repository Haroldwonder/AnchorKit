/// Transaction State Tracker Tests
/// This test file demonstrates and validates the Transaction State Tracker implementation

use crate::transaction_state_tracker::*;
use soroban_sdk::Env;

#[cfg(test)]
mod transaction_state_tracker_tests {
    use super::*;
    use soroban_sdk::testutils::Address;
    use soroban_sdk::String;

    #[test]
    fn test_transaction_state_to_string() {
        assert_eq!(TransactionState::Pending.as_str(), "pending");
        assert_eq!(TransactionState::InProgress.as_str(), "in_progress");
        assert_eq!(TransactionState::Completed.as_str(), "completed");
        assert_eq!(TransactionState::Failed.as_str(), "failed");
    }

    #[test]
    fn test_transaction_state_from_string() {
        assert_eq!(
            TransactionState::from_str("pending"),
            Some(TransactionState::Pending)
        );
        assert_eq!(
            TransactionState::from_str("in_progress"),
            Some(TransactionState::InProgress)
        );
        assert_eq!(
            TransactionState::from_str("completed"),
            Some(TransactionState::Completed)
        );
        assert_eq!(
            TransactionState::from_str("failed"),
            Some(TransactionState::Failed)
        );
        assert_eq!(TransactionState::from_str("unknown"), None);
    }

    #[test]
    fn test_full_transaction_lifecycle() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new(true);
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        // Create transaction -> Pending
        let tx1_result = tracker.create_transaction(1, initiator.clone(), &env);
        assert!(tx1_result.is_ok());
        let tx1 = tx1_result.unwrap();
        assert_eq!(tx1.state, TransactionState::Pending);

        // Start transaction -> In-progress
        let tx2_result = tracker.start_transaction(1, &env);
        assert!(tx2_result.is_ok());
        let tx2 = tx2_result.unwrap();
        assert_eq!(tx2.state, TransactionState::InProgress);

        // Complete transaction -> Completed
        let tx3_result = tracker.complete_transaction(1, &env);
        assert!(tx3_result.is_ok());
        let tx3 = tx3_result.unwrap();
        assert_eq!(tx3.state, TransactionState::Completed);
    }

    #[test]
    fn test_transaction_failure_with_error_message() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new(true);
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        tracker.start_transaction(1, &env).ok();

        let error_msg = String::from_str(&env, "Payment declined");
        let result = tracker.fail_transaction(1, error_msg.clone(), &env);

        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.state, TransactionState::Failed);
        assert_eq!(record.error_message, Some(error_msg));
    }

    #[test]
    fn test_query_transactions_by_state() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new(true);
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        // Create 5 transactions
        for i in 1..=5 {
            tracker.create_transaction(i, initiator.clone(), &env).ok();
        }

        // Move some to in-progress
        tracker.start_transaction(1, &env).ok();
        tracker.start_transaction(2, &env).ok();

        // Complete one
        tracker.complete_transaction(2, &env).ok();

        // Query by state
        let pending_result = tracker.get_transactions_by_state(TransactionState::Pending);
        assert!(pending_result.is_ok());
        let pending = pending_result.unwrap();
        assert_eq!(pending.len(), 3); // 3, 4, 5

        let in_progress_result = tracker.get_transactions_by_state(TransactionState::InProgress);
        assert!(in_progress_result.is_ok());
        let in_progress = in_progress_result.unwrap();
        assert_eq!(in_progress.len(), 1); // 1

        let completed_result = tracker.get_transactions_by_state(TransactionState::Completed);
        assert!(completed_result.is_ok());
        let completed = completed_result.unwrap();
        assert_eq!(completed.len(), 1); // 2
    }

    #[test]
    fn test_production_mode_flag() {
        let env = Env::default();
        let mut prod_tracker = TransactionStateTracker::new(false);
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        // In production mode, cache should not be populated
        let result = prod_tracker.create_transaction(1, initiator.clone(), &env);
        assert!(result.is_ok());
        assert_eq!(prod_tracker.cache_size(), 0); // Should be 0 in production mode

        // In dev mode, cache should be populated
        let mut dev_tracker = TransactionStateTracker::new(true);
        let result = dev_tracker.create_transaction(1, initiator.clone(), &env);
        assert!(result.is_ok());
        assert_eq!(dev_tracker.cache_size(), 1); // Should be 1 in dev mode
    }

    #[test]
    fn test_transaction_not_found() {
        let env = Env::default();
        let tracker = TransactionStateTracker::new(true);

        let result = tracker.get_transaction_state(999, &env);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_multiple_transactions_isolation() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new(true);
        let initiator1 = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let initiator2 = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        // Create transactions from different initiators
        tracker.create_transaction(1, initiator1.clone(), &env).ok();
        tracker.create_transaction(2, initiator2.clone(), &env).ok();

        // Update first one
        tracker.start_transaction(1, &env).ok();

        // Verify second one is still pending
        let tx2_state = tracker.get_transaction_state(2, &env);
        assert!(tx2_state.is_ok());
        let tx2 = tx2_state.unwrap().unwrap();
        assert_eq!(tx2.state, TransactionState::Pending);
        assert_eq!(tx2.initiator, initiator2);
    }

    #[test]
    fn test_clear_cache_dev_mode() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new(true);
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        tracker.create_transaction(2, initiator.clone(), &env).ok();
        assert_eq!(tracker.cache_size(), 2);

        let clear_result = tracker.clear_cache();
        assert!(clear_result.is_ok());
        assert_eq!(tracker.cache_size(), 0);
    }

    #[test]
    fn test_timestamp_tracking() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new(true);
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        let create_result = tracker.create_transaction(1, initiator.clone(), &env);
        let record1 = create_result.unwrap();
        let initial_timestamp = record1.timestamp;

        let update_result = tracker.start_transaction(1, &env);
        let record2 = update_result.unwrap();

        // Timestamps should be set and last_updated should reflect the change
        assert_eq!(record2.timestamp, initial_timestamp);
        assert!(record2.last_updated >= initial_timestamp);
    }

    #[test]
    fn test_get_transaction_count_by_state_basic() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new(true);
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        tracker.create_transaction(2, initiator.clone(), &env).ok();
        tracker.create_transaction(3, initiator.clone(), &env).ok();

        assert_eq!(tracker.get_transaction_count_by_state(TransactionState::Pending), 3);
        assert_eq!(tracker.get_transaction_count_by_state(TransactionState::InProgress), 0);

        tracker.start_transaction(1, &env).ok();
        assert_eq!(tracker.get_transaction_count_by_state(TransactionState::Pending), 2);
        assert_eq!(tracker.get_transaction_count_by_state(TransactionState::InProgress), 1);

        tracker.complete_transaction(1, &env).ok();
        assert_eq!(tracker.get_transaction_count_by_state(TransactionState::InProgress), 0);
        assert_eq!(tracker.get_transaction_count_by_state(TransactionState::Completed), 1);
    }

    #[test]
    fn test_get_transaction_count_by_state_failed() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new(true);
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        tracker.create_transaction(2, initiator.clone(), &env).ok();
        let err = String::from_str(&env, "err");
        tracker.fail_transaction(1, err, &env).ok();

        assert_eq!(tracker.get_transaction_count_by_state(TransactionState::Pending), 1);
        assert_eq!(tracker.get_transaction_count_by_state(TransactionState::Failed), 1);
    }

    #[test]
    fn test_get_transaction_count_by_state_after_clear() {
        let env = Env::default();
        let mut tracker = TransactionStateTracker::new(true);
        let initiator = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        tracker.create_transaction(1, initiator.clone(), &env).ok();
        assert_eq!(tracker.get_transaction_count_by_state(TransactionState::Pending), 1);

        tracker.clear_cache().ok();
        assert_eq!(tracker.get_transaction_count_by_state(TransactionState::Pending), 0);
    }
}
