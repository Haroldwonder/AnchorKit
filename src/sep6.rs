//! SEP-6 Deposit & Withdrawal Service Layer
//!
//! Provides normalized service functions for initiating deposits, withdrawals,
//! and fetching transaction status across different anchors.


extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::errors::{Error, ErrorCode};
use crate::retry::RetryConfig;

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
            "error" => Self::Error,
            _ => Self::Unknown(String::from(s)),
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
    /// Whether the anchor supports claimable balances as the deposit destination.
    /// Sourced from the `CLAIMABLE_BALANCE_SUPPORTED` flag in stellar.toml.
    pub claimable_balance_supported: bool,
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
    Unknown(String),
}

impl TransactionKind {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "withdrawal" | "withdraw" => Self::Withdrawal,
            "deposit" => Self::Deposit,
            _ => Self::Unknown(String::from(s)),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Deposit => "deposit",
            Self::Withdrawal => "withdrawal",
            Self::Unknown(s) => s.as_str(),
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
    /// Whether the anchor supports claimable balances as the deposit destination.
    /// Sourced from the `CLAIMABLE_BALANCE_SUPPORTED` flag in stellar.toml.
    pub claimable_balance_supported: bool,
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

/// Raw fields from an anchor's `/withdraw-exchange` response.
pub struct RawWithdrawExchangeResponse {
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
#[derive(Clone, Debug)]
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

/// StrKey version byte for an ed25519 public key ("G..." account address).
const STRKEY_VERSION_ACCOUNT_ID: u8 = 6 << 3;

/// Decodes a 56-character StrKey string into its raw 35-byte representation
/// (1 version byte + 32-byte payload + 2-byte CRC16-XModem checksum),
/// validating that every character belongs to the Base32 alphabet (`A`-`Z`, `2`-`7`).
fn decode_strkey(s: &str) -> Option<[u8; 35]> {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

    if s.len() != 56 || !s.is_ascii() {
        return None;
    }

    let mut out = [0u8; 35];
    let mut buffer: u64 = 0;
    let mut bits: u32 = 0;
    let mut out_idx = 0;

    for &b in s.as_bytes() {
        let value = ALPHABET.iter().position(|&c| c == b)? as u64;
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out[out_idx] = ((buffer >> bits) & 0xFF) as u8;
            out_idx += 1;
        }
    }

    Some(out)
}

/// Computes the CRC16-XModem checksum used by Stellar's StrKey encoding.
fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Validates that `s` is a well-formed Stellar account address (StrKey "G..." format):
/// correct length, strict Base32 alphabet, the ed25519-public-key version byte, and a
/// matching CRC16-XModem checksum.
fn is_valid_stellar_address(s: &str) -> bool {
    let decoded = match decode_strkey(s) {
        Some(d) => d,
        None => return false,
    };

    if decoded[0] != STRKEY_VERSION_ACCOUNT_ID {
        return false;
    }

    let payload = &decoded[..33];
    let expected_checksum = u16::from_le_bytes([decoded[33], decoded[34]]);
    crc16_xmodem(payload) == expected_checksum
}

fn is_valid_asset_code(s: &str) -> bool {
    let len = s.len();
    len >= 1 && len <= 12 && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// Classifies whether an HTTP status code represents a retryable error.
///
/// Returns `true` for transient errors (5xx server errors, timeouts, connection errors).
/// Returns `false` for client errors (4xx), which are not retryable.
/// 
/// - 5xx: Server errors (retryable)
/// - 4xx: Client errors like 400, 401, 403, 404 (not retryable — don't retry bad requests)
/// - Network timeouts and connection errors (represented externally) are retryable
pub fn is_http_error_retryable(http_status: u32) -> bool {
    match http_status {
        // 5xx Server errors: retryable
        500..=599 => true,
        // 4xx Client errors: not retryable
        400..=499 => false,
        // Other codes (1xx, 2xx, 3xx): not retryable
        _ => false,
    }
}

/// Wraps transaction status fetching with automatic exponential backoff retry logic.
///
/// This function will retry on transient errors but will not retry on permanent
/// client errors or validation failures.
///
/// # Arguments
/// - `fetch_fn`: A closure that fetches a raw transaction response. Called with the attempt number (0-based).
/// - `retry_config`: Optional retry configuration. If `None`, uses `RetryConfig::default()`.
/// - `sleep_fn`: A function to sleep between retries (useful for testing with mocks).
///
/// # Behavior
/// The function automatically retries if:
/// - `fetch_fn` returns an error marked as retryable
/// - Network timeouts or transient failures occur
///
/// The function does NOT retry if:
/// - `fetch_fn` returns an error marked as non-retryable
/// - The maximum retry attempts are exhausted
///
/// # Example (conceptual, assuming HTTP client integration)
/// ```ignore
/// let result = fetch_transaction_status_with_retry(
///     |_attempt| http_client.get("/transaction?id=123"),
///     None,
///     |ms| std::thread::sleep(std::time::Duration::from_millis(ms)),
/// );
/// ```
pub fn fetch_transaction_status_with_retry<F, S>(
    fetch_fn: F,
    retry_config: Option<RetryConfig>,
    sleep_fn: S,
) -> Result<TransactionStatusResponse, Error>
where
    F: FnMut(u32) -> Result<RawTransactionResponse, Error>,
    S: FnMut(u64),
{
    use crate::retry::retry_with_backoff;

    let config = retry_config.unwrap_or_default();
    let raw_result = retry_with_backoff(&config, 0, fetch_fn, |_err| true, sleep_fn);

    raw_result.and_then(|raw| fetch_transaction_status(raw))
}

/// Normalize a raw anchor deposit response into a canonical [`DepositResponse`].
///
/// Validates that asset_code is non-empty and matches the Stellar asset code format (1-12 uppercase alphanumeric).
///
/// Returns `Err(Error::invalid_transaction_intent())` if required fields are missing.
/// Returns `Err(Error::ValidationError)` if asset_code is invalid.
pub fn initiate_deposit(raw: RawDepositResponse, asset_code: &str) -> Result<DepositResponse, Error> {
    if !is_valid_asset_code(asset_code) {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "Invalid asset code format: must be 1-12 uppercase alphanumeric characters",
            asset_code,
        ));
    }
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
        claimable_balance_supported: raw.claimable_balance_supported,
    })
}

