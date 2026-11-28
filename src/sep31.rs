//! SEP-31 Cross-Border Payments Service Layer
//!
//! Implements the sending-anchor client for SEP-31: the protocol used to send
//! payments directly between two anchors on behalf of a retail sender and
//! receiver.
//!
//! | Endpoint                              | Function                            |
//! |---------------------------------------|-------------------------------------|
//! | `GET  /info`                          | [`get_sep31_info`]                  |
//! | `POST /transactions`                  | [`initiate_sep31_transaction`]      |
//! | `GET  /transactions/:id`              | [`get_sep31_transaction`]           |
//! | `PUT  /transactions/:id`              | [`update_sep31_transaction`]        |
//! | `PUT  /transactions/:id/callback`     | [`set_sep31_callback`]              |
//!
//! # Design
//!
//! All functions in this module follow the same pattern used by [`crate::sep6`]:
//!
//! - Callers supply raw HTTP response data via `Raw*` input structs.
//! - Each function validates inputs and returns a normalized output struct.
//! - No HTTP calls are made inside this module.
//!
//! SEP-10 authentication is enforced by the caller — the SEP-10 JWT must be
//! obtained via [`crate::sep10_jwt::verify_sep10_jwt`] and passed as the
//! `Authorization: Bearer <jwt>` header on every request. Customer references
//! (`sender_id`, `receiver_id`) correspond to SEP-12 KYC identifiers.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::errors::{Error, ErrorCode};

// ── Validation helpers ────────────────────────────────────────────────────────

/// Stellar asset code: 1–12 alphanumeric characters.
fn is_valid_asset_code(s: &str) -> bool {
    let len = s.len();
    len >= 1 && len <= 12 && s.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Stellar StrKey version byte for account IDs (the 'G' prefix).
const STRKEY_VERSION_ACCOUNT_ID: u8 = 6 << 3; // 48

/// Base32 decode a 56-character Stellar StrKey string into 35 bytes.
/// Returns `None` on any non-base32 character or wrong length.
fn decode_strkey(s: &str) -> Option<[u8; 35]> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    if s.len() != 56 {
        return None;
    }
    let mut out = [0u8; 35];
    let mut buffer: u64 = 0;
    let mut bits: u32 = 0;
    let mut out_idx = 0usize;
    for byte in s.bytes() {
        let val = ALPHABET.iter().position(|&c| c == byte)? as u64;
        buffer = (buffer << 5) | val;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            if out_idx >= 35 {
                return None;
            }
            out[out_idx] = ((buffer >> bits) & 0xFF) as u8;
            out_idx += 1;
        }
    }
    Some(out)
}

/// CRC16-XModem checksum used in Stellar StrKey encoding.
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

/// Validate a full Stellar G-address (StrKey + CRC16 checksum).
fn is_valid_stellar_address(s: &str) -> bool {
    let decoded = match decode_strkey(s) {
        Some(d) => d,
        None => return false,
    };
    if decoded[0] != STRKEY_VERSION_ACCOUNT_ID {
        return false;
    }
    let payload = &decoded[..33];
    let expected = u16::from_le_bytes([decoded[33], decoded[34]]);
    crc16_xmodem(payload) == expected
}

// ── Status enum ──────────────────────────────────────────────────────────────

/// SEP-31 transaction status values as defined in the protocol specification.
///
/// Maps directly to the string values returned by an anchor's
/// `GET /transactions/:id` endpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Sep31TransactionStatus {
    /// Waiting for the sending anchor to initiate the Stellar payment.
    PendingSender,
    /// Stellar payment submitted; waiting for on-chain confirmation.
    PendingStellar,
    /// Receiver anchor needs updated KYC from the sender or receiver.
    PendingCustomerInfoUpdate,
    /// Receiver anchor needs corrected transaction fields.
    PendingTransactionInfoUpdate,
    /// Funds received on-chain; receiver anchor is processing.
    PendingReceiver,
    /// Receiver anchor is waiting for external (off-chain) delivery.
    PendingExternal,
    /// Funds successfully delivered to the receiver.
    Completed,
    /// Transaction refunded back to the sender.
    Refunded,
    /// Transaction expired before it could be completed.
    Expired,
    /// Transaction failed; see `message` for details.
    Error,
    /// Catch-all for status values not yet covered by this enum.
    Unknown(String),
}

