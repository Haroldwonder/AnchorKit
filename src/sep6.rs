//! SEP-6 Deposit & Withdrawal Service Layer
//!
//! Provides normalized service functions for initiating deposits, withdrawals,
//! and fetching transaction status across different anchors.


extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::errors::{Error, ErrorCode};

// ── Normalized response types ────────────────────────────────────────────────

/// Normalized status values across all SEP-6 anchors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Incomplete,
    PendingExternal,
    PendingAnchor,
    PendingTrust,
    PendingUser,
    Completed,
    Refunded,
    Expired,
    Error,
    Unknown(String),
}

impl TransactionStatus {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "pending_external" => Self::PendingExternal,
            "pending_anchor" => Self::PendingAnchor,
            "pending_trust" => Self::PendingTrust,
            "pending_user" | "pending_user_transfer_start" => Self::PendingUser,
            "completed" => Self::Completed,
            "refunded" => Self::Refunded,
            "expired" => Self::Expired,
            "incomplete" => Self::Incomplete,
            "pending" => Self::Pending,
            _ => Self::Unknown(s.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::Incomplete => "incomplete",
            Self::PendingExternal => "pending_external",
            Self::PendingAnchor => "pending_anchor",
            Self::PendingTrust => "pending_trust",
            Self::PendingUser => "pending_user",
            Self::Completed => "completed",
            Self::Refunded => "refunded",
            Self::Expired => "expired",
            Self::Error => "error",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

/// Normalized response for a deposit initiation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DepositResponse {
    /// Unique transaction ID assigned by the anchor.
    pub transaction_id: String,
    /// How the user should send funds (e.g. bank account, address).
    pub how: String,
    /// Optional extra instructions from the anchor.
    pub extra_info: Option<String>,
    /// Minimum deposit amount (in asset units), if provided.
    pub min_amount: Option<u64>,
    /// Maximum deposit amount (in asset units), if provided.
    pub max_amount: Option<u64>,
    /// Fee charged for the deposit, if provided.
    pub fee_fixed: Option<u64>,
    /// Percentage fee charged for the deposit in basis points, if provided (e.g. `150` = 1.50%).
    pub fee_percent: Option<u32>,
    /// Current status of the transaction.
    pub status: TransactionStatus,
}

/// Normalized response for a withdrawal initiation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawalResponse {
    /// Unique transaction ID assigned by the anchor.
    pub transaction_id: String,
    /// Stellar account the user should send funds to.
    pub account_id: String,
    /// Destination bank/wallet account for the off-chain withdrawal, if provided.
    pub dest_account_id: Option<String>,
    /// Optional memo to attach to the Stellar payment.
    pub memo: Option<String>,
    /// Optional memo type (`text`, `id`, `hash`).
    pub memo_type: Option<String>,
    /// Minimum withdrawal amount (in asset units), if provided.
    pub min_amount: Option<u64>,
    /// Maximum withdrawal amount (in asset units), if provided.
    pub max_amount: Option<u64>,
    /// Fee charged for the withdrawal, if provided.
    pub fee_fixed: Option<u64>,
    /// Percentage fee charged for the withdrawal in basis points, if provided (e.g. `150` = 1.50%).
    pub fee_percent: Option<u32>,
    /// Current status of the transaction.
    pub status: TransactionStatus,
}

/// Normalized transaction status response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionStatusResponse {
    pub transaction_id: String,
    pub kind: TransactionKind,
    pub status: TransactionStatus,
    /// Amount sent by the user (in asset units), if known.
    pub amount_in: Option<u64>,
    /// Amount received by the user after fees (in asset units), if known.
    pub amount_out: Option<u64>,
    /// Fee charged (in asset units), if known.
    pub amount_fee: Option<u64>,
    /// Human-readable message from the anchor, if any.
    pub message: Option<String>,
}

/// Whether the transaction is a deposit or withdrawal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransactionKind {
    Deposit,
    Withdrawal,
}

impl TransactionKind {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "withdrawal" | "withdraw" => Self::Withdrawal,
            _ => Self::Deposit,
        }
    }
}

// ── Raw anchor response shapes (anchor-agnostic input) ───────────────────────

/// Raw fields from an anchor's `/deposit` response.
/// Callers populate only the fields the anchor actually returns.
pub struct RawDepositResponse {
    pub transaction_id: String,
    pub how: String,
    pub extra_info: Option<String>,
    pub min_amount: Option<u64>,
    pub max_amount: Option<u64>,
    pub fee_fixed: Option<u64>,
    pub fee_percent: Option<u32>,
    /// Raw status string from the anchor (e.g. `"pending_external"`).
    pub status: Option<String>,
    /// Optional Stellar account (G-address) of the depositor.
    pub depositor_account: Option<String>,
}

