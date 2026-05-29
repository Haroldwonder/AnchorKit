use soroban_sdk::{contracttype, Address, Bytes, String, Vec};
extern crate alloc;
use alloc::string::String as AllocString;

// ---------------------------------------------------------------------------
// SEP-6 Response types (canonical, merged from sep6.rs and response_validator.rs)
// ---------------------------------------------------------------------------

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
    Unknown(AllocString),
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
            _ => Self::Unknown(AllocString::from(s)),
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

/// Canonical merged DepositResponse combining fields from sep6 normalization and response validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DepositResponse {
    /// Unique transaction ID assigned by the anchor.
    pub transaction_id: AllocString,
    /// How the user should send funds (from sep6 normalization).
    pub how: Option<AllocString>,
    /// Optional extra instructions from the anchor (from sep6 normalization).
    pub extra_info: Option<AllocString>,
    /// Deposit address provided by the anchor (from response validation).
    pub deposit_address: Option<AllocString>,
    /// Minimum deposit amount (in asset units), if provided.
    pub min_amount: Option<u64>,
    /// Maximum deposit amount (in asset units), if provided.
    pub max_amount: Option<u64>,
    /// Fee charged for the deposit, if provided.
    pub fee_fixed: Option<u64>,
    /// Percentage fee charged for the deposit in basis points.
    pub fee_percent: Option<u32>,
    /// Expiration time of the deposit address (from response validation).
    pub expires_at: Option<u64>,
    /// Current status of the transaction.
    pub status: TransactionStatus,
}

/// Canonical merged WithdrawalResponse combining fields from sep6 normalization and response validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawalResponse {
    /// Unique transaction ID assigned by the anchor.
    pub transaction_id: AllocString,
    /// Stellar account the user should send funds to.
    pub account_id: Option<AllocString>,
    /// Destination bank/wallet account for the off-chain withdrawal, if provided.
    pub dest_account_id: Option<AllocString>,
    /// Optional memo to attach to the Stellar payment.
    pub memo: Option<AllocString>,
    /// Optional memo type (`text`, `id`, `hash`).
    pub memo_type: Option<AllocString>,
    /// Minimum withdrawal amount (in asset units), if provided.
    pub min_amount: Option<u64>,
    /// Maximum withdrawal amount (in asset units), if provided.
    pub max_amount: Option<u64>,
    /// Fee charged for the withdrawal, if provided.
    pub fee_fixed: Option<u64>,
    /// Percentage fee charged for the withdrawal in basis points.
    pub fee_percent: Option<u32>,
    /// Estimated completion time (from response validation).
    pub estimated_completion: Option<u64>,
    /// Current status of the transaction.
    pub status: TransactionStatus,
}

// ---------------------------------------------------------------------------
// Service constants
// ---------------------------------------------------------------------------

pub const SERVICE_DEPOSITS: u32 = 1;
pub const SERVICE_WITHDRAWALS: u32 = 2;
pub const SERVICE_QUOTES: u32 = 3;
pub const SERVICE_KYC: u32 = 4;

/// Typed representation of a service capability an anchor can support.
///
/// Each variant maps to a stable `u32` discriminant stored on-chain.
/// Use [`ServiceType::as_u32`] to convert before passing to contract functions.
#[derive(Clone, PartialEq)]
pub enum ServiceType {
    Deposits,
    Withdrawals,
    Quotes,
    KYC,
}