impl Sep31TransactionStatus {
    /// Parse a raw status string from the anchor into a typed variant.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "pending_sender"                    => Self::PendingSender,
            "pending_stellar"                   => Self::PendingStellar,
            "pending_customer_info_update"      => Self::PendingCustomerInfoUpdate,
            "pending_transaction_info_update"   => Self::PendingTransactionInfoUpdate,
            "pending_receiver"                  => Self::PendingReceiver,
            "pending_external"                  => Self::PendingExternal,
            "completed"                         => Self::Completed,
            "refunded"                          => Self::Refunded,
            "expired"                           => Self::Expired,
            "error"                             => Self::Error,
            other                               => Self::Unknown(String::from(other)),
        }
    }

    /// Return the canonical string representation of this status.
    pub fn as_str(&self) -> &str {
        match self {
            Self::PendingSender                 => "pending_sender",
            Self::PendingStellar                => "pending_stellar",
            Self::PendingCustomerInfoUpdate     => "pending_customer_info_update",
            Self::PendingTransactionInfoUpdate  => "pending_transaction_info_update",
            Self::PendingReceiver               => "pending_receiver",
            Self::PendingExternal               => "pending_external",
            Self::Completed                     => "completed",
            Self::Refunded                      => "refunded",
            Self::Expired                       => "expired",
            Self::Error                         => "error",
            Self::Unknown(s)                    => s.as_str(),
        }
    }

    /// `true` when no further status transitions are expected.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Refunded | Self::Expired | Self::Error)
    }

    /// `true` when the sender or receiver must supply additional information
    /// before the transaction can proceed.
    pub fn requires_action(&self) -> bool {
        matches!(self, Self::PendingCustomerInfoUpdate | Self::PendingTransactionInfoUpdate)
    }
}

// ── Raw input structs (anchor-agnostic) ──────────────────────────────────────

/// Fields required by the sending anchor when calling `POST /transactions`.
///
/// `sender_id` and `receiver_id` are SEP-12 KYC identifiers obtained after
/// the customer has completed the KYC flow with the respective anchors.
pub struct RawSep31TransactionRequest {
    /// Amount to send in stroops (1 XLM = 10 000 000 stroops). Must be > 0.
    pub amount: u64,
    /// Asset code to send (e.g. `"USDC"`). 1–12 alphanumeric characters.
    pub asset_code: String,
    /// Optional Stellar issuer G-address for the asset.
    pub asset_issuer: Option<String>,
    /// Optional destination asset for a cross-asset payment (SEP-38 integration).
    pub destination_asset: Option<String>,
    /// Optional SEP-38 quote ID to lock in the exchange rate.
    pub quote_id: Option<String>,
    /// SEP-12 KYC identifier of the sender.
    pub sender_id: String,
    /// SEP-12 KYC identifier of the receiver.
    pub receiver_id: String,
    /// Optional arbitrary key-value fields required by the receiving anchor.
    pub fields: Option<Vec<(String, String)>>,
}

/// Raw response body from `POST /transactions`.
#[derive(Clone, Debug)]
pub struct RawSep31TransactionResponse {
    /// Anchor-assigned transaction ID.
    pub id: String,
    /// Stellar account the sending anchor should send funds to.
    pub stellar_account_id: String,
    /// Memo to attach to the Stellar payment, if required.
    pub stellar_memo: Option<String>,
    /// Memo type (`"text"`, `"id"`, or `"hash"`).
    pub stellar_memo_type: Option<String>,
}

/// Raw response body from `GET /transactions/:id`.
#[derive(Clone, Debug)]
pub struct RawSep31TransactionStatusResponse {
    /// Anchor-assigned transaction ID.
    pub id: String,
    /// Raw status string (e.g. `"pending_stellar"`).
    pub status: String,
    /// Human-readable status message from the anchor.
    pub message: Option<String>,
    /// Amount sent by the user in stroops, if reported.
    pub amount_in: Option<u64>,
    /// Amount delivered to the receiver in stroops, if reported.
    pub amount_out: Option<u64>,
    /// Fee charged in stroops, if reported.
    pub amount_fee: Option<u64>,
    /// Stellar account the sending anchor should send funds to.
    pub stellar_account_id: String,
    /// Memo to attach to the Stellar payment.
    pub stellar_memo: Option<String>,
    /// Memo type (`"text"`, `"id"`, or `"hash"`).
    pub stellar_memo_type: Option<String>,
    /// Unix timestamp when the transaction was initiated.
    pub started_at: Option<u64>,
    /// Unix timestamp when the transaction reached a terminal state.
    pub completed_at: Option<u64>,
    /// External tracking ID assigned by the receiver anchor.
    pub external_transaction_id: Option<String>,
}

/// Request body for `PUT /transactions/:id` (update transaction fields).
pub struct RawSep31UpdateRequest {
    /// Transaction ID to update.
    pub transaction_id: String,
    /// Updated fields to send to the receiver anchor.
    pub fields: Vec<(String, String)>,
}

/// Request body for `PUT /transactions/:id/callback`.
pub struct RawSep31CallbackRequest {
    /// Transaction ID to register the callback for.
    pub transaction_id: String,
    /// URL that the receiver anchor should POST status updates to (must be HTTPS).
    pub callback_url: String,
}