/// Raw fields from an anchor's `/withdraw` response.
pub struct RawWithdrawalResponse {
    pub transaction_id: String,
    pub account_id: String,
    pub dest_account_id: Option<String>,
    pub memo: Option<String>,
    pub memo_type: Option<String>,
    pub min_amount: Option<u64>,
    pub max_amount: Option<u64>,
    pub fee_fixed: Option<u64>,
    pub fee_percent: Option<u32>,
    pub status: Option<String>,
}

/// Raw fields from an anchor's `/transaction` response.
pub struct RawTransactionResponse {
    pub transaction_id: String,
    pub kind: Option<String>,
    pub status: String,
    pub amount_in: Option<u64>,
    pub amount_out: Option<u64>,
    pub amount_fee: Option<u64>,
    pub message: Option<String>,
}

/// Input parameters for fetching a list of transactions from an anchor.
pub struct RawTransactionListRequest {
    /// Stellar account (G-address) whose transactions to fetch.
    pub account: String,
    /// Asset code to filter by (e.g. `"USDC"`).
    pub asset_code: String,
    /// Maximum number of transactions to return.
    pub limit: u32,
    /// Pagination cursor — the transaction ID to start after, if any.
    pub cursor: Option<String>,
}

// ── Service functions ─────────────────────────────────────────────────────────

