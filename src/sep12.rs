//! SEP-12 KYC customer API client.
//!
//! SEP-6 deposits and withdrawals frequently pause with `needs_info` or
//! `pending_customer_info_update`. Those statuses mean the anchor needs KYC
//! data that is collected through SEP-12. This module provides a small,
//! transport-agnostic client for the three SEP-12 customer endpoints:
//!
//! | Endpoint                     | Client method                     |
//! |------------------------------|-----------------------------------|
//! | `GET /customer`              | [`Sep12Client::get_customer`]     |
//! | `PUT /customer`              | [`Sep12Client::put_customer`]     |
//! | `DELETE /customer/{account}` | [`Sep12Client::delete_customer`]  |
//!
//! # Authentication
//!
//! Every request is authenticated with a SEP-10 JWT. Callers obtain the token
//! through the SEP-10 flow and wrap it in [`Sep10Auth`]; the client attaches it
//! as an `Authorization: Bearer <jwt>` header on **every** request. A missing or
//! malformed token is rejected locally before any transport call is made, and
//! a `401`/`403` response is surfaced as
//! [`ErrorCode::InvalidSep10Token`](crate::ErrorCode::InvalidSep10Token).
//!
//! # Transport
//!
//! This module makes no HTTP calls itself. The caller supplies a
//! [`Sep12Transport`] implementation, which keeps the client usable from
//! both `std` HTTP backends and the Soroban host, and makes the client easy to
//! exercise with [`MockSep12Transport`] in tests.

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::errors::{Error, ErrorCode};

// ── Status ────────────────────────────────────────────────────────────────────

/// SEP-12 customer status values.
///
/// The four known variants correspond to the status strings defined by SEP-12.
/// [`Unknown`](Self::Unknown) preserves a status returned by an anchor that has
/// added a new value, so parsing never discards information.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CustomerStatus {
    /// KYC information is complete and approved.
    Accepted,
    /// The anchor is reviewing the submitted information.
    Processing,
    /// The customer must submit additional information before processing can continue.
    NeedsInfo,
    /// The submitted information was rejected.
    Rejected,
    /// A status value not yet known to this client.
    Unknown(String),
}

impl CustomerStatus {
    /// Wire-name aliases, mirroring the uppercase SEP-12 status literals.
    pub const ACCEPTED: Self = Self::Accepted;
    pub const PROCESSING: Self = Self::Processing;
    pub const NEEDS_INFO: Self = Self::NeedsInfo;
    pub const REJECTED: Self = Self::Rejected;

    /// Parse a status string returned by an anchor.
    ///
    /// Matching is case-insensitive because anchors have historically returned
    /// both `"NEEDS_INFO"` and `"needs_info"`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_uppercase().as_str() {
            "ACCEPTED" => Self::Accepted,
            "PROCESSING" => Self::Processing,
            "NEEDS_INFO" => Self::NeedsInfo,
            "REJECTED" => Self::Rejected,
            _ => Self::Unknown(String::from(s)),
        }
    }

    /// Return the canonical SEP-12 wire representation of this status.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Accepted => "ACCEPTED",
            Self::Processing => "PROCESSING",
            Self::NeedsInfo => "NEEDS_INFO",
            Self::Rejected => "REJECTED",
            Self::Unknown(s) => s.as_str(),
        }
    }

    /// `true` when the customer must supply more information to the anchor.
    pub fn requires_info(&self) -> bool {
        matches!(self, Self::NeedsInfo)
    }
}

// ── Customer data ────────────────────────────────────────────────────────────

/// A KYC field as described by the anchor.
///
/// Used both for the fields an anchor asks for (`fields`) and for the fields a
/// customer has already submitted (`provided_fields`, which also carry a
/// per-field `status` and `error`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomerField {
    pub name: String,
    pub description: String,
    pub field_type: String,
    pub required: bool,
    pub status: Option<CustomerStatus>,
    pub error: Option<String>,
}

impl CustomerField {
    /// Construct a field-request entry (no per-field status yet).
    pub fn new(name: &str, description: &str, field_type: &str, required: bool) -> Self {
        Self {
            name: String::from(name),
            description: String::from(description),
            field_type: String::from(field_type),
            required,
            status: None,
            error: None,
        }
    }

    /// Attach the anchor's evaluation of a submitted field.
    pub fn with_status(mut self, status: CustomerStatus, error: Option<&str>) -> Self {
        self.status = Some(status);
        self.error = error.map(String::from);
        self
    }
}