/// A single asset entry from `GET /info`.
#[derive(Clone, Debug)]
pub struct RawSep31Asset {
    /// Asset code (e.g. `"USDC"`).
    pub asset_code: String,
    /// SEP-12 customer types required for senders.
    pub sep12_sender_types: Vec<String>,
    /// SEP-12 customer types required for receivers.
    pub sep12_receiver_types: Vec<String>,
    /// Minimum sendable amount in stroops.
    pub min_amount: Option<u64>,
    /// Maximum sendable amount in stroops.
    pub max_amount: Option<u64>,
    /// Fixed fee in stroops.
    pub fee_fixed: Option<u64>,
    /// Percentage fee in basis points (e.g. `150` = 1.50%).
    pub fee_percent: Option<u32>,
    /// Required transaction fields for this asset.
    pub fields: Vec<RawSep31Field>,
}

/// A single field descriptor from `GET /info`.
#[derive(Clone, Debug)]
pub struct RawSep31Field {
    /// Field name (the key expected in `RawSep31TransactionRequest::fields`).
    pub name: String,
    /// Human-readable description shown in wallets.
    pub description: String,
    /// Whether this field is optional.
    pub optional: bool,
    /// Allowed values, if the anchor restricts choices.
    pub choices: Vec<String>,
}

/// Raw body of a `GET /info` response.
pub struct RawSep31InfoResponse {
    /// Assets the receiver anchor can accept.
    pub assets: Vec<RawSep31Asset>,
}

// ── Normalized output structs ─────────────────────────────────────────────────

/// Normalized response after initiating a SEP-31 transaction (`POST /transactions`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep31TransactionResponse {
    /// Anchor-assigned transaction ID.
    pub id: String,
    /// Stellar account the sending anchor must send funds to.
    pub stellar_account_id: String,
    /// Memo string to attach to the Stellar payment.
    pub stellar_memo: Option<String>,
    /// Memo type: `"text"`, `"id"`, or `"hash"`.
    pub stellar_memo_type: Option<String>,
}

/// Normalized full transaction status (from `GET /transactions/:id`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep31TransactionStatusResponse {
    /// Anchor-assigned transaction ID.
    pub id: String,
    /// Typed transaction status.
    pub status: Sep31TransactionStatus,
    /// Human-readable status message from the anchor.
    pub message: Option<String>,
    /// Amount sent in stroops, if reported.
    pub amount_in: Option<u64>,
    /// Amount delivered in stroops, if reported.
    pub amount_out: Option<u64>,
    /// Fee charged in stroops, if reported.
    pub amount_fee: Option<u64>,
    /// Stellar account the sending anchor should send funds to.
    pub stellar_account_id: String,
    /// Memo string.
    pub stellar_memo: Option<String>,
    /// Memo type.
    pub stellar_memo_type: Option<String>,
    /// Unix timestamp when the transaction was initiated.
    pub started_at: Option<u64>,
    /// Unix timestamp when the transaction reached a terminal state.
    pub completed_at: Option<u64>,
    /// External tracking ID assigned by the receiver anchor.
    pub external_transaction_id: Option<String>,
}

/// A single normalized field descriptor from `GET /info`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep31Field {
    pub name: String,
    pub description: String,
    pub optional: bool,
    pub choices: Vec<String>,
}

/// A single normalized asset entry from `GET /info`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep31Asset {
    pub asset_code: String,
    pub sep12_sender_types: Vec<String>,
    pub sep12_receiver_types: Vec<String>,
    pub min_amount: Option<u64>,
    pub max_amount: Option<u64>,
    pub fee_fixed: Option<u64>,
    pub fee_percent: Option<u32>,
    pub fields: Vec<Sep31Field>,
}

/// Normalized `GET /info` response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep31InfoResponse {
    pub assets: Vec<Sep31Asset>,
}

// ── Internal helpers ─────────────────────────────────────────────────────────

fn normalize_field(raw: RawSep31Field) -> Result<Sep31Field, Error> {
    if raw.name.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "SEP-31 field name must not be empty",
            "fields[].name",
        ));
    }
    Ok(Sep31Field {
        name: raw.name,
        description: raw.description,
        optional: raw.optional,
        choices: raw.choices,
    })
}

fn normalize_asset(raw: RawSep31Asset) -> Result<Sep31Asset, Error> {
    if !is_valid_asset_code(&raw.asset_code) {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "Invalid SEP-31 asset code",
            &raw.asset_code,
        ));
    }
    let mut fields = Vec::new();
    for f in raw.fields {
        fields.push(normalize_field(f)?);
    }
    Ok(Sep31Asset {
        asset_code: raw.asset_code,
        sep12_sender_types: raw.sep12_sender_types,
        sep12_receiver_types: raw.sep12_receiver_types,
        min_amount: raw.min_amount,
        max_amount: raw.max_amount,
        fee_fixed: raw.fee_fixed,
        fee_percent: raw.fee_percent,
        fields,
    })
}

fn validate_memo_type(memo_type: &str) -> Result<(), Error> {
    if !matches!(memo_type, "text" | "id" | "hash") {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "Invalid stellar_memo_type: must be \"text\", \"id\", or \"hash\"",
            memo_type,
        ));
    }
    Ok(())
}

