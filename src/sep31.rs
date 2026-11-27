//! SEP-31 Cross-Border Payments API client for sending anchors.
//!
//! A sending anchor talks to a receiving anchor's `DIRECT_PAYMENT_SERVER`.
//! This module provides a small, typed, transport-agnostic client for the
//! endpoints involved in that flow:
//!
//! | Endpoint                        | Client method                       |
//! |---------------------------------|-------------------------------------|
//! | `GET /info`                     | [`Sep31Client::get_info`]           |
//! | `POST /transactions`            | [`Sep31Client::create_transaction`] |
//! | `GET /transactions/:id`         | [`Sep31Client::get_transaction`]    |
//! | `PATCH /transactions/:id`       | [`Sep31Client::patch_transaction`]  |
//! | `PUT /transactions/:id/callback`| [`Sep31Client::register_callback`]  |
//! | callback `POST`                 | [`Sep31Callback`]                   |
//!
//! # Authentication
//!
//! SEP-31 requires SEP-10 Web Authentication on every request. Callers obtain
//! a JWT through the SEP-10 flow and wrap it in [`Sep31Auth`]; the client adds
//! `Authorization: Bearer <jwt>` to each request. A missing or malformed token
//! is rejected locally, and a `401`/`403` from the anchor is surfaced as
//! [`ErrorCode::InvalidSep10Token`].
//!
//! # SEP-12 customer references
//!
//! `POST /transactions` carries the IDs returned by SEP-12 `PUT /customer`
//! for the sending and receiving clients. [`Sep31CustomerReference`] keeps the
//! customer ID together with the optional SEP-12 `type` that was used to
//! register it. [`Sep31Info`] also exposes the customer types advertised by
//! the anchor so callers know which SEP-12 type to request.
//!
//! # Transport
//!
//! The module makes no HTTP calls itself. A caller supplies a
//! [`Sep31Transport`], which keeps it usable from `std` HTTP backends as well
//! as tests using [`MockSep31Transport`]. All amounts are represented as
//! decimal strings so no floating-point rounding is introduced by the client.

extern crate alloc;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use crate::errors::{Error, ErrorCode};
use crate::validate_anchor_domain;

// ── Status ────────────────────────────────────────────────────────────────────

/// SEP-31 transaction status values.
///
/// The known variants are the statuses defined by SEP-31.
/// [`Unknown`](Self::Unknown) preserves a new value returned by an anchor so
/// parsing never discards information.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Sep31Status {
    /// Awaiting the Stellar payment from the sending anchor.
    PendingSender,
    /// Submitted to Stellar but not yet confirmed.
    PendingStellar,
    /// SEP-9 KYC values need to be updated through SEP-12.
    PendingCustomerInfoUpdate,
    /// Deprecated per-transaction `fields` need to be updated through PATCH.
    PendingTransactionInfoUpdate,
    /// The receiving anchor is processing the payment.
    PendingReceiver,
    /// Submitted to the external network but not yet confirmed.
    PendingExternal,
    /// Funds were delivered to the receiving client.
    Completed,
    /// Funds were refunded to the sending anchor.
    Refunded,
    /// The transaction was abandoned by the sending client.
    Expired,
    /// Catch-all error status.
    Error,
    /// A status value not yet known to this client.
    Unknown(String),
}

impl Sep31Status {
    /// Parse a SEP-31 status string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.trim() {
            "pending_sender" => Self::PendingSender,
            "pending_stellar" => Self::PendingStellar,
            "pending_customer_info_update" => Self::PendingCustomerInfoUpdate,
            "pending_transaction_info_update" => Self::PendingTransactionInfoUpdate,
            "pending_receiver" => Self::PendingReceiver,
            "pending_external" => Self::PendingExternal,
            "completed" => Self::Completed,
            "refunded" => Self::Refunded,
            "expired" => Self::Expired,
            "error" => Self::Error,
            _ => Self::Unknown(String::from(s)),
        }
    }

    /// Canonical SEP-31 wire representation of this status.
    pub fn as_str(&self) -> &str {
        match self {
            Self::PendingSender => "pending_sender",
            Self::PendingStellar => "pending_stellar",
            Self::PendingCustomerInfoUpdate => "pending_customer_info_update",
            Self::PendingTransactionInfoUpdate => "pending_transaction_info_update",
            Self::PendingReceiver => "pending_receiver",
            Self::PendingExternal => "pending_external",
            Self::Completed => "completed",
            Self::Refunded => "refunded",
            Self::Expired => "expired",
            Self::Error => "error",
            Self::Unknown(s) => s.as_str(),
        }
    }

    /// `true` when no further status change is expected.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Refunded | Self::Expired | Self::Error
        )
    }

    /// `true` when the sending anchor must collect updated KYC or transaction
    /// information before the payment can continue.
    pub fn requires_info_update(&self) -> bool {
        matches!(
            self,
            Self::PendingCustomerInfoUpdate | Self::PendingTransactionInfoUpdate
        )
    }
}

// ── Info ──────────────────────────────────────────────────────────────────────

/// An asset the receiving anchor accepts in `GET /info`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep31AssetInfo {
    pub asset_code: String,
    pub funding_methods: Vec<String>,
    pub min_amount: Option<String>,
    pub max_amount: Option<String>,
    pub fee_fixed: Option<String>,
    pub fee_percent: Option<String>,
    pub quotes_supported: bool,
    pub quotes_required: bool,
}

impl Sep31AssetInfo {
    pub fn new(asset_code: &str, funding_methods: &[&str]) -> Self {
        Self {
            asset_code: String::from(asset_code),
            funding_methods: funding_methods.iter().map(|s| String::from(*s)).collect(),
            min_amount: None,
            max_amount: None,
            fee_fixed: None,
            fee_percent: None,
            quotes_supported: false,
            quotes_required: false,
        }
    }

    pub fn with_limits(mut self, min: Option<&str>, max: Option<&str>) -> Self {
        self.min_amount = min.map(String::from);
        self.max_amount = max.map(String::from);
        self
    }
}

/// Typed result of `GET /info`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep31Info {
    /// Stellar assets the receiving anchor can receive.
    pub receive: Vec<Sep31AssetInfo>,
    /// SEP-12 customer types accepted for the sending client.
    pub sender_customer_types: Vec<String>,
    /// SEP-12 customer types accepted for the receiving client.
    pub receiver_customer_types: Vec<String>,
}

impl Sep31Info {
    /// `true` when the anchor advertises a SEP-12 type for the given role.
    pub fn requires_customer(&self, sender: bool) -> bool {
        if sender {
            !self.sender_customer_types.is_empty()
        } else {
            !self.receiver_customer_types.is_empty()
        }
    }
}