/// Normalized customer record returned by `GET /customer` and `PUT /customer`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomerResponse {
    pub id: String,
    pub account: String,
    pub customer_type: Option<String>,
    pub status: CustomerStatus,
    pub fields: Vec<CustomerField>,
    pub provided_fields: Vec<CustomerField>,
    pub message: Option<String>,
}

// ── Transport-level messages ─────────────────────────────────────────────────

/// A SEP-12 request handed to a [`Sep12Transport`].
///
/// The bearer token is always present: [`Sep12Client`] builds these requests
/// from a validated [`Sep10Auth`], so a transport can rely on `authorization`
/// being a well-formed `Bearer` header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Sep12Request {
    GetCustomer {
        account: String,
        customer_type: Option<String>,
        authorization: String,
    },
    PutCustomer {
        account: String,
        customer_type: Option<String>,
        fields: Vec<(String, String)>,
        authorization: String,
    },
    DeleteCustomer {
        account: String,
        customer_type: Option<String>,
        authorization: String,
    },
}

impl Sep12Request {
    /// Build a `GET /customer` request with the given bearer token value.
    pub fn get(account: &str, customer_type: Option<&str>, jwt: &str) -> Self {
        Self::GetCustomer {
            account: String::from(account),
            customer_type: customer_type.map(String::from),
            authorization: bearer_header(jwt),
        }
    }

    /// Build a `PUT /customer` request with the given bearer token value.
    pub fn put(
        account: &str,
        customer_type: Option<&str>,
        fields: &[(&str, &str)],
        jwt: &str,
    ) -> Self {
        Self::PutCustomer {
            account: String::from(account),
            customer_type: customer_type.map(String::from),
            fields: fields
                .iter()
                .map(|(name, value)| (String::from(*name), String::from(*value)))
                .collect(),
            authorization: bearer_header(jwt),
        }
    }

    /// Build a `DELETE /customer/{account}` request with the given bearer token value.
    pub fn delete(account: &str, customer_type: Option<&str>, jwt: &str) -> Self {
        Self::DeleteCustomer {
            account: String::from(account),
            customer_type: customer_type.map(String::from),
            authorization: bearer_header(jwt),
        }
    }

    /// The `Authorization` header value carried by this request.
    pub fn authorization(&self) -> &str {
        match self {
            Self::GetCustomer { authorization, .. }
            | Self::PutCustomer { authorization, .. }
            | Self::DeleteCustomer { authorization, .. } => authorization,
        }
    }
}

/// Raw customer body returned by an anchor, including the HTTP status code.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawCustomerResponse {
    pub status_code: u16,
    pub id: String,
    pub account: String,
    pub customer_type: Option<String>,
    pub status: String,
    pub fields: Vec<CustomerField>,
    pub provided_fields: Vec<CustomerField>,
    pub message: Option<String>,
}

impl RawCustomerResponse {
    /// Build a representative 200 OK customer response.
    pub fn new(account: &str, status: CustomerStatus) -> Self {
        Self {
            status_code: 200,
            id: String::from("customer-1"),
            account: String::from(account),
            customer_type: None,
            status: String::from(status.as_str()),
            fields: Vec::new(),
            provided_fields: Vec::new(),
            message: None,
        }
    }

    /// Override the HTTP status code (for error-response tests).
    pub fn with_status_code(mut self, status_code: u16) -> Self {
        self.status_code = status_code;
        self
    }
}

/// A SEP-12 response returned by a [`Sep12Transport`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Sep12Response {
    /// A customer record (HTTP 200/201 or an error status with a body).
    Customer(RawCustomerResponse),
    /// `DELETE /customer/{account}` completed (HTTP 204).
    Deleted { status_code: u16 },
}

impl Sep12Response {
    /// Build a successful customer response.
    pub fn customer(account: &str, status: CustomerStatus) -> Self {
        Self::Customer(RawCustomerResponse::new(account, status))
    }

    /// Build a successful 204 No Content response.
    pub fn deleted() -> Self {
        Self::Deleted { status_code: 204 }
    }
}

/// Abstraction over the HTTP exchange for SEP-12 endpoints.
pub trait Sep12Transport {
    fn send(&mut self, request: Sep12Request) -> Result<Sep12Response, Error>;
}

// ── Authentication ───────────────────────────────────────────────────────────

/// A SEP-10 JWT used to authenticate SEP-12 requests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep10Auth {
    jwt: String,
}

impl Sep10Auth {
    /// Wrap a SEP-10 JWT.
    ///
    /// Accepts either a bare token or a value already prefixed with `Bearer `.
    /// Rejects empty tokens and tokens containing whitespace, which would
    /// produce a malformed `Authorization` header.
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