/// Normalize a raw anchor withdrawal response into a canonical [`WithdrawalResponse`].
///
/// Validates that asset_code is non-empty and matches the Stellar asset code format (1-12 uppercase alphanumeric).
///
/// Returns `Err(Error::invalid_transaction_intent())` if required fields are missing.
/// Returns `Err(Error::ValidationError)` if asset_code is invalid.
pub fn initiate_withdrawal(raw: RawWithdrawalResponse, asset_code: &str) -> Result<WithdrawalResponse, Error> {
    if !is_valid_asset_code(asset_code) {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "Invalid asset code format: must be 1-12 uppercase alphanumeric characters",
            asset_code,
        ));
    }
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

/// Normalize a raw anchor cross-asset withdrawal response into a canonical [`WithdrawalResponse`].
///
/// Follows the same normalization rules as [`initiate_withdrawal`].
/// Returns `Err(Error::invalid_transaction_intent())` if required fields are missing.
/// Returns `Err(Error::ValidationError)` if `source_asset` or `destination_asset` is invalid.
pub fn withdraw_exchange(
    raw: RawWithdrawExchangeResponse,
    source_asset: &str,
    destination_asset: &str,
) -> Result<WithdrawalResponse, Error> {
    if !is_valid_asset_code(source_asset) {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "Invalid source asset code format",
            source_asset,
        ));
    }
    if !is_valid_asset_code(destination_asset) {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "Invalid destination asset code format",
            destination_asset,
        ));
    }
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

/// Fetch and normalize transaction status, handling HTTP status codes separately.
///
/// Maps HTTP status codes to specific errors:
/// - 404 → AttestationNotFound
/// - 429 → RateLimitExceeded
/// - Other non-2xx → Generic HTTP error
/// - 2xx → Normalizes the raw response to TransactionStatusResponse
///
/// Returns `Err(Error::invalid_transaction_intent())` if the transaction ID is missing (for 2xx responses).
pub fn get_transaction_status(
    http_status: u32,
    raw: RawTransactionResponse,
) -> Result<TransactionStatusResponse, Error> {
    match http_status {
        404 => Err(Error::attestation_not_found()),
        429 => Err(Error::rate_limit_exceeded()),
        200..=299 => fetch_transaction_status(raw),
        _ => Err(Error::with_context(
            ErrorCode::ValidationError,
            "HTTP request failed",
            &alloc::format!("HTTP {}", http_status),
        )),
    }
}