/// Raw `GET /info` body returned by a [`Sep31Transport`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawSep31InfoResponse {
    pub status_code: u16,
    pub receive: Vec<Sep31AssetInfo>,
    pub sender_customer_types: Vec<String>,
    pub receiver_customer_types: Vec<String>,
}

impl RawSep31InfoResponse {
    pub fn new(receive: Vec<Sep31AssetInfo>) -> Self {
        Self {
            status_code: 200,
            receive,
            sender_customer_types: Vec::new(),
            receiver_customer_types: Vec::new(),
        }
    }

    pub fn with_customer_types(mut self, sender_types: &[&str], receiver_types: &[&str]) -> Self {
        self.sender_customer_types = sender_types.iter().map(|s| String::from(*s)).collect();
        self.receiver_customer_types = receiver_types.iter().map(|s| String::from(*s)).collect();
        self
    }

    pub fn with_status_code(mut self, status_code: u16) -> Self {
        self.status_code = status_code;
        self
    }
}

// ── Requests ──────────────────────────────────────────────────────────────────

/// A SEP-12 customer ID referenced by a SEP-31 transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep31CustomerReference {
    /// The `id` returned by SEP-12 `PUT /customer`.
    pub id: String,
    /// The SEP-12 `type` used to register the customer, when known.
    pub customer_type: Option<String>,
}

impl Sep31CustomerReference {
    pub fn new(id: &str) -> Self {
        Self {
            id: String::from(id),
            customer_type: None,
        }
    }

    pub fn with_type(id: &str, customer_type: &str) -> Self {
        Self {
            id: String::from(id),
            customer_type: Some(String::from(customer_type)),
        }
    }
}

/// Body of a SEP-31 `POST /transactions` request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep31TransactionRequest {
    pub amount: String,
    pub asset_code: String,
    pub funding_method: String,
    pub asset_issuer: Option<String>,
    pub destination_asset: Option<String>,
    pub quote_id: Option<String>,
    pub sender: Option<Sep31CustomerReference>,
    pub receiver: Option<Sep31CustomerReference>,
    pub refund_memo: Option<String>,
    pub refund_memo_type: Option<String>,
    pub fields: Vec<(String, String)>,
}

impl Sep31TransactionRequest {
    /// Create a request for a plain (non-quoted) payment.
    pub fn new(amount: &str, asset_code: &str, funding_method: &str) -> Self {
        Self {
            amount: String::from(amount),
            asset_code: String::from(asset_code),
            funding_method: String::from(funding_method),
            asset_issuer: None,
            destination_asset: None,
            quote_id: None,
            sender: None,
            receiver: None,
            refund_memo: None,
            refund_memo_type: None,
            fields: Vec::new(),
        }
    }

    pub fn with_asset_issuer(mut self, issuer: &str) -> Self {
        self.asset_issuer = Some(String::from(issuer));
        self
    }

    pub fn with_destination_asset(mut self, asset: &str) -> Self {
        self.destination_asset = Some(String::from(asset));
        self
    }

    pub fn with_quote_id(mut self, quote_id: &str) -> Self {
        self.quote_id = Some(String::from(quote_id));
        self
    }

    /// Attach the SEP-12 ID of the sending client.
    pub fn with_sender(mut self, customer: Sep31CustomerReference) -> Self {
        self.sender = Some(customer);
        self
    }

    /// Attach the SEP-12 ID of the receiving client.
    pub fn with_receiver(mut self, customer: Sep31CustomerReference) -> Self {
        self.receiver = Some(customer);
        self
    }

    pub fn with_sender_id(self, id: &str) -> Self {
        self.with_sender(Sep31CustomerReference::new(id))
    }

    pub fn with_receiver_id(self, id: &str) -> Self {
        self.with_receiver(Sep31CustomerReference::new(id))
    }

    pub fn with_refund_memo(mut self, memo: &str, memo_type: &str) -> Self {
        self.refund_memo = Some(String::from(memo));
        self.refund_memo_type = Some(String::from(memo_type));
        self
    }

    /// Add a deprecated per-transaction `fields` entry.
    pub fn with_field(mut self, name: &str, value: &str) -> Self {
        self.fields.push((String::from(name), String::from(value)));
        self
    }

    /// Validate the request locally, before a transport call is made.
    pub fn validate(&self) -> Result<(), Error> {
        if !is_valid_decimal(&self.amount) {
            return Err(invalid_field("amount"));
        }
        if !is_valid_asset_code(&self.asset_code) {
            return Err(invalid_field("asset_code"));
        }
        if self.funding_method.is_empty() {
            return Err(invalid_field("funding_method"));
        }
        if let Some(issuer) = &self.asset_issuer {
            if !is_valid_stellar_address(issuer) {
                return Err(invalid_field("asset_issuer"));
            }
        }
        if let Some(customer) = &self.sender {
            validate_customer_reference(customer, "sender.id")?;
        }
        if let Some(customer) = &self.receiver {
            validate_customer_reference(customer, "receiver.id")?;
        }
        match (&self.refund_memo, &self.refund_memo_type) {
            (None, None) => {}
            (Some(memo), Some(memo_type)) => {
                if memo.is_empty() || !is_valid_memo_type(memo_type) {
                    return Err(invalid_field("refund_memo"));
                }
            }
            _ => return Err(invalid_field("refund_memo_type")),
        }
        for (name, _) in &self.fields {
            if name.is_empty() {
                return Err(invalid_field("fields"));
            }
        }
        Ok(())
    }
}

/// A request handed to a [`Sep31Transport`].
///
/// Every variant carries a well-formed `Authorization: Bearer ...` value,
/// built from a validated [`Sep31Auth`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Sep31Request {
    Info {
        lang: Option<String>,
        authorization: String,
    },
    CreateTransaction {
        request: Box<Sep31TransactionRequest>,
        authorization: String,
    },
    GetTransaction {
        id: String,
        authorization: String,
    },
    PatchTransaction {
        id: String,
        fields: Vec<(String, String)>,
        authorization: String,
    },
    RegisterCallback {
        id: String,
        url: String,
        authorization: String,
    },
}

impl Sep31Request {
    pub fn info(jwt: &str, lang: Option<&str>) -> Self {
        Self::Info {
            lang: lang.map(String::from),
            authorization: bearer_header(jwt),
        }
    }

    pub fn create(request: &Sep31TransactionRequest, jwt: &str) -> Self {
        Self::CreateTransaction {
            request: Box::new(request.clone()),
            authorization: bearer_header(jwt),
        }
    }

    pub fn get(id: &str, jwt: &str) -> Self {
        Self::GetTransaction {
            id: String::from(id),
            authorization: bearer_header(jwt),
        }
    }