fn normalize_sep31_transaction(
    raw: RawSep31TransactionResponse,
) -> Result<Sep31TransactionResponse, Error> {
    if raw.id.is_empty() {
        return Err(Error::invalid_transaction_intent());
    }
    if raw.stellar_account_id.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "SEP-31 transaction missing stellar_account_id",
            "stellar_account_id",
        ));
    }
    if !is_valid_stellar_address(&raw.stellar_account_id) {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "Invalid stellar_account_id: must be a valid Stellar G-address",
            &raw.stellar_account_id,
        ));
    }
    if let Some(ref memo_type) = raw.stellar_memo_type {
        validate_memo_type(memo_type)?;
    }
    Ok(Sep31TransactionResponse {
        id: raw.id,
        stellar_account_id: raw.stellar_account_id,
        stellar_memo: raw.stellar_memo,
        stellar_memo_type: raw.stellar_memo_type,
    })
}

fn normalize_sep31_transaction_status(
    raw: RawSep31TransactionStatusResponse,
) -> Result<Sep31TransactionStatusResponse, Error> {
    if raw.id.is_empty() {
        return Err(Error::invalid_transaction_intent());
    }
    if raw.stellar_account_id.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "SEP-31 transaction missing stellar_account_id",
            "stellar_account_id",
        ));
    }
    if !is_valid_stellar_address(&raw.stellar_account_id) {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "Invalid stellar_account_id: must be a valid Stellar G-address",
            &raw.stellar_account_id,
        ));
    }
    if let Some(ref memo_type) = raw.stellar_memo_type {
        validate_memo_type(memo_type)?;
    }
    Ok(Sep31TransactionStatusResponse {
        id: raw.id,
        status: Sep31TransactionStatus::from_str(&raw.status),
        message: raw.message,
        amount_in: raw.amount_in,
        amount_out: raw.amount_out,
        amount_fee: raw.amount_fee,
        stellar_account_id: raw.stellar_account_id,
        stellar_memo: raw.stellar_memo,
        stellar_memo_type: raw.stellar_memo_type,
        started_at: raw.started_at,
        completed_at: raw.completed_at,
        external_transaction_id: raw.external_transaction_id,
    })
}

// ── Public service functions ──────────────────────────────────────────────────

/// Normalize a raw `GET /info` response into a typed [`Sep31InfoResponse`].
///
/// Returns `Err(ValidationError)` if any asset code is invalid or if any
/// field descriptor has an empty name.
///
/// # SEP-31 reference
/// <https://stellar.org/protocol/sep-31#get-info>
pub fn get_sep31_info(raw: RawSep31InfoResponse) -> Result<Sep31InfoResponse, Error> {
    let mut assets = Vec::new();
    for a in raw.assets {
        assets.push(normalize_asset(a)?);
    }
    Ok(Sep31InfoResponse { assets })
}

/// Validate a `POST /transactions` request and normalize the anchor response.
///
/// Validates:
/// - `amount` is non-zero.
/// - `asset_code` is 1–12 alphanumeric characters.
/// - `asset_issuer`, if provided, is a valid Stellar G-address.
/// - `sender_id` and `receiver_id` are both non-empty (required SEP-12 KYC identifiers).
/// - The anchor response contains a non-empty `id` and a valid `stellar_account_id`.
/// - `stellar_memo_type`, if present, is one of `"text"`, `"id"`, or `"hash"`.
///
/// Returns `Err(InvalidAmount)` for a zero amount.
/// Returns `Err(ValidationError)` for any format violation.
/// Returns `Err(InvalidTransactionIntent)` if the anchor response is missing
/// required fields (`id` or `stellar_account_id`).
///
/// # SEP-31 reference
/// <https://stellar.org/protocol/sep-31#post-transactions>
pub fn initiate_sep31_transaction(
    req: RawSep31TransactionRequest,
    raw: RawSep31TransactionResponse,
) -> Result<Sep31TransactionResponse, Error> {
    if req.amount == 0 {
        return Err(Error::invalid_amount());
    }
    if !is_valid_asset_code(&req.asset_code) {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "Invalid asset code format: must be 1-12 alphanumeric characters",
            &req.asset_code,
        ));
    }
    if let Some(ref issuer) = req.asset_issuer {
        if !is_valid_stellar_address(issuer) {
            return Err(Error::with_context(
                ErrorCode::ValidationError,
                "Invalid asset_issuer: must be a valid Stellar G-address",
                issuer,
            ));
        }
    }
    if req.sender_id.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "sender_id must not be empty (required SEP-12 KYC identifier)",
            "sender_id",
        ));
    }
    if req.receiver_id.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "receiver_id must not be empty (required SEP-12 KYC identifier)",
            "receiver_id",
        ));
    }
    normalize_sep31_transaction(raw)
}

