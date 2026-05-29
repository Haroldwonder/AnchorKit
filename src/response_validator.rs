//! Response schema validation for AnchorKit API responses.
//!
//! Validates that anchor API responses contain all required fields before
//! returning them to the SDK consumer. Throws [`Error::ValidationError`] on mismatch.


extern crate alloc;

use crate::errors::Error;
use crate::types::{DepositResponse, WithdrawalResponse, TransactionStatus};

/// A validated quote response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuoteResponse {
    pub id: alloc::string::String,
    pub status: alloc::string::String,
    pub amount: u64,
    pub asset: alloc::string::String,
    pub fee: u64,
}

/// A validated anchor info response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorInfoResponse {
    pub name: alloc::string::String,
    pub supported_assets: alloc::vec::Vec<alloc::string::String>,
}

/// Validates a raw deposit response map, returning a typed [`DepositResponse`]
/// or [`Error::validation_error`] if any required field is missing or empty,
/// or if `expires_at` is not strictly in the future relative to `current_time`.
pub fn validate_deposit_response(
    transaction_id: &str,
    status: &str,
    deposit_address: &str,
    expires_at: u64,
    current_time: u64,
) -> Result<DepositResponse, Error> {
    if transaction_id.is_empty() {
        return Err(Error::validation_error("transaction_id is empty"));
    }
    if status.is_empty() {
        return Err(Error::validation_error("status is empty"));
    }
    if deposit_address.is_empty() {
        return Err(Error::validation_error("deposit_address is empty"));
    }
    if expires_at <= current_time {
        return Err(Error::validation_error("deposit has already expired"));
    }

    Ok(DepositResponse {
        transaction_id: alloc::string::String::from(transaction_id),
        how: None,
        extra_info: None,
        deposit_address: Some(alloc::string::String::from(deposit_address)),
        min_amount: None,
        max_amount: None,
        fee_fixed: None,
        fee_percent: None,
        expires_at: Some(expires_at),
        status: TransactionStatus::from_str(status),
    })
}

/// Validates a raw withdraw response, returning a typed [`WithdrawalResponse`]
/// or [`Error::validation_error`] if any required field is missing or empty.
pub fn validate_withdraw_response(
    transaction_id: &str,
    status: &str,
    estimated_completion: u64,
) -> Result<WithdrawalResponse, Error> {
    if transaction_id.is_empty() {
        return Err(Error::validation_error("transaction_id is empty"));
    }
    if status.is_empty() {
        return Err(Error::validation_error("status is empty"));
    }

    Ok(WithdrawalResponse {
        transaction_id: alloc::string::String::from(transaction_id),
        account_id: None,
        dest_account_id: None,
        memo: None,
        memo_type: None,
        min_amount: None,
        max_amount: None,
        fee_fixed: None,
        fee_percent: None,
        estimated_completion: Some(estimated_completion),
        status: TransactionStatus::from_str(status),
    })
}

/// Validates a raw quote response, returning a typed [`QuoteResponse`]
/// or [`Error::validation_error`] if any required field is missing or empty,
/// or if `amount` is zero.
pub fn validate_quote_response(
    id: &str,
    status: &str,
    amount: u64,
    asset: &str,
    fee: u64,
) -> Result<QuoteResponse, Error> {
    if id.is_empty() {
        return Err(Error::validation_error("id is empty"));
    }
    if status.is_empty() {
        return Err(Error::validation_error("status is empty"));
    }
    if asset.is_empty() {
        return Err(Error::validation_error("asset is empty"));
    }
    if amount == 0 {
        return Err(Error::validation_error("amount must be greater than zero"));
    }

    Ok(QuoteResponse {
        id: alloc::string::String::from(id),
        status: alloc::string::String::from(status),
        amount,
        asset: alloc::string::String::from(asset),
        fee,
    })
}