impl ServiceType {
    pub fn as_u32(&self) -> u32 {
        match self {
            ServiceType::Deposits => SERVICE_DEPOSITS,
            ServiceType::Withdrawals => SERVICE_WITHDRAWALS,
            ServiceType::Quotes => SERVICE_QUOTES,
            ServiceType::KYC => SERVICE_KYC,
        }
    }
}

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
pub struct Session {
    pub session_id: u64,
    pub initiator: Address,
    pub created_at: u64,
    pub nonce: u64,
    pub operation_count: u64,
    pub expires_at: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct Quote {
    pub quote_id: u64,
    pub anchor: Address,
    pub base_asset: String,
    pub quote_asset: String,
    pub rate: u64,
    pub fee_percentage: u32,
    pub minimum_amount: u64,
    pub maximum_amount: u64,
    pub valid_until: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct OperationContext {
    pub session_id: u64,
    pub operation_index: u64,
    pub operation_type: String,
    pub timestamp: u64,
    pub status: String,
    /// Human-readable outcome, e.g. `"attestation_id=42"`.
    pub result_summary: String,
}

#[contracttype]
#[derive(Clone)]
pub struct AuditLog {
    pub log_id: u64,
    pub session_id: u64,
    pub actor: Address,
    pub operation: OperationContext,
}

#[contracttype]
#[derive(Clone)]
pub struct RequestId {
    pub id: Bytes,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct Attestation {
    pub id: u64,
    pub issuer: Address,
    pub subject: Address,
    pub timestamp: u64,
    pub payload_hash: Bytes,
    pub signature: Bytes,
    /// Set to `true` when the issuer attestor has been revoked after this
    /// attestation was submitted. Historical attestations are preserved for
    /// audit purposes; callers should treat `issuer_revoked = true` as a
    /// signal that the issuer's authority has been withdrawn.
    pub issuer_revoked: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct TracingSpan {
    pub request_id: RequestId,
    pub operation: String,
    pub actor: Address,
    pub started_at: u64,
    pub completed_at: u64,
    pub status: String,
}

#[contracttype]
#[derive(Clone)]
pub struct AnchorServices {
    pub anchor: Address,
    pub services: Vec<u32>,
}

// ---------------------------------------------------------------------------
// Routing types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
pub struct RoutingRequest {
    pub base_asset: String,
    pub quote_asset: String,
    pub amount: u64,
    pub operation_type: u32,
}

/// Options passed to `route_transaction` to control anchor selection.
///
/// # Strategy
///
/// The `strategy` field is a single-element `Vec<Symbol>` that selects how the
/// best anchor is chosen from all valid candidates. Valid strategy symbols:
///
/// | Symbol                | Behaviour                                                  |
/// |-----------------------|------------------------------------------------------------|
/// | `"LowestFee"`         | Selects the anchor with the lowest `fee_percentage`.       |
/// | `"FastestSettlement"` | Selects the anchor with the lowest `average_settlement_time`. |
/// | `"HighestReputation"` | Selects the anchor with the highest `reputation_score`.    |
/// | `"Balanced"`          | Composite scoring: (40_000/fee) + (30_000/time) + (reputation*3000/10000). |
///
/// **Validation:** `strategy` is required and must contain exactly one symbol.
/// - Passing an empty `Vec` causes the call to panic with `NoQuotesAvailable`.
/// - An unrecognised symbol causes the call to panic with `InvalidStrategy`.
///
/// # Other fields
///
/// - `min_reputation` — anchors with a `reputation_score` strictly below this
///   value are excluded before strategy selection. When `min_reputation = 0`,
///   reputation filtering is disabled and all anchors are included regardless
///   of their reputation score.
/// - `max_anchors` / `require_kyc` — reserved for future filtering; not yet
///   enforced by the current implementation.
#[contracttype]
#[derive(Clone)]
pub struct RoutingOptions {
    pub request: RoutingRequest,
    pub strategy: Vec<soroban_sdk::Symbol>,
    pub min_reputation: u32,
    pub max_anchors: u32,
    pub require_kyc: bool,
}

// ---------------------------------------------------------------------------
// Metadata cache types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
pub struct AnchorMetadata {
    pub anchor: Address,
    pub reputation_score: u32,
    pub liquidity_score: u32,
    pub uptime_percentage: u32,
    pub total_volume: u64,
    pub average_settlement_time: u64,
    pub is_active: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct MetadataCache {
    pub metadata: AnchorMetadata,
    pub cached_at: u64,
    pub ttl_seconds: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct CapabilitiesCache {
    pub toml_url: String,
    pub capabilities: Vec<u32>,
    pub cached_at: u64,
    pub ttl_seconds: u64,
}

// ---------------------------------------------------------------------------
// Anchor Info Discovery types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
pub struct AssetInfo {
    pub code: String,
    pub issuer: String,
    pub deposit_enabled: bool,
    pub withdrawal_enabled: bool,
    pub deposit_fee_fixed: u64,
    pub deposit_fee_percent: u32,
    pub withdrawal_fee_fixed: u64,
    pub withdrawal_fee_percent: u32,
    pub deposit_min_amount: u64,
    pub deposit_max_amount: u64,
    pub withdrawal_min_amount: u64,
    pub withdrawal_max_amount: u64,
    /// Number of decimal places for the asset (e.g. 7 for USDC on Stellar).
    /// Parsed from the `significant_decimals` field of stellar.toml; defaults to 7.
    pub decimals: u32,
}

/// Represents a fiat currency supported by an anchor (e.g. USD, EUR).
#[contracttype]
#[derive(Clone)]
pub struct FiatCurrency {
    pub code: String,
    pub name: String,
    pub deposit_enabled: bool,
    pub withdrawal_enabled: bool,
    /// ISO 3166-1 alpha-3 country code (e.g. "USA", "GBR"). From stellar.toml / SEP-6.
    pub country_code: Option<String>,
    /// Human-readable description of the currency.
    pub desc: Option<String>,
    /// Preferred number of decimal places to display (0–7).
    pub display_decimals: Option<u32>,
}

#[contracttype]
#[derive(Clone)]
pub struct StellarToml {
    pub version: String,
    pub network_passphrase: String,
    pub accounts: Vec<String>,
    /// The SIGNING_KEY from stellar.toml, used for SEP-10 verification.
    /// `None` when the anchor does not publish a signing key.
    pub signing_key: Option<String>,
    pub currencies: Vec<AssetInfo>,
    /// Fiat currencies supported by this anchor (USD, EUR, etc.).
    pub fiat_currencies: Vec<FiatCurrency>,
    pub transfer_server: String,
    pub transfer_server_sep0024: String,
    pub kyc_server: String,
    pub web_auth_endpoint: String,
}

#[contracttype]
#[derive(Clone)]
pub struct CachedToml {
    pub toml: StellarToml,
    pub cached_at: u64,
    pub ttl_seconds: u64,
}

// ---------------------------------------------------------------------------
// Health monitoring types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
pub struct HealthStatus {
    pub anchor: Address,
    pub latency_ms: u64,
    pub failure_count: u32,
    pub availability_percent: u32,
}