    pub fn patch(id: &str, fields: &[(&str, &str)], jwt: &str) -> Self {
        Self::PatchTransaction {
            id: String::from(id),
            fields: fields
                .iter()
                .map(|(name, value)| (String::from(*name), String::from(*value)))
                .collect(),
            authorization: bearer_header(jwt),
        }
    }

    pub fn register_callback(id: &str, url: &str, jwt: &str) -> Self {
        Self::RegisterCallback {
            id: String::from(id),
            url: String::from(url),
            authorization: bearer_header(jwt),
        }
    }

    /// The `Authorization` header value carried by this request.
    pub fn authorization(&self) -> &str {
        match self {
            Self::Info { authorization, .. }
            | Self::CreateTransaction { authorization, .. }
            | Self::GetTransaction { authorization, .. }
            | Self::PatchTransaction { authorization, .. }
            | Self::RegisterCallback { authorization, .. } => authorization,
        }
    }

    /// HTTP method used by this endpoint.
    pub fn method(&self) -> &'static str {
        match self {
            Self::Info { .. } | Self::GetTransaction { .. } => "GET",
            Self::CreateTransaction { .. } => "POST",
            Self::PatchTransaction { .. } => "PATCH",
            Self::RegisterCallback { .. } => "PUT",
        }
    }

    /// Request path relative to the receiving anchor's `DIRECT_PAYMENT_SERVER`.
    pub fn path(&self) -> String {
        match self {
            Self::Info { .. } => String::from("/info"),
            Self::CreateTransaction { .. } => String::from("/transactions"),
            Self::GetTransaction { id, .. } | Self::PatchTransaction { id, .. } => {
                format!("/transactions/{id}")
            }
            Self::RegisterCallback { id, .. } => format!("/transactions/{id}/callback"),
        }
    }

    /// The transaction ID in the path, when the request targets a transaction.
    pub fn transaction_id(&self) -> Option<&str> {
        match self {
            Self::GetTransaction { id, .. }
            | Self::PatchTransaction { id, .. }
            | Self::RegisterCallback { id, .. } => Some(id.as_str()),
            _ => None,
        }
    }
}

// ── Responses ─────────────────────────────────────────────────────────────────

/// Result of a successful `POST /transactions` request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep31CreatedTransaction {
    pub id: String,
    pub stellar_account_id: Option<String>,
    pub stellar_memo: Option<String>,
    pub stellar_memo_type: Option<String>,
}

impl Sep31CreatedTransaction {
    /// `true` once the anchor has supplied the Stellar payment instructions.
    pub fn is_ready_to_pay(&self) -> bool {
        self.stellar_account_id.is_some()
            && self.stellar_memo.is_some()
            && self.stellar_memo_type.is_some()
    }
}

/// Typed SEP-31 transaction from `GET`/`PATCH /transactions/:id`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep31Transaction {
    pub id: String,
    pub status: Sep31Status,
    pub status_eta: Option<u64>,
    pub status_message: Option<String>,
    pub amount_in: Option<String>,
    pub amount_in_asset: Option<String>,
    pub amount_out: Option<String>,
    pub amount_out_asset: Option<String>,
    pub amount_fee: Option<String>,
    pub quote_id: Option<String>,
    pub stellar_account_id: Option<String>,
    pub stellar_memo: Option<String>,
    pub stellar_memo_type: Option<String>,
    pub stellar_transaction_id: Option<String>,
    pub external_transaction_id: Option<String>,
    pub started_at: Option<String>,
    pub updated_at: Option<String>,
    pub completed_at: Option<String>,
    pub required_info_message: Option<String>,
}

/// Raw `POST /transactions` body returned by a [`Sep31Transport`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawSep31CreationResponse {
    pub status_code: u16,
    pub id: String,
    pub stellar_account_id: Option<String>,
    pub stellar_memo: Option<String>,
    pub stellar_memo_type: Option<String>,
}

impl RawSep31CreationResponse {
    pub fn new(id: &str) -> Self {
        Self {
            status_code: 201,
            id: String::from(id),
            stellar_account_id: None,
            stellar_memo: None,
            stellar_memo_type: None,
        }
    }

    pub fn with_stellar(mut self, account_id: &str, memo: &str, memo_type: &str) -> Self {
        self.stellar_account_id = Some(String::from(account_id));
        self.stellar_memo = Some(String::from(memo));
        self.stellar_memo_type = Some(String::from(memo_type));
        self
    }

    pub fn with_status_code(mut self, status_code: u16) -> Self {
        self.status_code = status_code;
        self
    }
}

/// Raw transaction body returned by a [`Sep31Transport`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawSep31Transaction {
    pub status_code: u16,
    pub id: String,
    pub status: String,
    pub status_eta: Option<u64>,
    pub status_message: Option<String>,
    pub amount_in: Option<String>,
    pub amount_in_asset: Option<String>,
    pub amount_out: Option<String>,
    pub amount_out_asset: Option<String>,
    pub amount_fee: Option<String>,
    pub quote_id: Option<String>,
    pub stellar_account_id: Option<String>,
    pub stellar_memo: Option<String>,
    pub stellar_memo_type: Option<String>,
    pub stellar_transaction_id: Option<String>,
    pub external_transaction_id: Option<String>,
    pub started_at: Option<String>,
    pub updated_at: Option<String>,
    pub completed_at: Option<String>,
    pub required_info_message: Option<String>,
}

impl RawSep31Transaction {
    pub fn new(id: &str, status: Sep31Status) -> Self {
        Self {
            status_code: 200,
            id: String::from(id),
            status: String::from(status.as_str()),
            status_eta: None,
            status_message: None,
            amount_in: None,
            amount_in_asset: None,
            amount_out: None,
            amount_out_asset: None,
            amount_fee: None,
            quote_id: None,
            stellar_account_id: None,
            stellar_memo: None,
            stellar_memo_type: None,
            stellar_transaction_id: None,
            external_transaction_id: None,
            started_at: None,
            updated_at: None,
            completed_at: None,
            required_info_message: None,
        }
    }

    pub fn with_status_code(mut self, status_code: u16) -> Self {
        self.status_code = status_code;
        self
    }

    pub fn with_amounts(mut self, amount_in: &str, amount_out: &str, fee: &str) -> Self {
        self.amount_in = Some(String::from(amount_in));
        self.amount_out = Some(String::from(amount_out));
        self.amount_fee = Some(String::from(fee));
        self
    }

    pub fn with_stellar(mut self, account_id: &str, memo: &str, memo_type: &str) -> Self {
        self.stellar_account_id = Some(String::from(account_id));
        self.stellar_memo = Some(String::from(memo));
        self.stellar_memo_type = Some(String::from(memo_type));
        self
    }