/// Normalize a `GET /transactions/:id` response, dispatching on HTTP status.
///
/// | HTTP status | Result                                       |
/// |-------------|----------------------------------------------|
/// | 200–299     | Normalized [`Sep31TransactionStatusResponse`]|
/// | 404         | `Err(TransactionNotFound)`                   |
/// | 408         | `Err(TransportTimeout)` (retryable)          |
/// | 429         | `Err(RateLimitExceeded)`                     |
/// | 5xx         | `Err(ServerError)` (retryable)               |
/// | other       | `Err(ValidationError)`                       |
///
/// # SEP-31 reference
/// <https://stellar.org/protocol/sep-31#get-transaction>
pub fn get_sep31_transaction(
    http_status: u32,
    raw: RawSep31TransactionStatusResponse,
) -> Result<Sep31TransactionStatusResponse, Error> {
    match http_status {
        404       => Err(Error::from_code(ErrorCode::TransactionNotFound)),
        408       => Err(Error::from_code(ErrorCode::TransportTimeout)),
        429       => Err(Error::rate_limit_exceeded()),
        500..=599 => Err(Error::from_code(ErrorCode::ServerError)),
        200..=299 => normalize_sep31_transaction_status(raw),
        _         => Err(Error::with_context(
            ErrorCode::ValidationError,
            "HTTP request failed",
            &alloc::format!("HTTP {}", http_status),
        )),
    }
}

/// Validate a `PUT /transactions/:id` update request.
///
/// Ensures the `transaction_id` is non-empty and that at least one field is
/// provided. The caller is responsible for serializing the validated fields
/// into the HTTP request body.
///
/// Returns `Err(InvalidTransactionIntent)` for a missing `transaction_id`.
/// Returns `Err(ValidationError)` for an empty fields list.
///
/// # SEP-31 reference
/// <https://stellar.org/protocol/sep-31#patch-transaction>
pub fn update_sep31_transaction(
    req: RawSep31UpdateRequest,
) -> Result<RawSep31UpdateRequest, Error> {
    if req.transaction_id.is_empty() {
        return Err(Error::invalid_transaction_intent());
    }
    if req.fields.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "update request must include at least one field",
            "fields",
        ));
    }
    Ok(req)
}

/// Validate a `PUT /transactions/:id/callback` request.
///
/// Ensures `transaction_id` is non-empty and `callback_url` starts with
/// `https://` (plain HTTP callbacks are rejected to prevent data leakage).
///
/// Returns `Err(InvalidTransactionIntent)` for a missing `transaction_id`.
/// Returns `Err(ValidationError)` for a missing or non-HTTPS callback URL.
///
/// # SEP-31 reference
/// <https://stellar.org/protocol/sep-31#post-callback>
pub fn set_sep31_callback(
    req: RawSep31CallbackRequest,
) -> Result<RawSep31CallbackRequest, Error> {
    if req.transaction_id.is_empty() {
        return Err(Error::invalid_transaction_intent());
    }
    if req.callback_url.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "callback_url must not be empty",
            "callback_url",
        ));
    }
    if !req.callback_url.starts_with("https://") {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "callback_url must use HTTPS",
            &req.callback_url,
        ));
    }
    Ok(req)
}