fn is_valid_stellar_address(s: &str) -> bool {
    s.len() == 56
        && s.starts_with('G')
        && s.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Normalize a raw anchor deposit response into a canonical [`DepositResponse`].
///
/// Returns `Err(Error::invalid_transaction_intent())` if required fields are missing.
pub fn initiate_deposit(raw: RawDepositResponse) -> Result<DepositResponse, Error> {
    if raw.transaction_id.is_empty() || raw.how.is_empty() {
        return Err(Error::invalid_transaction_intent());
    }
    if let Some(ref acct) = raw.depositor_account {
        if !is_valid_stellar_address(acct) {
            return Err(Error::with_context(
                ErrorCode::ValidationError,
                "Invalid Stellar address",
                acct,
            ));
        }
    }

    Ok(DepositResponse {
        transaction_id: raw.transaction_id,
        how: raw.how,
        extra_info: raw.extra_info,
        min_amount: raw.min_amount,
        max_amount: raw.max_amount,
        fee_fixed: raw.fee_fixed,
        fee_percent: raw.fee_percent,
        status: raw
            .status
            .as_deref()
            .map(TransactionStatus::from_str)
            .unwrap_or(TransactionStatus::Pending),
    })
}

/// Normalize a raw anchor withdrawal response into a canonical [`WithdrawalResponse`].
///
/// Returns `Err(Error::invalid_transaction_intent())` if required fields are missing.
pub fn initiate_withdrawal(raw: RawWithdrawalResponse) -> Result<WithdrawalResponse, Error> {
    if raw.transaction_id.is_empty() || raw.account_id.is_empty() {
        return Err(Error::invalid_transaction_intent());
    }

    Ok(WithdrawalResponse {
        transaction_id: raw.transaction_id,
        account_id: raw.account_id,
        dest_account_id: raw.dest_account_id,
        memo: raw.memo,
        memo_type: raw.memo_type,
        min_amount: raw.min_amount,
        max_amount: raw.max_amount,
        fee_fixed: raw.fee_fixed,
        fee_percent: raw.fee_percent,
        status: raw
            .status
            .as_deref()
            .map(TransactionStatus::from_str)
            .unwrap_or(TransactionStatus::Pending),
    })
}

/// Normalize a raw anchor transaction-status response into a canonical
/// [`TransactionStatusResponse`].
///
/// Returns `Err(Error::invalid_transaction_intent())` if the transaction ID is missing.
pub fn fetch_transaction_status(
    raw: RawTransactionResponse,
) -> Result<TransactionStatusResponse, Error> {
    if raw.transaction_id.is_empty() {
        return Err(Error::invalid_transaction_intent());
    }

    Ok(TransactionStatusResponse {
        transaction_id: raw.transaction_id,
        kind: raw
            .kind
            .as_deref()
            .map(TransactionKind::from_str)
            .unwrap_or(TransactionKind::Deposit),
        status: TransactionStatus::from_str(&raw.status),
        amount_in: raw.amount_in,
        amount_out: raw.amount_out,
        amount_fee: raw.amount_fee,
        message: raw.message,
    })
}

/// Normalize a list of raw transaction responses for the given account and asset.
///
/// Returns `Err(Error::ValidationError)` if `account` is not a valid Stellar address
/// or if `asset_code` is empty. Individual items that fail normalization are skipped.
/// The `cursor` field in `req` is available for callers to pass to the anchor's API;
/// this function applies it as a filter — items up to and including the cursor ID are
/// dropped, mirroring standard cursor-based pagination.
pub fn list_transactions(
    req: RawTransactionListRequest,
    raw_items: Vec<RawTransactionResponse>,
) -> Result<Vec<TransactionStatusResponse>, Error> {
    if !is_valid_stellar_address(&req.account) {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "Invalid Stellar address",
            &req.account,
        ));
    }
    if req.asset_code.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "asset_code must not be empty",
            "asset_code",
        ));
    }

    let mut skip = req.cursor.is_some();
    let mut results = Vec::new();

    for item in raw_items {
        if let Some(ref cursor) = req.cursor {
            if item.transaction_id == *cursor {
                skip = false;
                continue;
            }
            if skip {
                continue;
            }
        }
        if results.len() as u32 >= req.limit {
            break;
        }
        if let Ok(normalized) = fetch_transaction_status(item) {
            results.push(normalized);
        }
    }

    Ok(results)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_deposit() -> RawDepositResponse {
        RawDepositResponse {
            transaction_id: "txn-001".to_string(),
            how: "Send to bank account 1234".to_string(),
            extra_info: None,
            min_amount: Some(10),
            max_amount: Some(10_000),
            fee_fixed: Some(1),
            fee_percent: None,
            status: Some("pending_external".to_string()),
            depositor_account: None,
        }
    }

    fn raw_withdrawal() -> RawWithdrawalResponse {
        RawWithdrawalResponse {
            transaction_id: "txn-002".to_string(),
            account_id: "GABC123".to_string(),
            dest_account_id: Some("bank-account-9876".to_string()),
            memo: Some("12345".to_string()),
            memo_type: Some("id".to_string()),
            min_amount: Some(5),
            max_amount: Some(5_000),
            fee_fixed: Some(2),
            fee_percent: None,
            status: Some("pending_user".to_string()),
        }
    }

    fn raw_tx_status() -> RawTransactionResponse {
        RawTransactionResponse {
            transaction_id: "txn-001".to_string(),
            kind: Some("deposit".to_string()),
            status: "completed".to_string(),
            amount_in: Some(100),
            amount_out: Some(99),
            amount_fee: Some(1),
            message: None,
        }
    }

    #[test]
    fn test_initiate_deposit_normalizes_response() {
        let resp = initiate_deposit(raw_deposit()).unwrap();
        assert_eq!(resp.transaction_id, "txn-001");
        assert_eq!(resp.status, TransactionStatus::PendingExternal);
        assert_eq!(resp.fee_fixed, Some(1));
    }

    #[test]
    fn test_initiate_deposit_missing_fields_returns_error() {
        let mut raw = raw_deposit();
        raw.transaction_id = "".to_string();
        assert_eq!(initiate_deposit(raw), Err(Error::invalid_transaction_intent()));
    }

    #[test]
    fn test_initiate_deposit_invalid_stellar_address_returns_error() {
        let mut raw = raw_deposit();
        raw.depositor_account = Some("not-a-stellar-address".to_string());
        let err = initiate_deposit(raw).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_initiate_deposit_valid_stellar_address_accepted() {
        let mut raw = raw_deposit();
        // 56-char G-address
        raw.depositor_account = Some("GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN".to_string());
        assert!(initiate_deposit(raw).is_ok());
    }

    #[test]
    fn test_initiate_deposit_defaults_status_to_pending() {
        let mut raw = raw_deposit();
        raw.status = None;
        let resp = initiate_deposit(raw).unwrap();
        assert_eq!(resp.status, TransactionStatus::Pending);
    }

    #[test]
    fn test_initiate_withdrawal_normalizes_response() {
        let resp = initiate_withdrawal(raw_withdrawal()).unwrap();
        assert_eq!(resp.transaction_id, "txn-002");
        assert_eq!(resp.status, TransactionStatus::PendingUser);
        assert_eq!(resp.memo_type, Some("id".to_string()));
        assert_eq!(resp.dest_account_id, Some("bank-account-9876".to_string()));
    }

    #[test]
    fn test_initiate_withdrawal_missing_account_returns_error() {
        let mut raw = raw_withdrawal();
        raw.account_id = "".to_string();
        assert_eq!(
            initiate_withdrawal(raw),
            Err(Error::invalid_transaction_intent())
        );
    }

    #[test]
    fn test_fetch_transaction_status_normalizes_response() {
        let resp = fetch_transaction_status(raw_tx_status()).unwrap();
        assert_eq!(resp.status, TransactionStatus::Completed);
        assert_eq!(resp.kind, TransactionKind::Deposit);
        assert_eq!(resp.amount_out, Some(99));
    }

    #[test]
    fn test_fetch_transaction_status_missing_id_returns_error() {
        let mut raw = raw_tx_status();
        raw.transaction_id = "".to_string();
        assert_eq!(
            fetch_transaction_status(raw),
            Err(Error::invalid_transaction_intent())
        );
    }

    #[test]
    fn test_fetch_transaction_status_unknown_status_maps_to_error() {
        let mut raw = raw_tx_status();
        raw.status = "some_unknown_status".to_string();
        let resp = fetch_transaction_status(raw).unwrap();
        assert_eq!(resp.status, TransactionStatus::Unknown("some_unknown_status".to_string()));
    }

    #[test]
    fn test_withdrawal_kind_normalization() {
        let mut raw = raw_tx_status();
        raw.kind = Some("withdraw".to_string());
        let resp = fetch_transaction_status(raw).unwrap();
        assert_eq!(resp.kind, TransactionKind::Withdrawal);

        // Mixed-case withdrawal variants
        for s in &["Withdrawal", "WITHDRAWAL", "Withdraw", "WITHDRAW"] {
            let mut r = raw_tx_status();
            r.kind = Some(s.to_string());
            assert_eq!(fetch_transaction_status(r).unwrap().kind, TransactionKind::Withdrawal, "failed for {s}");
        }

        // Mixed-case deposit variants
        for s in &["Deposit", "DEPOSIT", "deposit"] {
            let mut r = raw_tx_status();
            r.kind = Some(s.to_string());
            assert_eq!(fetch_transaction_status(r).unwrap().kind, TransactionKind::Deposit, "failed for {s}");
        }
    }

    #[test]
    fn test_initiate_deposit_fee_percent_propagated() {
        let mut raw = raw_deposit();
        raw.fee_percent = Some(150);
        let resp = initiate_deposit(raw).unwrap();
        assert_eq!(resp.fee_percent, Some(150));
    }

    #[test]
    fn test_initiate_withdrawal_fee_percent_propagated() {
        let mut raw = raw_withdrawal();
        raw.fee_percent = Some(50);
        let resp = initiate_withdrawal(raw).unwrap();
        assert_eq!(resp.fee_percent, Some(50));
    }

    // ── list_transactions ────────────────────────────────────────────────────

    const VALID_ACCOUNT: &str = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";

    fn make_raw_tx(id: &str, status: &str) -> RawTransactionResponse {
        RawTransactionResponse {
            transaction_id: id.to_string(),
            kind: Some("deposit".to_string()),
            status: status.to_string(),
            amount_in: None,
            amount_out: None,
            amount_fee: None,
            message: None,
        }
    }

    fn base_req() -> RawTransactionListRequest {
        RawTransactionListRequest {
            account: VALID_ACCOUNT.to_string(),
            asset_code: "USDC".to_string(),
            limit: 10,
            cursor: None,
        }
    }

    #[test]
    fn test_list_transactions_returns_all_items() {
        let items = alloc::vec![
            make_raw_tx("t1", "completed"),
            make_raw_tx("t2", "pending"),
        ];
        let result = list_transactions(base_req(), items).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].transaction_id, "t1");
        assert_eq!(result[1].transaction_id, "t2");
    }

    #[test]
    fn test_list_transactions_respects_limit() {
        let items = alloc::vec![
            make_raw_tx("t1", "completed"),
            make_raw_tx("t2", "completed"),
            make_raw_tx("t3", "completed"),
        ];
        let mut req = base_req();
        req.limit = 2;
        let result = list_transactions(req, items).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_list_transactions_cursor_pagination() {
        let items = alloc::vec![
            make_raw_tx("t1", "completed"),
            make_raw_tx("t2", "completed"),
            make_raw_tx("t3", "completed"),
        ];
        let mut req = base_req();
        req.cursor = Some("t1".to_string());
        let result = list_transactions(req, items).unwrap();
        // t1 is the cursor — items after it are t2, t3
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].transaction_id, "t2");
    }

    #[test]
    fn test_list_transactions_invalid_account_returns_error() {
        let result = list_transactions(
            RawTransactionListRequest {
                account: "bad-account".to_string(),
                asset_code: "USDC".to_string(),
                limit: 10,
                cursor: None,
            },
            alloc::vec![],
        );
        assert_eq!(result.unwrap_err().code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_list_transactions_empty_asset_code_returns_error() {
        let mut req = base_req();
        req.asset_code = "".to_string();
        let result = list_transactions(req, alloc::vec![]);
        assert_eq!(result.unwrap_err().code, ErrorCode::ValidationError);
    }
}