    pub fn with_message(mut self, message: &str) -> Self {
        self.status_message = Some(String::from(message));
        self
    }
}

/// A response returned by a [`Sep31Transport`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Sep31Response {
    Info(RawSep31InfoResponse),
    Created(RawSep31CreationResponse),
    /// Boxed because a transaction body is considerably larger than the other
    /// response variants.
    Transaction(Box<RawSep31Transaction>),
    CallbackRegistered {
        status_code: u16,
    },
}

impl Sep31Response {
    pub fn info(receive: Vec<Sep31AssetInfo>) -> Self {
        Self::Info(RawSep31InfoResponse::new(receive))
    }

    pub fn created(id: &str) -> Self {
        Self::Created(RawSep31CreationResponse::new(id))
    }

    pub fn transaction(id: &str, status: Sep31Status) -> Self {
        Self::Transaction(Box::new(RawSep31Transaction::new(id, status)))
    }

    pub fn callback_registered() -> Self {
        Self::CallbackRegistered { status_code: 204 }
    }
}

/// Abstraction over the HTTP exchange for SEP-31 endpoints.
pub trait Sep31Transport {
    fn send(&mut self, request: Sep31Request) -> Result<Sep31Response, Error>;
}

// ── Authentication ────────────────────────────────────────────────────────────

/// A SEP-10 JWT used to authenticate SEP-31 requests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep31Auth {
    jwt: String,
}

impl Sep31Auth {
    /// Wrap a SEP-10 JWT.
    ///
    /// Accepts either a bare token or a value already prefixed with `Bearer `.
    /// Empty tokens and tokens containing whitespace are rejected before any
    /// transport call is made.
    pub fn new(jwt: &str) -> Result<Self, Error> {
        let token = jwt.strip_prefix("Bearer ").unwrap_or(jwt).trim();
        if token.is_empty() || token.bytes().any(|b| b.is_ascii_whitespace()) {
            return Err(Error::with_context(
                ErrorCode::InvalidSep10Token,
                "SEP-10 JWT must be a non-empty, whitespace-free token",
                "authorization",
            ));
        }
        Ok(Self {
            jwt: String::from(token),
        })
    }

    /// The raw JWT value, without the `Bearer ` prefix.
    pub fn jwt(&self) -> &str {
        &self.jwt
    }

    /// The complete `Authorization` header value.
    pub fn authorization_header(&self) -> String {
        bearer_header(&self.jwt)
    }
}

fn bearer_header(jwt: &str) -> String {
    format!("Bearer {}", jwt)
}

// ── Client ────────────────────────────────────────────────────────────────────

/// Transport-agnostic SEP-31 sending-anchor client.
pub struct Sep31Client<T: Sep31Transport> {
    transport: T,
    auth: Sep31Auth,
}

impl<T: Sep31Transport> Sep31Client<T> {
    pub fn new(transport: T, auth: Sep31Auth) -> Self {
        Self { transport, auth }
    }

    /// The SEP-10 authentication attached to every request.
    pub fn auth(&self) -> &Sep31Auth {
        &self.auth
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// `GET /info` — supported assets and SEP-12 customer types.
    pub fn get_info(&mut self, lang: Option<&str>) -> Result<Sep31Info, Error> {
        if let Some(lang) = lang {
            validate_lang(lang)?;
        }
        let request = Sep31Request::info(self.auth.jwt(), lang);
        match self.transport.send(request)? {
            Sep31Response::Info(raw) => {
                ensure_success(raw.status_code)?;
                Ok(Sep31Info {
                    receive: raw.receive,
                    sender_customer_types: raw.sender_customer_types,
                    receiver_customer_types: raw.receiver_customer_types,
                })
            }
            other => Err(unexpected_response(other)),
        }
    }

    /// `POST /transactions` — create a transaction with the receiving anchor.
    pub fn create_transaction(
        &mut self,
        request: &Sep31TransactionRequest,
    ) -> Result<Sep31CreatedTransaction, Error> {
        request.validate()?;
        let wire_request = Sep31Request::create(request, self.auth.jwt());
        match self.transport.send(wire_request)? {
            Sep31Response::Created(raw) => {
                ensure_success(raw.status_code)?;
                if raw.id.is_empty() {
                    return Err(Error::validation_error(
                        "SEP-31 creation response is missing transaction id",
                    ));
                }
                Ok(Sep31CreatedTransaction {
                    id: raw.id,
                    stellar_account_id: raw.stellar_account_id,
                    stellar_memo: raw.stellar_memo,
                    stellar_memo_type: raw.stellar_memo_type,
                })
            }
            other => Err(unexpected_response(other)),
        }
    }

    /// `GET /transactions/:id` — fetch the current transaction state.
    pub fn get_transaction(&mut self, id: &str) -> Result<Sep31Transaction, Error> {
        validate_transaction_id(id)?;
        let request = Sep31Request::get(id, self.auth.jwt());
        transaction_from_response(self.transport.send(request)?)
    }

    /// `PATCH /transactions/:id` — supply transaction fields requested by the
    /// receiving anchor while the status is `pending_transaction_info_update`.
    pub fn patch_transaction(
        &mut self,
        id: &str,
        fields: &[(&str, &str)],
    ) -> Result<Sep31Transaction, Error> {
        validate_transaction_id(id)?;
        if fields.is_empty() || fields.iter().any(|(name, _)| name.is_empty()) {
            return Err(Error::validation_error(
                "SEP-31 PATCH requires at least one named field",
            ));
        }
        let request = Sep31Request::patch(id, fields, self.auth.jwt());
        transaction_from_response(self.transport.send(request)?)
    }

    /// `PUT /transactions/:id/callback` — register the HTTPS URL the receiving
    /// anchor should POST status updates to.
    pub fn register_callback(&mut self, id: &str, url: &str) -> Result<(), Error> {
        validate_transaction_id(id)?;
        // SEP-31 is HTTPS-only; the callback URL is supplied by us, so make
        // sure we never register an insecure or malformed endpoint.
        validate_anchor_domain(url)?;
        let request = Sep31Request::register_callback(id, url, self.auth.jwt());
        match self.transport.send(request)? {
            Sep31Response::CallbackRegistered { status_code } => ensure_success(status_code),
            other => Err(unexpected_response(other)),
        }
    }
}

fn transaction_from_response(response: Sep31Response) -> Result<Sep31Transaction, Error> {
    match response {
        Sep31Response::Transaction(raw) => {
            ensure_success(raw.status_code)?;
            normalize_transaction(*raw)
        }
        other => Err(unexpected_response(other)),
    }
}

fn normalize_transaction(raw: RawSep31Transaction) -> Result<Sep31Transaction, Error> {
    if raw.id.is_empty() {
        return Err(Error::validation_error(
            "SEP-31 transaction response is missing id",
        ));
    }
    Ok(Sep31Transaction {
        id: raw.id,
        status: Sep31Status::from_str(&raw.status),
        status_eta: raw.status_eta,
        status_message: raw.status_message,
        amount_in: raw.amount_in,
        amount_in_asset: raw.amount_in_asset,
        amount_out: raw.amount_out,
        amount_out_asset: raw.amount_out_asset,
        amount_fee: raw.amount_fee,
        quote_id: raw.quote_id,
        stellar_account_id: raw.stellar_account_id,
        stellar_memo: raw.stellar_memo,
        stellar_memo_type: raw.stellar_memo_type,
        stellar_transaction_id: raw.stellar_transaction_id,
        external_transaction_id: raw.external_transaction_id,
        started_at: raw.started_at,
        updated_at: raw.updated_at,
        completed_at: raw.completed_at,
        required_info_message: raw.required_info_message,
    })
}

fn ensure_success(status_code: u16) -> Result<(), Error> {
    if (200..=299).contains(&status_code) {
        Ok(())
    } else {
        Err(http_error(status_code))
    }
}

fn unexpected_response(response: Sep31Response) -> Error {
    let name = match response {
        Sep31Response::Info(_) => "info",
        Sep31Response::Created(_) => "created",
        Sep31Response::Transaction(_) => "transaction",
        Sep31Response::CallbackRegistered { .. } => "callback_registered",
    };
    Error::with_context(
        ErrorCode::ValidationError,
        "unexpected SEP-31 transport response",
        name,
    )
}

fn http_error(status_code: u16) -> Error {
    match status_code {
        400 | 422 => Error::validation_error("SEP-31 request was rejected by the anchor"),
        401 | 403 => Error::invalid_sep10_token(),
        404 => Error::with_context(
            ErrorCode::TransactionNotFound,
            "SEP-31 transaction not found",
            "transaction",
        ),
        408 => Error::from_code(ErrorCode::TransportTimeout),
        429 => Error::rate_limit_exceeded(),
        500..=599 => Error::from_code(ErrorCode::ServerError),
        _ => Error::with_context(
            ErrorCode::ValidationError,
            "unexpected SEP-31 HTTP status",
            &status_code.to_string(),
        ),
    }
}

// ── Callback handling ─────────────────────────────────────────────────────────

/// A parsed SEP-31 callback POST from a receiving anchor.
///
/// The receiving anchor signs `<timestamp>.<host>.<body>` with the Ed25519
/// key from its `SIGNING_KEY` stellar.toml entry and sends the signature in
/// `Signature: t=<unix timestamp>, s=<base64 signature>`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep31Callback {
    pub transaction: Sep31Transaction,
    pub timestamp: u64,
    /// Base64-encoded Ed25519 signature from the callback header.
    pub signature: String,
    /// The sending-anchor host that was included in the signed payload.
    pub host: String,
}