    /// The raw JWT value (without the `Bearer ` prefix).
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

// ── Client ───────────────────────────────────────────────────────────────────

/// Transport-agnostic SEP-12 customer client.
pub struct Sep12Client<T: Sep12Transport> {
    transport: T,
    auth: Sep10Auth,
}

impl<T: Sep12Transport> Sep12Client<T> {
    pub fn new(transport: T, auth: Sep10Auth) -> Self {
        Self { transport, auth }
    }

    /// The SEP-10 authentication attached to every request.
    pub fn auth(&self) -> &Sep10Auth {
        &self.auth
    }

    /// The underlying transport (useful for inspecting recorded calls).
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Mutable access to the underlying transport.
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// `GET /customer` — fetch the customer record for a Stellar account.
    pub fn get_customer(
        &mut self,
        account: &str,
        customer_type: Option<&str>,
    ) -> Result<CustomerResponse, Error> {
        validate_account(account)?;
        let request = Sep12Request::get(account, customer_type, self.auth.jwt());
        customer_from_response(self.transport.send(request)?)
    }

    /// `PUT /customer` — submit or update KYC fields for a Stellar account.
    pub fn put_customer(
        &mut self,
        account: &str,
        customer_type: Option<&str>,
        fields: &[(&str, &str)],
    ) -> Result<CustomerResponse, Error> {
        validate_account(account)?;
        let request = Sep12Request::put(account, customer_type, fields, self.auth.jwt());
        customer_from_response(self.transport.send(request)?)
    }

    /// `DELETE /customer/{account}` — remove the customer record.
    pub fn delete_customer(
        &mut self,
        account: &str,
        customer_type: Option<&str>,
    ) -> Result<(), Error> {
        validate_account(account)?;
        let request = Sep12Request::delete(account, customer_type, self.auth.jwt());
        match self.transport.send(request)? {
            Sep12Response::Deleted { status_code } if (200..=299).contains(&status_code) => Ok(()),
            Sep12Response::Deleted { status_code } => Err(http_error(status_code)),
            Sep12Response::Customer(raw) => Err(http_error(raw.status_code)),
        }
    }
}

/// Unwrap a transport response for a customer-returning endpoint.
fn customer_from_response(response: Sep12Response) -> Result<CustomerResponse, Error> {
    match response {
        Sep12Response::Customer(raw) => normalize_customer(raw),
        Sep12Response::Deleted { status_code } => Err(http_error(status_code)),
    }
}

/// Convert a raw customer response into a typed [`CustomerResponse`], mapping
/// HTTP error codes onto AnchorKit errors in the same way as the SEP-6 client.
fn normalize_customer(raw: RawCustomerResponse) -> Result<CustomerResponse, Error> {
    if !(200..=299).contains(&raw.status_code) {
        return Err(http_error(raw.status_code));
    }
    if raw.account.is_empty() {
        return Err(Error::validation_error(
            "customer response is missing account",
        ));
    }

    Ok(CustomerResponse {
        id: raw.id,
        account: raw.account,
        customer_type: raw.customer_type,
        status: CustomerStatus::from_str(&raw.status),
        fields: raw.fields,
        provided_fields: raw.provided_fields,
        message: raw.message,
    })
}

fn http_error(status_code: u16) -> Error {
    match status_code {
        400 | 422 => Error::validation_error("SEP-12 customer request was rejected"),
        401 | 403 => Error::invalid_sep10_token(),
        404 => Error::with_context(
            ErrorCode::TransactionNotFound,
            "SEP-12 customer not found",
            "customer",
        ),
        408 => Error::transport_timeout(),
        429 => Error::rate_limit_exceeded(),
        500..=599 => Error::from_code(ErrorCode::ServerError),
        _ => Error::with_context(
            ErrorCode::ValidationError,
            "unexpected SEP-12 HTTP status",
            &status_code.to_string(),
        ),
    }
}

// ── Validation ───────────────────────────────────────────────────────────────

const STRKEY_VERSION_ACCOUNT_ID: u8 = 6 << 3;
const BASE32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Validate a Stellar G-address before it is placed in a URL path or query.
fn validate_account(account: &str) -> Result<(), Error> {
    if !is_valid_stellar_address(account) {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "account must be a valid Stellar G-address",
            account,
        ));
    }
    Ok(())
}

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

// ── Mock transport ───────────────────────────────────────────────────────────

/// Deterministic [`Sep12Transport`] for tests: returns preconfigured responses
/// and records every request it receives.
#[derive(Default)]
pub struct MockSep12Transport {
    responses: Vec<(Sep12Request, Sep12Response)>,
    calls: Vec<Sep12Request>,
    should_fail: bool,
}