/// Validates a raw anchor info response, returning a typed [`AnchorInfoResponse`]
/// or [`Error::validation_error`] if any required field is missing or empty.
pub fn validate_anchor_info_response(
    name: &str,
    supported_assets: alloc::vec::Vec<alloc::string::String>,
) -> Result<AnchorInfoResponse, Error> {
    if name.is_empty() {
        return Err(Error::validation_error("name is empty"));
    }
    if supported_assets.is_empty() {
        return Err(Error::validation_error("supported_assets is empty"));
    }

    Ok(AnchorInfoResponse {
        name: alloc::string::String::from(name),
        supported_assets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- validate_deposit_response ---

    #[test]
    fn test_valid_deposit_response() {
        let result = validate_deposit_response("dep_123", "pending", "GDEPOSIT...", 9999, 1000);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.transaction_id, "dep_123");
        assert_eq!(r.status, TransactionStatus::Pending);
        assert_eq!(r.deposit_address, Some(alloc::string::String::from("GDEPOSIT...")));
        assert_eq!(r.expires_at, Some(9999));
    }

    #[test]
    fn test_deposit_missing_transaction_id() {
        let result = validate_deposit_response("", "pending", "GDEPOSIT...", 9999, 1000);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, crate::errors::ErrorCode::ValidationError);
    }

    #[test]
    fn test_deposit_missing_status() {
        let result = validate_deposit_response("dep_123", "", "GDEPOSIT...", 9999, 1000);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, crate::errors::ErrorCode::ValidationError);
    }

    #[test]
    fn test_deposit_missing_deposit_address() {
        let result = validate_deposit_response("dep_123", "pending", "", 9999, 1000);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, crate::errors::ErrorCode::ValidationError);
    }

    #[test]
    fn test_deposit_zero_expires_at_fails() {
        // expires_at = 0 is always in the past — must be rejected
        let result = validate_deposit_response("dep_123", "pending", "GDEPOSIT...", 0, 1000);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, crate::errors::ErrorCode::ValidationError);
        assert!(err.context.as_deref().unwrap_or("").contains("expired"));
    }

    #[test]
    fn test_deposit_past_expires_at_fails() {
        // expires_at in the past relative to current_time
        let result = validate_deposit_response("dep_123", "pending", "GDEPOSIT...", 500, 1000);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, crate::errors::ErrorCode::ValidationError);
        assert!(err.context.as_deref().unwrap_or("").contains("expired"));
    }

    #[test]
    fn test_deposit_expires_at_equal_to_current_time_fails() {
        // expires_at == current_time is not strictly in the future
        let result = validate_deposit_response("dep_123", "pending", "GDEPOSIT...", 1000, 1000);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, crate::errors::ErrorCode::ValidationError);
        assert!(err.context.as_deref().unwrap_or("").contains("expired"));
    }

    #[test]
    fn test_deposit_future_expires_at_passes() {
        let result = validate_deposit_response("dep_123", "pending", "GDEPOSIT...", 2000, 1000);
        assert!(result.is_ok());
    }

    // --- validate_withdraw_response ---

    #[test]
    fn test_valid_withdraw_response() {
        let result = validate_withdraw_response("wd_456", "processing", 2000);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.transaction_id, "wd_456");
        assert_eq!(r.status, TransactionStatus::Unknown(alloc::string::String::from("processing")));
        assert_eq!(r.estimated_completion, Some(2000));
    }

    #[test]
    fn test_withdraw_missing_transaction_id() {
        let result = validate_withdraw_response("", "processing", 2000);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, crate::errors::ErrorCode::ValidationError);
    }

    #[test]
    fn test_withdraw_missing_status() {
        let result = validate_withdraw_response("wd_456", "", 2000);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, crate::errors::ErrorCode::ValidationError);
    }

    // --- validate_quote_response ---

    #[test]
    fn test_valid_quote_response() {
        let result = validate_quote_response("quote_789", "quoted", 100_0000000, "USDC", 500000);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.id, "quote_789");
        assert_eq!(r.status, "quoted");
        assert_eq!(r.amount, 100_0000000);
        assert_eq!(r.asset, "USDC");
        assert_eq!(r.fee, 500000);
    }

    #[test]
    fn test_quote_missing_id() {
        let result = validate_quote_response("", "quoted", 100_0000000, "USDC", 500000);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, crate::errors::ErrorCode::ValidationError);
    }

    #[test]
    fn test_quote_missing_status() {
        let result = validate_quote_response("quote_789", "", 100_0000000, "USDC", 500000);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, crate::errors::ErrorCode::ValidationError);
    }

    #[test]
    fn test_quote_missing_asset() {
        let result = validate_quote_response("quote_789", "quoted", 100_0000000, "", 500000);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, crate::errors::ErrorCode::ValidationError);
    }

    #[test]
    fn test_quote_zero_amount_fails() {
        // amount = 0 must be rejected to prevent division-by-zero downstream
        let result = validate_quote_response("quote_789", "quoted", 0, "USDC", 0);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, crate::errors::ErrorCode::ValidationError);
        assert!(err.context.as_deref().unwrap_or("").contains("amount must be greater than zero"));
    }

    #[test]
    fn test_quote_zero_fee_with_nonzero_amount_passes() {
        // fee = 0 is valid (free transaction); only amount must be > 0
        let result = validate_quote_response("quote_789", "quoted", 100_0000000, "USDC", 0);
        assert!(result.is_ok());
    }

    // --- validate_anchor_info_response ---

    #[test]
    fn test_valid_anchor_info_response() {
        let assets = alloc::vec![
            alloc::string::String::from("USDC"),
            alloc::string::String::from("XLM"),
        ];
        let result = validate_anchor_info_response("MyAnchor", assets);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.name, "MyAnchor");
        assert_eq!(r.supported_assets.len(), 2);
    }

    #[test]
    fn test_anchor_info_missing_name() {
        let assets = alloc::vec![alloc::string::String::from("USDC")];
        let result = validate_anchor_info_response("", assets);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, crate::errors::ErrorCode::ValidationError);
    }

    #[test]
    fn test_anchor_info_empty_assets() {
        let result = validate_anchor_info_response("MyAnchor", alloc::vec![]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, crate::errors::ErrorCode::ValidationError);
    }

    // --- SDK does not crash on validation error ---

    #[test]
    fn test_validation_error_does_not_panic() {
        // Simulates SDK consumer handling the error gracefully
        let result = validate_deposit_response("", "", "", 0, 1000);
        match result {
            Err(e) if e.code == crate::errors::ErrorCode::ValidationError => { /* handled, no crash */ }
            _ => panic!("Expected ValidationError"),
        }
    }
}