impl Sep31Callback {
    /// Parse and validate a callback request.
    ///
    /// `signature_header` is the value of the `Signature` (or deprecated
    /// `X-Stellar-Signature`) header. `now` and `max_age_seconds` let callers
    /// enforce freshness and reject replays.
    pub fn parse(
        signature_header: &str,
        host: &str,
        body: RawSep31Transaction,
        now: u64,
        max_age_seconds: u64,
    ) -> Result<Self, Error> {
        if host.is_empty() || max_age_seconds == 0 {
            return Err(Error::validation_error(
                "callback host and maximum age are required",
            ));
        }
        let (timestamp, signature) = parse_signature_header(signature_header)?;
        if timestamp > now {
            return Err(Error::with_context(
                ErrorCode::InvalidTimestamp,
                "SEP-31 callback timestamp is in the future",
                "callback",
            ));
        }
        if now - timestamp > max_age_seconds {
            return Err(Error::with_context(
                ErrorCode::ReplayAttack,
                "SEP-31 callback timestamp is too old",
                "callback",
            ));
        }
        let transaction = normalize_transaction(body)?;
        Ok(Self {
            transaction,
            timestamp,
            signature: String::from(signature),
            host: String::from(host),
        })
    }

    /// Rebuild the exact string the receiving anchor signed.
    ///
    /// `body_json` must be the raw request body bytes, before JSON parsing.
    pub fn signing_payload(&self, body_json: &str) -> String {
        format!("{}.{}.{}", self.timestamp, self.host, body_json)
    }

    /// Verify the callback signature with the receiving anchor's 32-byte
    /// Ed25519 `SIGNING_KEY`.
    pub fn verify(&self, body_json: &str, signing_key: &[u8; 32]) -> Result<(), Error> {
        let decoded = BASE64_STANDARD
            .decode(self.signature.as_str())
            .map_err(|_| {
                Error::with_context(
                    ErrorCode::InvalidSep10Token,
                    "SEP-31 callback signature is not valid base64",
                    "signature",
                )
            })?;
        if decoded.len() != 64 {
            return Err(Error::with_context(
                ErrorCode::InvalidSep10Token,
                "SEP-31 callback signature must be 64 bytes",
                "signature",
            ));
        }
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&decoded);
        let signature = Signature::from_bytes(&bytes);
        let verifying_key = VerifyingKey::from_bytes(signing_key).map_err(|_| {
            Error::with_context(
                ErrorCode::InvalidSep10Token,
                "invalid receiving anchor signing key",
                "signing_key",
            )
        })?;
        let payload = self.signing_payload(body_json);
        verifying_key
            .verify(payload.as_bytes(), &signature)
            .map_err(|_| {
                Error::with_context(
                    ErrorCode::InvalidSep10Token,
                    "SEP-31 callback signature verification failed",
                    "signature",
                )
            })
    }
}

/// Pick the callback signature header, preferring `Signature` over the
/// deprecated `X-Stellar-Signature`.
pub fn callback_signature_header<'a>(
    signature: Option<&'a str>,
    legacy: Option<&'a str>,
) -> Result<&'a str, Error> {
    signature
        .or(legacy)
        .filter(|header| !header.trim().is_empty())
        .ok_or_else(|| {
            Error::with_context(
                ErrorCode::InvalidSep10Token,
                "SEP-31 callback signature header is missing",
                "signature",
            )
        })
}