/// Normalize a list of raw transaction responses for the given account and asset.
///
/// Returns `Err(Error::ValidationError)` if `account` is not a valid Stellar address
/// or if `asset_code` is empty. Individual items that fail normalization are skipped.
/// The `cursor` field in `req` is available for callers to pass to the anchor's API;
/// this function applies it as a filter — items up to and including the cursor ID are
/// dropped, mirroring standard cursor-based pagination.
///
/// Returns `Err(Error::ValidationError)` if `cursor` is set but no item in
/// `raw_items` has a matching `transaction_id` (stale, foreign, or garbage cursor).
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
    let mut cursor_found = req.cursor.is_none();
    let mut results = Vec::new();

    for item in raw_items {
        if let Some(ref cursor) = req.cursor {
            if item.transaction_id == *cursor {
                skip = false;
                cursor_found = true;
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

    if !cursor_found {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "cursor not found in result set",
            req.cursor.as_deref().unwrap_or(""),
        ));
    }

    Ok(results)
}

// ── #650 deposit_exchange ─────────────────────────────────────────────────────

/// Raw fields for a deposit-exchange (buy/convert) request.
pub struct RawDepositExchangeRequest {
    /// Asset the user is sending (e.g. `"USD"`).
    pub source_asset: String,
    /// Asset the user wants to receive (e.g. `"USDC"`).
    pub destination_asset: String,
    /// Amount of `source_asset` to exchange (in asset units).
    pub amount: u64,
}

/// Normalized response for a deposit-exchange initiation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DepositExchangeResponse {
    pub transaction_id: String,
    pub how: String,
    pub extra_info: Option<String>,
    pub fee_fixed: Option<u64>,
    pub fee_percent: Option<u32>,
    pub status: TransactionStatus,
}

/// Initiate a deposit-exchange flow (buy crypto / convert one asset to another).
///
/// Returns `Err(ValidationError)` if asset codes are invalid.
/// Returns `Err(InvalidAmount)` if amount is zero.
/// Returns `Err(InvalidTransactionIntent)` if required response fields are missing.
pub fn deposit_exchange(
    req: RawDepositExchangeRequest,
    raw: RawDepositResponse,
) -> Result<DepositExchangeResponse, Error> {
    if !is_valid_asset_code(&req.source_asset) {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "Invalid source_asset code",
            &req.source_asset,
        ));
    }
    if !is_valid_asset_code(&req.destination_asset) {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "Invalid destination_asset code",
            &req.destination_asset,
        ));
    }
    if req.amount == 0 {
        return Err(Error::invalid_amount());
    }
    if raw.transaction_id.is_empty() || raw.how.is_empty() {
        return Err(Error::invalid_transaction_intent());
    }
    Ok(DepositExchangeResponse {
        transaction_id: raw.transaction_id,
        how: raw.how,
        extra_info: raw.extra_info,
        fee_fixed: raw.fee_fixed,
        fee_percent: raw.fee_percent,
        status: raw.status.as_deref().map(TransactionStatus::from_str).unwrap_or(TransactionStatus::Pending),
    })
}

// ── #651 validate_amount ──────────────────────────────────────────────────────

/// Asset limit info used by `validate_amount`.
pub struct AssetLimits {
    pub min_amount: u64,
    pub max_amount: u64,
}

/// Validate that `amount` falls within the anchor's limits for the given asset.
///
/// Returns `Err(InvalidAmount)` if amount is zero, below min, or above max.
pub fn validate_amount(amount: u64, limits: &AssetLimits) -> Result<(), Error> {
    if amount == 0 || amount < limits.min_amount || amount > limits.max_amount {
        Err(Error::invalid_amount())
    } else {
        Ok(())
    }
}

// ── #652 get_fee_estimate ─────────────────────────────────────────────────────

/// Operation type for fee estimation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeeOperation {
    Deposit,
    Withdrawal,
}

/// Anchor fee data used by `get_fee_estimate`.
pub struct AnchorFeeData {
    pub fee_fixed: u64,
    /// Fee percentage in basis points (e.g. `150` = 1.50%).
    pub fee_percent_bps: u32,
}

/// Estimated fee for a given asset, amount, and operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeEstimate {
    pub total_fee: u64,
    pub fee_fixed: u64,
    pub fee_percent_bps: u32,
}

/// Calculate the expected anchor fee for a deposit or withdrawal.
///
/// Returns `Err(ValidationError)` for invalid asset code.
/// Returns `Err(InvalidAmount)` if amount is zero.
pub fn get_fee_estimate(
    asset_code: &str,
    amount: u64,
    operation: FeeOperation,
    fee_data: &AnchorFeeData,
) -> Result<FeeEstimate, Error> {
    if !is_valid_asset_code(asset_code) {
        return Err(Error::with_context(ErrorCode::ValidationError, "Invalid asset code", asset_code));
    }
    if amount == 0 {
        return Err(Error::invalid_amount());
    }
    if fee_data.fee_percent_bps > 10_000 {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "fee_percent_bps exceeds maximum (10000)",
            "fee_percent_bps",
        ));
    }
    let _ = operation;
    let percent_fee = (amount as u128 * fee_data.fee_percent_bps as u128 / 10_000) as u64;
    let total_fee = fee_data.fee_fixed.saturating_add(percent_fee);
    Ok(FeeEstimate { total_fee, fee_fixed: fee_data.fee_fixed, fee_percent_bps: fee_data.fee_percent_bps })
}