/// HTTP status dispatcher (callback style) for any SEP-31 response.
///
/// Calls `response_fn` only on a 2xx status; maps error codes as follows:
///
/// | HTTP status | Error                       |
/// |-------------|-----------------------------|
/// | 403         | `ComplianceNotMet`          |
/// | 404         | `TransactionNotFound`       |
/// | 408         | `TransportTimeout`          |
/// | 429         | `RateLimitExceeded`         |
/// | 5xx         | `ServerError`               |
/// | other       | `ValidationError`           |
pub fn handle_sep31_http_status<T>(
    http_status: u32,
    response_fn: impl FnOnce() -> Result<T, Error>,
) -> Result<T, Error> {
    match http_status {
        403       => Err(Error::from_code(ErrorCode::ComplianceNotMet)),
        404       => Err(Error::from_code(ErrorCode::TransactionNotFound)),
        408       => Err(Error::from_code(ErrorCode::TransportTimeout)),
        429       => Err(Error::rate_limit_exceeded()),
        500..=599 => Err(Error::from_code(ErrorCode::ServerError)),
        200..=299 => response_fn(),
        _         => Err(Error::with_context(
            ErrorCode::ValidationError,
            "HTTP request failed",
            &alloc::format!("HTTP {}", http_status),
        )),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Fixtures ─────────────────────────────────────────────────────────────

    /// Valid Stellar G-address with correct CRC16 checksum (same as sep6.rs tests).
    const VALID_ACCOUNT: &str = "GAAACAQDAQCQMBYIBEFAWDANBYHRAEISCMKBKFQXDAMRUGY4DUPB7JZX";

    fn raw_tx_request() -> RawSep31TransactionRequest {
        RawSep31TransactionRequest {
            amount: 10_000_000,
            asset_code: "USDC".to_string(),
            asset_issuer: None,
            destination_asset: None,
            quote_id: None,
            sender_id: "sender-kyc-123".to_string(),
            receiver_id: "receiver-kyc-456".to_string(),
            fields: None,
        }
    }

    fn raw_tx_response() -> RawSep31TransactionResponse {
        RawSep31TransactionResponse {
            id: "txn-sep31-001".to_string(),
            stellar_account_id: VALID_ACCOUNT.to_string(),
            stellar_memo: Some("99887766".to_string()),
            stellar_memo_type: Some("id".to_string()),
        }
    }

    fn raw_tx_status() -> RawSep31TransactionStatusResponse {
        RawSep31TransactionStatusResponse {
            id: "txn-sep31-001".to_string(),
            status: "pending_stellar".to_string(),
            message: Some("Waiting for Stellar payment.".to_string()),
            amount_in: Some(10_000_000),
            amount_out: Some(9_750_000),
            amount_fee: Some(250_000),
            stellar_account_id: VALID_ACCOUNT.to_string(),
            stellar_memo: Some("99887766".to_string()),
            stellar_memo_type: Some("id".to_string()),
            started_at: Some(1_700_000_000),
            completed_at: None,
            external_transaction_id: None,
        }
    }

    // ── Sep31TransactionStatus ───────────────────────────────────────────────

    #[test]
    fn all_known_statuses_round_trip() {
        let cases = [
            ("pending_sender",                   Sep31TransactionStatus::PendingSender),
            ("pending_stellar",                  Sep31TransactionStatus::PendingStellar),
            ("pending_customer_info_update",     Sep31TransactionStatus::PendingCustomerInfoUpdate),
            ("pending_transaction_info_update",  Sep31TransactionStatus::PendingTransactionInfoUpdate),
            ("pending_receiver",                 Sep31TransactionStatus::PendingReceiver),
            ("pending_external",                 Sep31TransactionStatus::PendingExternal),
            ("completed",                        Sep31TransactionStatus::Completed),
            ("refunded",                         Sep31TransactionStatus::Refunded),
            ("expired",                          Sep31TransactionStatus::Expired),
            ("error",                            Sep31TransactionStatus::Error),
        ];
        for (s, expected) in cases {
            let got = Sep31TransactionStatus::from_str(s);
            assert_eq!(got, expected, "from_str({s:?}) mismatch");
            assert_eq!(got.as_str(), s, "as_str() roundtrip failed for {s:?}");
        }
    }

    #[test]
    fn unknown_status_preserved() {
        let got = Sep31TransactionStatus::from_str("future_status");
        assert_eq!(got, Sep31TransactionStatus::Unknown("future_status".to_string()));
        assert_eq!(got.as_str(), "future_status");
    }

    #[test]
    fn terminal_statuses_correct() {
        assert!(Sep31TransactionStatus::Completed.is_terminal());
        assert!(Sep31TransactionStatus::Refunded.is_terminal());
        assert!(Sep31TransactionStatus::Expired.is_terminal());
        assert!(Sep31TransactionStatus::Error.is_terminal());
        assert!(!Sep31TransactionStatus::PendingSender.is_terminal());
        assert!(!Sep31TransactionStatus::PendingStellar.is_terminal());
        assert!(!Sep31TransactionStatus::PendingReceiver.is_terminal());
        assert!(!Sep31TransactionStatus::PendingExternal.is_terminal());
    }

    #[test]
    fn requires_action_statuses_correct() {
        assert!(Sep31TransactionStatus::PendingCustomerInfoUpdate.requires_action());
        assert!(Sep31TransactionStatus::PendingTransactionInfoUpdate.requires_action());
        assert!(!Sep31TransactionStatus::PendingStellar.requires_action());
        assert!(!Sep31TransactionStatus::Completed.requires_action());
    }

    // ── initiate_sep31_transaction ───────────────────────────────────────────

    #[test]
    fn initiate_normalizes_response() {
        let tx = initiate_sep31_transaction(raw_tx_request(), raw_tx_response()).unwrap();
        assert_eq!(tx.id, "txn-sep31-001");
        assert_eq!(tx.stellar_account_id, VALID_ACCOUNT);
        assert_eq!(tx.stellar_memo, Some("99887766".to_string()));
        assert_eq!(tx.stellar_memo_type, Some("id".to_string()));
    }

    #[test]
    fn initiate_zero_amount_returns_invalid_amount() {
        let mut req = raw_tx_request();
        req.amount = 0;
        let err = initiate_sep31_transaction(req, raw_tx_response()).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidAmount);
    }

    #[test]
    fn initiate_empty_asset_code_returns_validation_error() {
        let mut req = raw_tx_request();
        req.asset_code = "".to_string();
        let err = initiate_sep31_transaction(req, raw_tx_response()).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn initiate_too_long_asset_code_returns_validation_error() {
        let mut req = raw_tx_request();
        req.asset_code = "TOOLONGASSET1".to_string(); // 13 chars
        let err = initiate_sep31_transaction(req, raw_tx_response()).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn initiate_invalid_asset_issuer_returns_validation_error() {
        let mut req = raw_tx_request();
        req.asset_issuer = Some("not-a-stellar-address".to_string());
        let err = initiate_sep31_transaction(req, raw_tx_response()).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn initiate_valid_asset_issuer_accepted() {
        let mut req = raw_tx_request();
        req.asset_issuer = Some(VALID_ACCOUNT.to_string());
        assert!(initiate_sep31_transaction(req, raw_tx_response()).is_ok());
    }

    #[test]
    fn initiate_empty_sender_id_returns_validation_error() {
        let mut req = raw_tx_request();
        req.sender_id = "".to_string();
        let err = initiate_sep31_transaction(req, raw_tx_response()).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn initiate_empty_receiver_id_returns_validation_error() {
        let mut req = raw_tx_request();
        req.receiver_id = "".to_string();
        let err = initiate_sep31_transaction(req, raw_tx_response()).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn initiate_missing_response_id_returns_invalid_intent() {
        let mut resp = raw_tx_response();
        resp.id = "".to_string();
        let err = initiate_sep31_transaction(raw_tx_request(), resp).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidTransactionIntent);
    }

    #[test]
    fn initiate_invalid_stellar_account_in_response_returns_validation_error() {
        let mut resp = raw_tx_response();
        resp.stellar_account_id = "not-a-stellar-address".to_string();
        let err = initiate_sep31_transaction(raw_tx_request(), resp).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn initiate_invalid_memo_type_returns_validation_error() {
        let mut resp = raw_tx_response();
        resp.stellar_memo_type = Some("unknown_type".to_string());
        let err = initiate_sep31_transaction(raw_tx_request(), resp).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn initiate_all_valid_memo_types_accepted() {
        for memo_type in &["text", "id", "hash"] {
            let mut resp = raw_tx_response();
            resp.stellar_memo_type = Some(memo_type.to_string());
            assert!(
                initiate_sep31_transaction(raw_tx_request(), resp).is_ok(),
                "memo_type={memo_type} should be accepted"
            );
        }
    }

    #[test]
    fn initiate_no_memo_accepted() {
        let mut resp = raw_tx_response();
        resp.stellar_memo = None;
        resp.stellar_memo_type = None;
        assert!(initiate_sep31_transaction(raw_tx_request(), resp).is_ok());
    }

    // ── get_sep31_transaction ────────────────────────────────────────────────

    #[test]
    fn get_transaction_200_normalizes() {
        let result = get_sep31_transaction(200, raw_tx_status()).unwrap();
        assert_eq!(result.id, "txn-sep31-001");
        assert_eq!(result.status, Sep31TransactionStatus::PendingStellar);
        assert_eq!(result.amount_in, Some(10_000_000));
        assert_eq!(result.amount_fee, Some(250_000));
    }

    #[test]
    fn get_transaction_404_returns_not_found() {
        let err = get_sep31_transaction(404, raw_tx_status()).unwrap_err();
        assert_eq!(err.code, ErrorCode::TransactionNotFound);
    }

    #[test]
    fn get_transaction_408_returns_timeout() {
        let err = get_sep31_transaction(408, raw_tx_status()).unwrap_err();
        assert_eq!(err.code, ErrorCode::TransportTimeout);
    }

    #[test]
    fn get_transaction_429_returns_rate_limit() {
        let err = get_sep31_transaction(429, raw_tx_status()).unwrap_err();
        assert_eq!(err.code, ErrorCode::RateLimitExceeded);
    }

    #[test]
    fn get_transaction_500_returns_server_error() {
        let err = get_sep31_transaction(500, raw_tx_status()).unwrap_err();
        assert_eq!(err.code, ErrorCode::ServerError);
    }

    #[test]
    fn get_transaction_503_returns_server_error() {
        let err = get_sep31_transaction(503, raw_tx_status()).unwrap_err();
        assert_eq!(err.code, ErrorCode::ServerError);
    }

    #[test]
    fn get_transaction_302_returns_validation_error() {
        let err = get_sep31_transaction(302, raw_tx_status()).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn get_transaction_missing_id_returns_invalid_intent() {
        let mut raw = raw_tx_status();
        raw.id = "".to_string();
        let err = get_sep31_transaction(200, raw).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidTransactionIntent);
    }

    // ── update_sep31_transaction ─────────────────────────────────────────────

    #[test]
    fn update_valid_request_passes() {
        let req = RawSep31UpdateRequest {
            transaction_id: "txn-001".to_string(),
            fields: alloc::vec![("routing_number".to_string(), "123456789".to_string())],
        };
        assert!(update_sep31_transaction(req).is_ok());
    }

    #[test]
    fn update_empty_transaction_id_returns_invalid_intent() {
        let req = RawSep31UpdateRequest {
            transaction_id: "".to_string(),
            fields: alloc::vec![("field".to_string(), "value".to_string())],
        };
        let err = update_sep31_transaction(req).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidTransactionIntent);
    }

    #[test]
    fn update_empty_fields_returns_validation_error() {
        let req = RawSep31UpdateRequest {
            transaction_id: "txn-001".to_string(),
            fields: alloc::vec![],
        };
        let err = update_sep31_transaction(req).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    // ── set_sep31_callback ───────────────────────────────────────────────────

    #[test]
    fn callback_valid_https_url_passes() {
        let req = RawSep31CallbackRequest {
            transaction_id: "txn-001".to_string(),
            callback_url: "https://my-service.example.com/sep31/callback".to_string(),
        };
        assert!(set_sep31_callback(req).is_ok());
    }

    #[test]
    fn callback_empty_transaction_id_returns_invalid_intent() {
        let req = RawSep31CallbackRequest {
            transaction_id: "".to_string(),
            callback_url: "https://example.com/cb".to_string(),
        };
        let err = set_sep31_callback(req).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidTransactionIntent);
    }

    #[test]
    fn callback_empty_url_returns_validation_error() {
        let req = RawSep31CallbackRequest {
            transaction_id: "txn-001".to_string(),
            callback_url: "".to_string(),
        };
        let err = set_sep31_callback(req).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn callback_http_url_returns_validation_error() {
        let req = RawSep31CallbackRequest {
            transaction_id: "txn-001".to_string(),
            callback_url: "http://insecure.example.com/cb".to_string(),
        };
        let err = set_sep31_callback(req).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    // ── handle_sep31_http_status ─────────────────────────────────────────────

    #[test]
    fn http_status_200_calls_fn() {
        let result = handle_sep31_http_status(200, || Ok::<u32, Error>(42));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn http_status_403_returns_compliance_not_met() {
        let err = handle_sep31_http_status(403, || Ok::<u32, Error>(0)).unwrap_err();
        assert_eq!(err.code, ErrorCode::ComplianceNotMet);
    }

    #[test]
    fn http_status_404_returns_not_found() {
        let err = handle_sep31_http_status(404, || Ok::<u32, Error>(0)).unwrap_err();
        assert_eq!(err.code, ErrorCode::TransactionNotFound);
    }

    #[test]
    fn http_status_429_returns_rate_limit() {
        let err = handle_sep31_http_status(429, || Ok::<u32, Error>(0)).unwrap_err();
        assert_eq!(err.code, ErrorCode::RateLimitExceeded);
    }

    #[test]
    fn http_status_500_returns_server_error() {
        let err = handle_sep31_http_status(500, || Ok::<u32, Error>(0)).unwrap_err();
        assert_eq!(err.code, ErrorCode::ServerError);
    }

    // ── get_sep31_info ───────────────────────────────────────────────────────

    #[test]
    fn get_info_normalizes_assets() {
        let raw = RawSep31InfoResponse {
            assets: alloc::vec![RawSep31Asset {
                asset_code: "USDC".to_string(),
                sep12_sender_types: alloc::vec!["sep31-sender".to_string()],
                sep12_receiver_types: alloc::vec!["sep31-receiver".to_string()],
                min_amount: Some(1_000_000),
                max_amount: Some(100_000_000),
                fee_fixed: Some(250_000),
                fee_percent: Some(50),
                fields: alloc::vec![RawSep31Field {
                    name: "routing_number".to_string(),
                    description: "US bank routing number".to_string(),
                    optional: false,
                    choices: alloc::vec![],
                }],
            }],
        };
        let info = get_sep31_info(raw).unwrap();
        assert_eq!(info.assets.len(), 1);
        let asset = &info.assets[0];
        assert_eq!(asset.asset_code, "USDC");
        assert_eq!(asset.min_amount, Some(1_000_000));
        assert_eq!(asset.fields.len(), 1);
        assert_eq!(asset.fields[0].name, "routing_number");
        assert!(!asset.fields[0].optional);
    }

    #[test]
    fn get_info_invalid_asset_code_returns_validation_error() {
        let raw = RawSep31InfoResponse {
            assets: alloc::vec![RawSep31Asset {
                asset_code: "INVALID!".to_string(),
                sep12_sender_types: alloc::vec![],
                sep12_receiver_types: alloc::vec![],
                min_amount: None,
                max_amount: None,
                fee_fixed: None,
                fee_percent: None,
                fields: alloc::vec![],
            }],
        };
        let err = get_sep31_info(raw).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn get_info_empty_field_name_returns_validation_error() {
        let raw = RawSep31InfoResponse {
            assets: alloc::vec![RawSep31Asset {
                asset_code: "USDC".to_string(),
                sep12_sender_types: alloc::vec![],
                sep12_receiver_types: alloc::vec![],
                min_amount: None,
                max_amount: None,
                fee_fixed: None,
                fee_percent: None,
                fields: alloc::vec![RawSep31Field {
                    name: "".to_string(),
                    description: "some field".to_string(),
                    optional: true,
                    choices: alloc::vec![],
                }],
            }],
        };
        let err = get_sep31_info(raw).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn get_info_empty_asset_list_is_ok() {
        let raw = RawSep31InfoResponse { assets: alloc::vec![] };
        assert!(get_sep31_info(raw).unwrap().assets.is_empty());
    }
}