/// Parse `t=<timestamp>, s=<base64 signature>`.
fn parse_signature_header(header: &str) -> Result<(u64, &str), Error> {
    let mut timestamp = None;
    let mut signature = None;
    for part in header.split(',') {
        let mut pair = part.trim().splitn(2, '=');
        let key = pair.next().unwrap_or("").trim();
        let value = pair.next().unwrap_or("").trim();
        match key {
            "t" => {
                timestamp = Some(value.parse::<u64>().map_err(|_| {
                    Error::with_context(
                        ErrorCode::ValidationError,
                        "callback timestamp must be a Unix timestamp",
                        "t",
                    )
                })?);
            }
            "s" => {
                if value.is_empty() {
                    return Err(Error::with_context(
                        ErrorCode::InvalidSep10Token,
                        "callback signature is empty",
                        "s",
                    ));
                }
                signature = Some(value);
            }
            _ => {}
        }
    }
    match (timestamp, signature) {
        (Some(t), Some(s)) => Ok((t, s)),
        _ => Err(Error::with_context(
            ErrorCode::InvalidSep10Token,
            "callback header must contain t and s",
            "signature",
        )),
    }
}

// ── Validation ────────────────────────────────────────────────────────────────

fn invalid_field(field: &str) -> Error {
    Error::with_context(
        ErrorCode::ValidationError,
        "invalid SEP-31 request field",
        field,
    )
}

fn validate_transaction_id(id: &str) -> Result<(), Error> {
    if id.is_empty()
        || id.len() > 128
        || id
            .chars()
            .any(|c| c.is_whitespace() || c == '/' || c == '?' || c == '#')
    {
        return Err(invalid_field("id"));
    }
    Ok(())
}

fn validate_lang(lang: &str) -> Result<(), Error> {
    if lang.len() == 2 && lang.bytes().all(|b| b.is_ascii_alphabetic()) {
        Ok(())
    } else {
        Err(invalid_field("lang"))
    }
}

fn validate_customer_reference(
    customer: &Sep31CustomerReference,
    field: &str,
) -> Result<(), Error> {
    if customer.id.is_empty() {
        return Err(invalid_field(field));
    }
    if customer
        .customer_type
        .as_deref()
        .is_some_and(|t| t.is_empty())
    {
        return Err(invalid_field("customer_type"));
    }
    Ok(())
}

fn is_valid_decimal(value: &str) -> bool {
    let mut dots = 0usize;
    let mut digits = 0usize;
    for byte in value.bytes() {
        match byte {
            b'0'..=b'9' => digits += 1,
            b'.' => dots += 1,
            _ => return false,
        }
    }
    dots <= 1 && digits > 0
}

fn is_valid_asset_code(code: &str) -> bool {
    (4..=12).contains(&code.len())
        && code
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

fn is_valid_memo_type(memo_type: &str) -> bool {
    matches!(memo_type, "text" | "id" | "hash")
}

const STRKEY_VERSION_ACCOUNT_ID: u8 = 6 << 3;
const BASE32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn decode_strkey(s: &str) -> Option<[u8; 35]> {
    if s.len() != 56 || !s.is_ascii() {
        return None;
    }
    let mut out = [0u8; 35];
    let mut buffer: u64 = 0;
    let mut bits: u32 = 0;
    let mut idx = 0usize;
    for &byte in s.as_bytes() {
        let value = BASE32_ALPHABET.iter().position(|&c| c == byte)? as u64;
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out[idx] = ((buffer >> bits) & 0xFF) as u8;
            idx += 1;
        }
    }
    Some(out)
}

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

fn is_valid_stellar_address(s: &str) -> bool {
    match decode_strkey(s) {
        Some(decoded) if decoded[0] == STRKEY_VERSION_ACCOUNT_ID => {
            let expected = u16::from_le_bytes([decoded[33], decoded[34]]);
            crc16_xmodem(&decoded[..33]) == expected
        }
        _ => false,
    }
}

// ── Mock transport ────────────────────────────────────────────────────────────

/// Deterministic [`Sep31Transport`] for tests and examples: returns
/// preconfigured responses and records every request it receives.
#[derive(Default)]
pub struct MockSep31Transport {
    responses: Vec<(Sep31Request, Sep31Response)>,
    calls: Vec<Sep31Request>,
    should_fail: bool,
}

impl MockSep31Transport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_response(&mut self, request: Sep31Request, response: Sep31Response) {
        self.responses.push((request, response));
    }

    pub fn with_response(mut self, request: Sep31Request, response: Sep31Response) -> Self {
        self.add_response(request, response);
        self
    }

    pub fn set_should_fail(&mut self, should_fail: bool) {
        self.should_fail = should_fail;
    }

    pub fn calls(&self) -> &[Sep31Request] {
        &self.calls
    }

    pub fn call_count(&self) -> usize {
        self.calls.len()
    }

    pub fn take_calls(&mut self) -> Vec<Sep31Request> {
        core::mem::take(&mut self.calls)
    }

    pub fn reset(&mut self) {
        self.responses.clear();
        self.calls.clear();
        self.should_fail = false;
    }
}