// ── #653 get_transactions ─────────────────────────────────────────────────────

/// Filters for the SEP-6 `/transactions` endpoint.
pub struct TransactionFilters {
    pub account: String,
    pub asset_code: String,
    pub status: Option<TransactionStatus>,
    pub limit: u32,
    pub cursor: Option<String>,
}

/// Fetch and normalize a list of transactions matching the given filters.
///
/// Maps to the SEP-6 `/transactions` endpoint. Returns `Err(ValidationError)`
/// for invalid account or empty asset_code.
pub fn get_transactions(
    filters: TransactionFilters,
    raw_items: Vec<RawTransactionResponse>,
) -> Result<Vec<TransactionStatusResponse>, Error> {
    let req = RawTransactionListRequest {
        account: filters.account,
        asset_code: filters.asset_code,
        limit: filters.limit,
        cursor: filters.cursor,
    };
    let mut results = list_transactions(req, raw_items)?;
    if let Some(status_filter) = filters.status {
        results.retain(|tx| tx.status == status_filter);
    }
    Ok(results)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

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
            claimable_balance_supported: false,
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
        let resp = initiate_deposit(raw_deposit(), "USDC").unwrap();
        assert_eq!(resp.transaction_id, "txn-001");
        assert_eq!(resp.status, TransactionStatus::PendingExternal);
        assert_eq!(resp.fee_fixed, Some(1));
    }

    #[test]
    fn test_initiate_deposit_empty_asset_code_returns_error() {
        let err = initiate_deposit(raw_deposit(), "").unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_initiate_deposit_asset_code_too_long_returns_error() {
        let err = initiate_deposit(raw_deposit(), "TOOLONGASSETCODE").unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_initiate_deposit_asset_code_with_lowercase_returns_error() {
        let err = initiate_deposit(raw_deposit(), "usdc").unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_initiate_deposit_valid_asset_code_proceeds() {
        let resp = initiate_deposit(raw_deposit(), "USDC").unwrap();
        assert_eq!(resp.transaction_id, "txn-001");
    }

    #[test]
    fn test_initiate_deposit_single_char_asset_code_accepted() {
        let resp = initiate_deposit(raw_deposit(), "X").unwrap();
        assert_eq!(resp.transaction_id, "txn-001");
    }

    #[test]
    fn test_initiate_deposit_twelve_char_asset_code_accepted() {
        let resp = initiate_deposit(raw_deposit(), "LONGASSETCOD").unwrap();
        assert_eq!(resp.transaction_id, "txn-001");
    }

    #[test]
    fn test_initiate_deposit_missing_fields_returns_error() {
        let mut raw = raw_deposit();
        raw.transaction_id = "".to_string();
        assert_eq!(initiate_deposit(raw, "USDC"), Err(Error::invalid_transaction_intent()));
    }

    #[test]
    fn test_initiate_deposit_invalid_stellar_address_returns_error() {
        let mut raw = raw_deposit();
        raw.depositor_account = Some("not-a-stellar-address".to_string());
        let err = initiate_deposit(raw, "USDC").unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_initiate_deposit_lowercase_stellar_address_returns_error() {
        // 56 chars, starts with 'G', all ASCII alphanumeric — but lowercase is not
        // part of the Base32 alphabet, so this must be rejected.
        let mut raw = raw_deposit();
        raw.depositor_account = Some(("G".to_string() + &"a".repeat(55)));
        let err = initiate_deposit(raw, "USDC").unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_initiate_deposit_checksum_corrupted_stellar_address_returns_error() {
        // Same address as the valid-address test, but with one character
        // transposed, which flips the CRC16 checksum without changing the length
        // or the (still-valid) Base32 alphabet.
        let mut raw = raw_deposit();
        raw.depositor_account = Some("GAAACAQDAQCQMBYIBEFAWDANBYHRAEISCMKBKFQXDAMRUGY4DUPB7JZY".to_string());
        let err = initiate_deposit(raw, "USDC").unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_initiate_deposit_valid_stellar_address_accepted() {
        let mut raw = raw_deposit();
        // 56-char G-address with a valid Base32 encoding and CRC16 checksum
        raw.depositor_account = Some("GAAACAQDAQCQMBYIBEFAWDANBYHRAEISCMKBKFQXDAMRUGY4DUPB7JZX".to_string());
        assert!(initiate_deposit(raw, "USDC").is_ok());
    }

    #[test]
    fn test_initiate_deposit_defaults_status_to_pending() {
        let mut raw = raw_deposit();
        raw.status = None;
        let resp = initiate_deposit(raw, "USDC").unwrap();
        assert_eq!(resp.status, TransactionStatus::Pending);
    }

    #[test]
    fn test_initiate_withdrawal_normalizes_response() {
        let resp = initiate_withdrawal(raw_withdrawal(), "USDC").unwrap();
        assert_eq!(resp.transaction_id, "txn-002");
        assert_eq!(resp.status, TransactionStatus::PendingUser);
        assert_eq!(resp.memo_type, Some("id".to_string()));
        assert_eq!(resp.dest_account_id, Some("bank-account-9876".to_string()));
    }

    #[test]
    fn test_initiate_withdrawal_empty_asset_code_returns_error() {
        let err = initiate_withdrawal(raw_withdrawal(), "").unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_initiate_withdrawal_asset_code_too_long_returns_error() {
        let err = initiate_withdrawal(raw_withdrawal(), "TOOLONGASSETCODE").unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_initiate_withdrawal_valid_asset_code_proceeds() {
        let resp = initiate_withdrawal(raw_withdrawal(), "USDC").unwrap();
        assert_eq!(resp.transaction_id, "txn-002");
    }

    #[test]
    fn test_initiate_withdrawal_missing_account_returns_error() {
        let mut raw = raw_withdrawal();
        raw.account_id = "".to_string();
        assert_eq!(
            initiate_withdrawal(raw, "USDC"),
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
    fn test_transaction_status_from_str_error() {
        let status = TransactionStatus::from_str("error");
        assert_eq!(status, TransactionStatus::Error);
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
    fn test_unknown_kind_preserved_not_coerced_to_deposit() {
        let mut raw = raw_tx_status();
        raw.kind = Some("typo_kind".to_string());
        let resp = fetch_transaction_status(raw).unwrap();
        assert_eq!(resp.kind, TransactionKind::Unknown("typo_kind".to_string()));
    }

    #[test]
    fn test_transaction_kind_from_str_roundtrip() {
        assert_eq!(TransactionKind::from_str("deposit"), TransactionKind::Deposit);
        assert_eq!(TransactionKind::from_str("withdrawal"), TransactionKind::Withdrawal);
        assert_eq!(TransactionKind::from_str("withdraw"), TransactionKind::Withdrawal);
        assert_eq!(
            TransactionKind::from_str("future_value"),
            TransactionKind::Unknown("future_value".to_string())
        );
    }

    // ── get_transaction_status tests ─────────────────────────────────────

    #[test]
    fn test_get_transaction_status_200_success() {
        let raw = raw_tx_status();
        let resp = get_transaction_status(200, raw).unwrap();
        assert_eq!(resp.transaction_id, "txn-001");
        assert_eq!(resp.status, TransactionStatus::Completed);
    }

    #[test]
    fn test_get_transaction_status_404_returns_attestation_not_found() {
        let raw = raw_tx_status();
        let err = get_transaction_status(404, raw).unwrap_err();
        assert_eq!(err.code, ErrorCode::AttestationNotFound);
    }

    #[test]
    fn test_get_transaction_status_429_returns_rate_limit_exceeded() {
        let raw = raw_tx_status();
        let err = get_transaction_status(429, raw).unwrap_err();
        assert_eq!(err.code, ErrorCode::RateLimitExceeded);
    }

    #[test]
    fn test_get_transaction_status_500_returns_generic_error() {
        let raw = raw_tx_status();
        let err = get_transaction_status(500, raw).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_get_transaction_status_201_success() {
        let raw = raw_tx_status();
        let resp = get_transaction_status(201, raw).unwrap();
        assert_eq!(resp.transaction_id, "txn-001");
    }

    #[test]
    fn test_initiate_deposit_fee_percent_propagated() {
        let mut raw = raw_deposit();
        raw.fee_percent = Some(150);
        let resp = initiate_deposit(raw, "USDC").unwrap();
        assert_eq!(resp.fee_percent, Some(150));
    }

    #[test]
    fn test_initiate_deposit_claimable_balance_supported_propagated() {
        let mut raw = raw_deposit();
        raw.claimable_balance_supported = true;
        let resp = initiate_deposit(raw, "USDC").unwrap();
        assert!(resp.claimable_balance_supported);

        let raw_false = raw_deposit(); // claimable_balance_supported = false by default
        let resp_false = initiate_deposit(raw_false, "USDC").unwrap();
        assert!(!resp_false.claimable_balance_supported);
    }

    #[test]
    fn test_initiate_withdrawal_fee_percent_propagated() {
        let mut raw = raw_withdrawal();
        raw.fee_percent = Some(50);
        let resp = initiate_withdrawal(raw, "USDC").unwrap();
        assert_eq!(resp.fee_percent, Some(50));
    }

    // ── list_transactions ────────────────────────────────────────────────────

    const VALID_ACCOUNT: &str = "GAAACAQDAQCQMBYIBEFAWDANBYHRAEISCMKBKFQXDAMRUGY4DUPB7JZX";

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
    fn test_list_transactions_unknown_cursor_returns_error() {
        let items = alloc::vec![
            make_raw_tx("t1", "completed"),
            make_raw_tx("t2", "completed"),
        ];
        let mut req = base_req();
        req.cursor = Some("missing-cursor".to_string());
        let err = list_transactions(req, items).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
        assert!(err.message.contains("cursor not found"));
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

    // ── Retry logic tests ────────────────────────────────────────────────────

    #[test]
    fn test_is_http_error_retryable_5xx_errors() {
        // 5xx server errors are retryable
        assert!(is_http_error_retryable(500));
        assert!(is_http_error_retryable(502));
        assert!(is_http_error_retryable(503)); // Service Unavailable
        assert!(is_http_error_retryable(504));
        assert!(is_http_error_retryable(599));
    }

    #[test]
    fn test_is_http_error_retryable_4xx_errors_not_retryable() {
        // 4xx client errors are NOT retryable
        assert!(!is_http_error_retryable(400)); // Bad Request
        assert!(!is_http_error_retryable(401)); // Unauthorized
        assert!(!is_http_error_retryable(403)); // Forbidden
        assert!(!is_http_error_retryable(404)); // Not Found
        assert!(!is_http_error_retryable(422)); // Unprocessable Entity
        assert!(!is_http_error_retryable(499));
    }

    #[test]
    fn test_is_http_error_retryable_2xx_3xx_1xx_not_retryable() {
        // Success and redirect codes don't need retry
        assert!(!is_http_error_retryable(200));
        assert!(!is_http_error_retryable(201));
        assert!(!is_http_error_retryable(204));
        assert!(!is_http_error_retryable(301));
        assert!(!is_http_error_retryable(302));
        assert!(!is_http_error_retryable(304));
        assert!(!is_http_error_retryable(100));
    }

    #[test]
    fn test_fetch_transaction_status_with_retry_basic_flow() {
        // Test that the retry wrapper works correctly with the base function
        let raw = raw_tx_status();
        let mut sleep_calls = 0;
        let mut fetch_attempts = 0;

        let result = fetch_transaction_status_with_retry(
            |_attempt| {
                fetch_attempts += 1;
                Ok(raw.clone())
            },
            None, // Use default retry config
            |_| {
                sleep_calls += 1;
            },
        );

        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.transaction_id, "txn-001");
        assert_eq!(resp.status, TransactionStatus::Completed);
        // No retries needed for successful response
        assert_eq!(sleep_calls, 0);
        assert_eq!(fetch_attempts, 1);
    }

    #[test]
    fn test_fetch_transaction_status_with_retry_respects_config() {
        // Test that custom retry config is respected
        let raw = raw_tx_status();
        let custom_config = RetryConfig::new(
            5,      // max_attempts
            50,     // base_delay_ms
            2000,   // max_delay_ms
            2,      // backoff_multiplier
        );
        let mut sleep_calls = 0;

        let result = fetch_transaction_status_with_retry(
            |_attempt| Ok(raw.clone()),
            Some(custom_config),
            |_| {
                sleep_calls += 1;
            },
        );

        assert!(result.is_ok());
        // No failures, so no sleeps
        assert_eq!(sleep_calls, 0);
    }

    #[test]
    fn test_fetch_transaction_status_with_retry_simulates_503_then_success() {
        // Test retry on transient errors: first attempt fails, second succeeds
        let raw = raw_tx_status();
        let mut attempt_count = 0;
        let mut sleep_calls = 0;

        let result = fetch_transaction_status_with_retry(
            |_attempt| {
                attempt_count += 1;
                if attempt_count == 1 {
                    Err(Error::with_context(
                        ErrorCode::ValidationError,
                        "Transient error (simulated 503)",
                        "HTTP 503",
                    ))
                } else {
                    Ok(raw.clone())
                }
            },
            None,
            |_| sleep_calls += 1,
        );

        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.transaction_id, "txn-001");
        assert_eq!(attempt_count, 2);
        assert_eq!(sleep_calls, 1);
    }

    #[test]
    fn test_fetch_transaction_status_with_retry_error_handling() {
        // Test that normalization errors are caught after retries exhausted
        let mut raw = raw_tx_status();
        raw.transaction_id = "".to_string(); // This will cause a validation error
        let mut attempt_count = 0;

        let result = fetch_transaction_status_with_retry(
            |_attempt| {
                attempt_count += 1;
                Ok(raw.clone())
            },
            None,
            |_| {},
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, ErrorCode::InvalidTransactionIntent);
        assert_eq!(attempt_count, 1); // Normalization errors don't retry
    }

    // ── withdraw_exchange tests ──────────────────────────────────────────────

    fn raw_withdraw_exchange() -> RawWithdrawExchangeResponse {
        RawWithdrawExchangeResponse {
            transaction_id: "txn-003".to_string(),
            account_id: "GABC456".to_string(),
            dest_account_id: Some("bank-789".to_string()),
            memo: Some("99".to_string()),
            memo_type: Some("id".to_string()),
            min_amount: Some(10),
            max_amount: Some(1_000),
            fee_fixed: Some(3),
            fee_percent: Some(100),
            status: Some("pending_external".to_string()),
        }
    }

    // ── deposit_exchange tests (#650) ────────────────────────────────────────

    fn exchange_req() -> RawDepositExchangeRequest {
        RawDepositExchangeRequest {
            source_asset: "USD".to_string(),
            destination_asset: "USDC".to_string(),
            amount: 100,
        }
    }

    #[test]
    fn test_withdraw_exchange_normalizes_response() {
        let resp = withdraw_exchange(raw_withdraw_exchange(), "USDC", "BTC").unwrap();
        assert_eq!(resp.transaction_id, "txn-003");
        assert_eq!(resp.status, TransactionStatus::PendingExternal);
        assert_eq!(resp.fee_fixed, Some(3));
        assert_eq!(resp.fee_percent, Some(100));
        assert_eq!(resp.memo_type, Some("id".to_string()));
    }

    #[test]
    fn test_withdraw_exchange_invalid_source_asset_returns_error() {
        let err = withdraw_exchange(raw_withdraw_exchange(), "", "BTC").unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_withdraw_exchange_invalid_destination_asset_returns_error() {
        let err = withdraw_exchange(raw_withdraw_exchange(), "USDC", "").unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_withdraw_exchange_missing_account_id_returns_error() {
        let mut raw = raw_withdraw_exchange();
        raw.account_id = "".to_string();
        let err = withdraw_exchange(raw, "USDC", "BTC").unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidTransactionIntent);
    }

    #[test]
    fn test_withdraw_exchange_defaults_status_to_pending() {
        let mut raw = raw_withdraw_exchange();
        raw.status = None;
        let resp = withdraw_exchange(raw, "USDC", "BTC").unwrap();
        assert_eq!(resp.status, TransactionStatus::Pending);
    }

    #[test]
    fn test_deposit_exchange_success() {
        let resp = deposit_exchange(exchange_req(), raw_deposit()).unwrap();
        assert_eq!(resp.transaction_id, "txn-001");
        assert_eq!(resp.status, TransactionStatus::PendingExternal);
    }

    #[test]
    fn test_deposit_exchange_invalid_source_asset() {
        let mut req = exchange_req();
        req.source_asset = "".to_string();
        assert_eq!(deposit_exchange(req, raw_deposit()).unwrap_err().code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_deposit_exchange_invalid_destination_asset() {
        let mut req = exchange_req();
        req.destination_asset = "toolongassetcode".to_string();
        assert_eq!(deposit_exchange(req, raw_deposit()).unwrap_err().code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_deposit_exchange_zero_amount_returns_invalid_amount() {
        let mut req = exchange_req();
        req.amount = 0;
        assert_eq!(deposit_exchange(req, raw_deposit()).unwrap_err().code, ErrorCode::InvalidAmount);
    }

    #[test]
    fn test_deposit_exchange_missing_transaction_id() {
        let mut raw = raw_deposit();
        raw.transaction_id = "".to_string();
        assert_eq!(deposit_exchange(exchange_req(), raw).unwrap_err().code, ErrorCode::InvalidTransactionIntent);
    }

    // ── validate_amount tests (#651) ─────────────────────────────────────────

    #[test]
    fn test_validate_amount_within_limits() {
        let limits = AssetLimits { min_amount: 10, max_amount: 1000 };
        assert!(validate_amount(100, &limits).is_ok());
    }

    #[test]
    fn test_validate_amount_at_min() {
        let limits = AssetLimits { min_amount: 10, max_amount: 1000 };
        assert!(validate_amount(10, &limits).is_ok());
    }

    #[test]
    fn test_validate_amount_at_max() {
        let limits = AssetLimits { min_amount: 10, max_amount: 1000 };
        assert!(validate_amount(1000, &limits).is_ok());
    }

    #[test]
    fn test_validate_amount_below_min() {
        let limits = AssetLimits { min_amount: 10, max_amount: 1000 };
        assert_eq!(validate_amount(5, &limits).unwrap_err().code, ErrorCode::InvalidAmount);
    }

    #[test]
    fn test_validate_amount_above_max() {
        let limits = AssetLimits { min_amount: 10, max_amount: 1000 };
        assert_eq!(validate_amount(2000, &limits).unwrap_err().code, ErrorCode::InvalidAmount);
    }

    #[test]
    fn test_validate_amount_zero() {
        let limits = AssetLimits { min_amount: 10, max_amount: 1000 };
        assert_eq!(validate_amount(0, &limits).unwrap_err().code, ErrorCode::InvalidAmount);
    }

    // ── get_fee_estimate tests (#652) ────────────────────────────────────────

    fn fee_data() -> AnchorFeeData {
        AnchorFeeData { fee_fixed: 5, fee_percent_bps: 100 } // 1%
    }

    #[test]
    fn test_get_fee_estimate_deposit() {
        let est = get_fee_estimate("USDC", 1000, FeeOperation::Deposit, &fee_data()).unwrap();
        // fixed=5, percent=1000*100/10000=10 => total=15
        assert_eq!(est.fee_fixed, 5);
        assert_eq!(est.total_fee, 15);
    }

    #[test]
    fn test_get_fee_estimate_withdrawal() {
        let est = get_fee_estimate("USDC", 500, FeeOperation::Withdrawal, &fee_data()).unwrap();
        assert_eq!(est.total_fee, 5 + 5); // 5 fixed + 5 percent
    }

    #[test]
    fn test_get_fee_estimate_zero_amount() {
        assert_eq!(
            get_fee_estimate("USDC", 0, FeeOperation::Deposit, &fee_data()).unwrap_err().code,
            ErrorCode::InvalidAmount
        );
    }

    #[test]
    fn test_get_fee_estimate_invalid_asset_code() {
        assert_eq!(
            get_fee_estimate("", 100, FeeOperation::Deposit, &fee_data()).unwrap_err().code,
            ErrorCode::ValidationError
        );
    }

    // ── get_transactions tests (#653) ────────────────────────────────────────

    fn tx_filters() -> TransactionFilters {
        TransactionFilters {
            account: VALID_ACCOUNT.to_string(),
            asset_code: "USDC".to_string(),
            status: None,
            limit: 10,
            cursor: None,
        }
    }

    #[test]
    fn test_get_transactions_returns_all() {
        let items = alloc::vec![make_raw_tx("t1", "completed"), make_raw_tx("t2", "pending")];
        let result = get_transactions(tx_filters(), items).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_get_transactions_status_filter() {
        let items = alloc::vec![
            make_raw_tx("t1", "completed"),
            make_raw_tx("t2", "pending"),
            make_raw_tx("t3", "completed"),
        ];
        let mut f = tx_filters();
        f.status = Some(TransactionStatus::Completed);
        let result = get_transactions(f, items).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|tx| tx.status == TransactionStatus::Completed));
    }

    #[test]
    fn test_get_transactions_invalid_account() {
        let f = TransactionFilters {
            account: "bad".to_string(),
            asset_code: "USDC".to_string(),
            status: None,
            limit: 10,
            cursor: None,
        };
        assert_eq!(get_transactions(f, alloc::vec![]).unwrap_err().code, ErrorCode::ValidationError);
    }

    #[test]
    fn test_get_transactions_cursor_pagination() {
        let items = alloc::vec![
            make_raw_tx("t1", "completed"),
            make_raw_tx("t2", "completed"),
            make_raw_tx("t3", "completed"),
        ];
        let mut f = tx_filters();
        f.cursor = Some("t1".to_string());
        let result = get_transactions(f, items).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].transaction_id, "t2");
    }
}