impl MockSep12Transport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register the response for a specific request (matched by equality).
    pub fn add_response(&mut self, request: Sep12Request, response: Sep12Response) {
        self.responses.push((request, response));
    }

    /// Builder-style variant of [`add_response`](Self::add_response).
    pub fn with_response(mut self, request: Sep12Request, response: Sep12Response) -> Self {
        self.add_response(request, response);
        self
    }

    /// Make every `send` fail with a transport timeout.
    pub fn set_should_fail(&mut self, should_fail: bool) {
        self.should_fail = should_fail;
    }

    /// All requests received so far, in order.
    pub fn calls(&self) -> &[Sep12Request] {
        &self.calls
    }

    pub fn call_count(&self) -> usize {
        self.calls.len()
    }

    /// Remove and return all recorded requests.
    pub fn take_calls(&mut self) -> Vec<Sep12Request> {
        core::mem::take(&mut self.calls)
    }

    pub fn reset(&mut self) {
        self.responses.clear();
        self.calls.clear();
        self.should_fail = false;
    }
}

impl Sep12Transport for MockSep12Transport {
    fn send(&mut self, request: Sep12Request) -> Result<Sep12Response, Error> {
        if self.should_fail {
            return Err(Error::transport_timeout());
        }
        self.calls.push(request.clone());
        for (expected, response) in &self.responses {
            if *expected == request {
                return Ok(response.clone());
            }
        }
        Err(Error::with_context(
            ErrorCode::ValidationError,
            "no mock SEP-12 response configured for this request",
            "request",
        ))
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const ACCOUNT: &str = "GAAACAQDAQCQMBYIBEFAWDANBYHRAEISCMKBKFQXDAMRUGY4DUPB7JZX";
    const JWT: &str = "header.payload.signature";

    fn auth() -> Sep10Auth {
        Sep10Auth::new(JWT).unwrap()
    }

    #[test]
    fn customer_status_round_trips_all_sep12_values() {
        for status in [
            CustomerStatus::Accepted,
            CustomerStatus::Processing,
            CustomerStatus::NeedsInfo,
            CustomerStatus::Rejected,
        ] {
            assert_eq!(CustomerStatus::from_str(status.as_str()), status);
        }
        assert_eq!(
            CustomerStatus::from_str("needs_info"),
            CustomerStatus::NEEDS_INFO
        );
        assert_eq!(CustomerStatus::NEEDS_INFO.as_str(), "NEEDS_INFO");
        assert!(CustomerStatus::NeedsInfo.requires_info());
        assert!(!CustomerStatus::Accepted.requires_info());
    }

    #[test]
    fn customer_status_preserves_unknown_values() {
        let status = CustomerStatus::from_str("SOMETHING_NEW");
        assert_eq!(
            status,
            CustomerStatus::Unknown(String::from("SOMETHING_NEW"))
        );
        assert_eq!(status.as_str(), "SOMETHING_NEW");
    }

    #[test]
    fn sep10_auth_rejects_empty_or_malformed_tokens() {
        assert_eq!(
            Sep10Auth::new("").unwrap_err().code,
            ErrorCode::InvalidSep10Token
        );
        assert_eq!(
            Sep10Auth::new("token with spaces").unwrap_err().code,
            ErrorCode::InvalidSep10Token
        );
    }

    #[test]
    fn sep10_auth_strips_bearer_prefix() {
        let auth = Sep10Auth::new("Bearer abc.def.ghi").unwrap();
        assert_eq!(auth.jwt(), "abc.def.ghi");
        assert_eq!(auth.authorization_header(), "Bearer abc.def.ghi");
    }

    #[test]
    fn get_customer_attaches_jwt_and_normalizes_needs_info() {
        let request = Sep12Request::get(ACCOUNT, Some("sep6_deposit"), JWT);
        let transport = MockSep12Transport::new().with_response(
            request.clone(),
            Sep12Response::customer(ACCOUNT, CustomerStatus::NeedsInfo),
        );
        let mut client = Sep12Client::new(transport, auth());

        let response = client.get_customer(ACCOUNT, Some("sep6_deposit")).unwrap();
        assert_eq!(response.status, CustomerStatus::NeedsInfo);
        assert!(response.status.requires_info());
        assert_eq!(response.account, ACCOUNT);
    }

    #[test]
    fn get_customer_request_carries_bearer_auth() {
        let request = Sep12Request::get(ACCOUNT, None, JWT);
        assert_eq!(request.authorization(), "Bearer header.payload.signature");
        match request {
            Sep12Request::GetCustomer {
                account,
                customer_type,
                ..
            } => {
                assert_eq!(account, ACCOUNT);
                assert_eq!(customer_type, None);
            }
            _ => panic!("expected GetCustomer request"),
        }
    }

    #[test]
    fn put_customer_sends_fields_and_returns_accepted() {
        let fields: [(&str, &str); 2] =
            [("email_address", "user@example.com"), ("first_name", "Ada")];
        let request = Sep12Request::put(ACCOUNT, Some("sep6_deposit"), &fields, JWT);
        let transport = MockSep12Transport::new().with_response(
            request,
            Sep12Response::customer(ACCOUNT, CustomerStatus::Accepted),
        );
        let mut client = Sep12Client::new(transport, auth());

        let response = client
            .put_customer(ACCOUNT, Some("sep6_deposit"), &fields)
            .unwrap();
        assert_eq!(response.status, CustomerStatus::Accepted);
    }

    #[test]
    fn put_customer_request_includes_submitted_fields() {
        let fields: [(&str, &str); 1] = [("email_address", "user@example.com")];
        let request = Sep12Request::put(ACCOUNT, None, &fields, JWT);
        match request {
            Sep12Request::PutCustomer { fields, .. } => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].0, "email_address");
                assert_eq!(fields[0].1, "user@example.com");
            }
            _ => panic!("expected PutCustomer request"),
        }
    }

    #[test]
    fn delete_customer_uses_account_path_and_succeeds_on_204() {
        let request = Sep12Request::delete(ACCOUNT, None, JWT);
        let transport = MockSep12Transport::new().with_response(request, Sep12Response::deleted());
        let mut client = Sep12Client::new(transport, auth());

        assert!(client.delete_customer(ACCOUNT, None).is_ok());
    }

    #[test]
    fn delete_customer_request_targets_the_account() {
        let request = Sep12Request::delete(ACCOUNT, Some("sep6_deposit"), JWT);
        match request {
            Sep12Request::DeleteCustomer {
                account,
                customer_type,
                ..
            } => {
                assert_eq!(account, ACCOUNT);
                assert_eq!(customer_type.as_deref(), Some("sep6_deposit"));
            }
            _ => panic!("expected DeleteCustomer request"),
        }
    }

    #[test]
    fn mock_transport_records_every_request() {
        let transport = MockSep12Transport::new().with_response(
            Sep12Request::get(ACCOUNT, None, JWT),
            Sep12Response::customer(ACCOUNT, CustomerStatus::Processing),
        );
        let mut client = Sep12Client::new(transport, auth());
        assert!(client.get_customer(ACCOUNT, None).is_ok());

        let calls = client.transport().calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].authorization(), "Bearer header.payload.signature");
    }

    #[test]
    fn invalid_account_is_rejected_before_the_transport_is_called() {
        let transport = MockSep12Transport::new();
        let mut client = Sep12Client::new(transport, auth());

        let err = client
            .get_customer("not-a-stellar-address", None)
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
        assert_eq!(client.transport().call_count(), 0);
    }

    #[test]
    fn http_error_statuses_map_to_anchor_kit_errors() {
        for (code, expected) in [
            (401, ErrorCode::InvalidSep10Token),
            (403, ErrorCode::InvalidSep10Token),
            (404, ErrorCode::TransactionNotFound),
            (408, ErrorCode::TransportTimeout),
            (429, ErrorCode::RateLimitExceeded),
            (500, ErrorCode::ServerError),
        ] {
            let err = http_error(code);
            assert_eq!(err.code, expected, "status {code}");
        }
    }

    #[test]
    fn unauthorized_response_maps_to_invalid_sep10_token() {
        let raw = RawCustomerResponse::new(ACCOUNT, CustomerStatus::Accepted).with_status_code(401);
        let transport = MockSep12Transport::new().with_response(
            Sep12Request::get(ACCOUNT, None, JWT),
            Sep12Response::Customer(raw),
        );
        let mut client = Sep12Client::new(transport, auth());

        assert_eq!(
            client.get_customer(ACCOUNT, None).unwrap_err().code,
            ErrorCode::InvalidSep10Token
        );
    }

    #[test]
    fn transport_failures_propagate() {
        let mut transport = MockSep12Transport::new();
        transport.set_should_fail(true);
        let mut client = Sep12Client::new(transport, auth());

        assert_eq!(
            client.get_customer(ACCOUNT, None).unwrap_err().code,
            ErrorCode::TransportTimeout
        );
    }
}