impl Sep31Transport for MockSep31Transport {
    fn send(&mut self, request: Sep31Request) -> Result<Sep31Response, Error> {
        if self.should_fail {
            return Err(Error::from_code(ErrorCode::TransportTimeout));
        }
        self.calls.push(request.clone());
        for (expected, response) in &self.responses {
            if *expected == request {
                return Ok(response.clone());
            }
        }
        Err(Error::with_context(
            ErrorCode::ValidationError,
            "no mock SEP-31 response configured for this request",
            "request",
        ))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    const ACCOUNT: &str = "GAAACAQDAQCQMBYIBEFAWDANBYHRAEISCMKBKFQXDAMRUGY4DUPB7JZX";
    const JWT: &str = "header.payload.signature";
    const HOST: &str = "https://sending.example.com/callback";
    const BODY: &str = r#"{"transaction":{"id":"tx-1","status":"pending_sender"}}"#;

    fn auth() -> Sep31Auth {
        Sep31Auth::new(JWT).unwrap()
    }

    #[test]
    fn status_round_trips_all_sep31_values() {
        for status in [
            Sep31Status::PendingSender,
            Sep31Status::PendingStellar,
            Sep31Status::PendingCustomerInfoUpdate,
            Sep31Status::PendingTransactionInfoUpdate,
            Sep31Status::PendingReceiver,
            Sep31Status::PendingExternal,
            Sep31Status::Completed,
            Sep31Status::Refunded,
            Sep31Status::Expired,
            Sep31Status::Error,
        ] {
            assert_eq!(Sep31Status::from_str(status.as_str()), status);
        }
    }

    #[test]
    fn status_preserves_unknown_values_and_exposes_helpers() {
        let status = Sep31Status::from_str("pending_new_status");
        assert_eq!(
            status,
            Sep31Status::Unknown(String::from("pending_new_status"))
        );
        assert_eq!(status.as_str(), "pending_new_status");
        assert!(Sep31Status::Completed.is_terminal());
        assert!(Sep31Status::Refunded.is_terminal());
        assert!(!Sep31Status::PendingSender.is_terminal());
        assert!(Sep31Status::PendingCustomerInfoUpdate.requires_info_update());
        assert!(Sep31Status::PendingTransactionInfoUpdate.requires_info_update());
        assert!(!Sep31Status::PendingReceiver.requires_info_update());
    }

    #[test]
    fn auth_rejects_empty_and_malformed_tokens() {
        assert_eq!(
            Sep31Auth::new("").unwrap_err().code,
            ErrorCode::InvalidSep10Token
        );
        assert_eq!(
            Sep31Auth::new("token with spaces").unwrap_err().code,
            ErrorCode::InvalidSep10Token
        );
    }

    #[test]
    fn auth_strips_bearer_prefix() {
        let auth = Sep31Auth::new("Bearer abc.def.ghi").unwrap();
        assert_eq!(auth.jwt(), "abc.def.ghi");
        assert_eq!(auth.authorization_header(), "Bearer abc.def.ghi");
    }

    #[test]
    fn get_info_returns_typed_assets_and_sep12_types() {
        let asset =
            Sep31AssetInfo::new("USDC", &["SEPA", "SWIFT"]).with_limits(Some("0.1"), Some("1000"));
        let request = Sep31Request::info(JWT, Some("en"));
        let response = Sep31Response::Info(
            RawSep31InfoResponse::new(alloc::vec![asset.clone()])
                .with_customer_types(&["sep31_sender"], &["sep31_receiver"]),
        );
        let transport = MockSep31Transport::new().with_response(request, response);
        let mut client = Sep31Client::new(transport, auth());

        let info = client.get_info(Some("en")).unwrap();
        assert_eq!(info.receive, alloc::vec![asset]);
        assert_eq!(info.sender_customer_types, ["sep31_sender"]);
        assert!(info.requires_customer(true));
        assert!(info.requires_customer(false));
        assert_eq!(
            client.transport().calls()[0].authorization(),
            "Bearer header.payload.signature"
        );
    }

    #[test]
    fn request_builders_carry_method_path_and_bearer_auth() {
        let request = Sep31TransactionRequest::new("100", "USDC", "SWIFT")
            .with_sender_id("sender-customer")
            .with_receiver(Sep31CustomerReference::with_type(
                "receiver-customer",
                "sep31_receiver",
            ));
        let wire = Sep31Request::create(&request, JWT);
        assert_eq!(wire.method(), "POST");
        assert_eq!(wire.path(), "/transactions");
        assert_eq!(wire.authorization(), "Bearer header.payload.signature");
        match &wire {
            Sep31Request::CreateTransaction { request, .. } => {
                assert_eq!(request.sender.as_ref().unwrap().id, "sender-customer");
                assert_eq!(
                    request.receiver.as_ref().unwrap().customer_type.as_deref(),
                    Some("sep31_receiver")
                );
            }
            _ => panic!("expected CreateTransaction request"),
        }

        let get = Sep31Request::get("tx-1", JWT);
        assert_eq!(get.method(), "GET");
        assert_eq!(get.path(), "/transactions/tx-1");
        assert_eq!(get.transaction_id(), Some("tx-1"));

        let callback = Sep31Request::register_callback("tx-1", HOST, JWT);
        assert_eq!(callback.method(), "PUT");
        assert_eq!(callback.path(), "/transactions/tx-1/callback");
    }

    #[test]
    fn create_transaction_sends_customer_references_and_returns_memo() {
        let request = Sep31TransactionRequest::new("100", "USDC", "SWIFT")
            .with_asset_issuer(ACCOUNT)
            .with_destination_asset("iso4217:BRL")
            .with_sender_id("sender-id")
            .with_receiver_id("receiver-id");
        let created =
            RawSep31CreationResponse::new("tx-1").with_stellar(ACCOUNT, "123456789", "id");
        let transport = MockSep31Transport::new().with_response(
            Sep31Request::create(&request, JWT),
            Sep31Response::Created(created),
        );
        let mut client = Sep31Client::new(transport, auth());

        let result = client.create_transaction(&request).unwrap();
        assert_eq!(result.id, "tx-1");
        assert!(result.is_ready_to_pay());
        assert_eq!(result.stellar_memo.as_deref(), Some("123456789"));
        assert_eq!(client.transport().call_count(), 1);
    }

    #[test]
    fn create_transaction_rejects_invalid_requests_before_transport() {
        let request = Sep31TransactionRequest::new("100", "usdc", "SWIFT");
        let transport = MockSep31Transport::new();
        let mut client = Sep31Client::new(transport, auth());

        assert_eq!(
            client.create_transaction(&request).unwrap_err().code,
            ErrorCode::ValidationError
        );
        assert_eq!(client.transport().call_count(), 0);

        let mismatched_memo =
            Sep31TransactionRequest::new("100", "USDC", "SWIFT").with_refund_memo("memo", "");
        assert_eq!(
            client
                .create_transaction(&mismatched_memo)
                .unwrap_err()
                .code,
            ErrorCode::ValidationError
        );
    }

    #[test]
    fn get_transaction_normalizes_status_and_amounts() {
        let raw = RawSep31Transaction::new("tx-1", Sep31Status::PendingExternal)
            .with_amounts("100", "90", "10")
            .with_message("Payment initiated");
        let transport = MockSep31Transport::new().with_response(
            Sep31Request::get("tx-1", JWT),
            Sep31Response::Transaction(Box::new(raw)),
        );
        let mut client = Sep31Client::new(transport, auth());

        let transaction = client.get_transaction("tx-1").unwrap();
        assert_eq!(transaction.status, Sep31Status::PendingExternal);
        assert_eq!(transaction.amount_in.as_deref(), Some("100"));
        assert_eq!(
            transaction.status_message.as_deref(),
            Some("Payment initiated")
        );
    }

    #[test]
    fn get_transaction_rejects_path_traversal_and_404() {
        let mut client = Sep31Client::new(MockSep31Transport::new(), auth());
        assert_eq!(
            client.get_transaction("../tx").unwrap_err().code,
            ErrorCode::ValidationError
        );
        assert_eq!(client.transport().call_count(), 0);

        let transport = MockSep31Transport::new().with_response(
            Sep31Request::get("missing", JWT),
            Sep31Response::Transaction(Box::new(
                RawSep31Transaction::new("missing", Sep31Status::Error).with_status_code(404),
            )),
        );
        let mut client = Sep31Client::new(transport, auth());
        assert_eq!(
            client.get_transaction("missing").unwrap_err().code,
            ErrorCode::TransactionNotFound
        );
    }

    #[test]
    fn patch_transaction_sends_requested_fields() {
        let fields: [(&str, &str); 2] = [
            ("receiver_bank_account", "123456789"),
            ("receiver_routing_number", "021000021"),
        ];
        let request = Sep31Request::patch("tx-1", &fields, JWT);
        let transport = MockSep31Transport::new().with_response(
            request.clone(),
            Sep31Response::transaction("tx-1", Sep31Status::PendingReceiver),
        );
        let mut client = Sep31Client::new(transport, auth());

        let transaction = client.patch_transaction("tx-1", &fields).unwrap();
        assert_eq!(transaction.status, Sep31Status::PendingReceiver);
        match &client.transport().calls()[0] {
            Sep31Request::PatchTransaction { id, fields, .. } => {
                assert_eq!(id, "tx-1");
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].1, "123456789");
            }
            _ => panic!("expected PatchTransaction request"),
        }
    }

    #[test]
    fn patch_transaction_requires_named_fields() {
        let mut client = Sep31Client::new(MockSep31Transport::new(), auth());
        assert_eq!(
            client.patch_transaction("tx-1", &[]).unwrap_err().code,
            ErrorCode::ValidationError
        );
        assert_eq!(
            client
                .patch_transaction("tx-1", &[("", "value")])
                .unwrap_err()
                .code,
            ErrorCode::ValidationError
        );
        assert_eq!(client.transport().call_count(), 0);
    }

    #[test]
    fn register_callback_uses_https_and_succeeds_on_204() {
        let transport = MockSep31Transport::new().with_response(
            Sep31Request::register_callback("tx-1", HOST, JWT),
            Sep31Response::callback_registered(),
        );
        let mut client = Sep31Client::new(transport, auth());
        assert!(client.register_callback("tx-1", HOST).is_ok());
    }

    #[test]
    fn register_callback_rejects_insecure_urls() {
        let mut client = Sep31Client::new(MockSep31Transport::new(), auth());
        assert!(client
            .register_callback("tx-1", "http://sending.example.com/callback")
            .is_err());
        assert_eq!(client.transport().call_count(), 0);
    }

    #[test]
    fn http_error_statuses_map_to_anchor_kit_errors() {
        for (code, expected) in [
            (400, ErrorCode::ValidationError),
            (401, ErrorCode::InvalidSep10Token),
            (403, ErrorCode::InvalidSep10Token),
            (404, ErrorCode::TransactionNotFound),
            (408, ErrorCode::TransportTimeout),
            (429, ErrorCode::RateLimitExceeded),
            (500, ErrorCode::ServerError),
        ] {
            assert_eq!(http_error(code).code, expected, "status {code}");
        }
    }

    #[test]
    fn unauthorized_info_response_maps_to_invalid_sep10_token() {
        let transport = MockSep31Transport::new().with_response(
            Sep31Request::info(JWT, None),
            Sep31Response::Info(RawSep31InfoResponse::new(alloc::vec![]).with_status_code(403)),
        );
        let mut client = Sep31Client::new(transport, auth());
        assert_eq!(
            client.get_info(None).unwrap_err().code,
            ErrorCode::InvalidSep10Token
        );
    }

    #[test]
    fn transport_failures_propagate() {
        let mut transport = MockSep31Transport::new();
        transport.set_should_fail(true);
        let mut client = Sep31Client::new(transport, auth());
        assert_eq!(
            client.get_transaction("tx-1").unwrap_err().code,
            ErrorCode::TransportTimeout
        );
    }

    #[test]
    fn callback_header_prefers_signature_over_legacy() {
        assert_eq!(
            callback_signature_header(Some("t=1, s=abc"), Some("t=2, s=def")).unwrap(),
            "t=1, s=abc"
        );
        assert_eq!(
            callback_signature_header(None, Some("t=2, s=def")).unwrap(),
            "t=2, s=def"
        );
        assert_eq!(
            callback_signature_header(None, None).unwrap_err().code,
            ErrorCode::InvalidSep10Token
        );
    }

    #[test]
    fn callback_parse_enforces_freshness() {
        let body = RawSep31Transaction::new("tx-1", Sep31Status::PendingSender);
        let header = "t=1000, s=c2lnbmF0dXJl";
        assert!(Sep31Callback::parse(header, HOST, body.clone(), 1030, 60).is_ok());
        assert_eq!(
            Sep31Callback::parse(header, HOST, body.clone(), 1200, 60)
                .unwrap_err()
                .code,
            ErrorCode::ReplayAttack
        );
        assert_eq!(
            Sep31Callback::parse(header, HOST, body, 900, 60)
                .unwrap_err()
                .code,
            ErrorCode::InvalidTimestamp
        );
    }

    #[test]
    fn callback_parse_rejects_malformed_headers() {
        let body = RawSep31Transaction::new("tx-1", Sep31Status::Completed);
        for header in ["", "t=1000", "s=abc", "t=x, s=abc", "t=1000, s="] {
            assert!(Sep31Callback::parse(header, HOST, body.clone(), 1000, 60).is_err());
        }
    }

    #[test]
    fn callback_signature_verifies_and_rejects_tampering() {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let timestamp = 1000u64;
        let payload = format!("{timestamp}.{HOST}.{BODY}");
        let signature = signing_key.sign(payload.as_bytes());
        let header = format!(
            "t={timestamp}, s={}",
            BASE64_STANDARD.encode(signature.to_bytes())
        );
        let body = RawSep31Transaction::new("tx-1", Sep31Status::PendingSender);
        let callback = Sep31Callback::parse(&header, HOST, body, timestamp + 10, 60).unwrap();

        assert!(callback
            .verify(BODY, signing_key.verifying_key().as_bytes())
            .is_ok());
        assert_eq!(
            callback
                .verify(
                    "{}\"tampered\":true}",
                    signing_key.verifying_key().as_bytes()
                )
                .unwrap_err()
                .code,
            ErrorCode::InvalidSep10Token
        );
    }

    #[test]
    fn mock_transport_records_and_resets_calls() {
        let transport = MockSep31Transport::new().with_response(
            Sep31Request::get("tx-1", JWT),
            Sep31Response::transaction("tx-1", Sep31Status::PendingSender),
        );
        let mut client = Sep31Client::new(transport, auth());
        assert!(client.get_transaction("tx-1").is_ok());
        assert_eq!(client.transport().call_count(), 1);
        assert_eq!(client.transport_mut().take_calls().len(), 1);
        assert_eq!(client.transport().call_count(), 0);
        client.transport_mut().reset();
    }
}
