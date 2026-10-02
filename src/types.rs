use soroban_sdk::{contracttype, Address, Bytes, String, Symbol, Vec};
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
            "error" => Self::Error,
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

    /// Returns `true` if this status represents a terminal (non-retryable) state.
    ///
    /// Terminal statuses are those where no further state transitions are expected:
    /// - `Completed` — transaction settled successfully
    /// - `Refunded`  — funds returned to sender
    /// - `Expired`   — transaction window closed
    /// - `Error`     — unrecoverable anchor-side failure
    ///
    /// All pending and incomplete variants return `false` since they may still
    /// progress to a final state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Refunded | Self::Expired | Self::Error)
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
pub const SERVICE_EXCHANGE_QUOTES: u32 = 5;

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
    ExchangeQuotes,
}

impl ServiceType {
    pub fn as_u32(&self) -> u32 {
        match self {
            ServiceType::Deposits => SERVICE_DEPOSITS,
            ServiceType::Withdrawals => SERVICE_WITHDRAWALS,
            ServiceType::Quotes => SERVICE_QUOTES,
            ServiceType::KYC => SERVICE_KYC,
            ServiceType::ExchangeQuotes => SERVICE_EXCHANGE_QUOTES,
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
    // NOTE: No per-session nonce field. Replay protection for attestations is
    // provided by the payload-hash Used marker in the attestation flow, which
    // is entirely independent of sessions. Adding a nonce here without a
    // corresponding "echo-back and increment" check on every session-scoped
    // call would be decorative and misleading, so the field is intentionally
    // absent.
    pub operation_count: u64,
    pub expires_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
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
    /// Error code captured for failed operations; `None` on success.
    pub error_code: Option<u32>,
    /// Number of retry attempts before success (0 for first attempt success).
    pub attempt_number: u32,
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
    /// Optional Unix timestamp (seconds) after which this attestation is
    /// considered expired. `None` means no expiry.
    pub expires_at: Option<u64>,
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
/// | `"Weighted"`          | Selects anchors proportionally based on health score.     |
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
/// - `max_anchors` — limits how many candidate anchors are considered after
///   other filters are applied. `0` means no limit.
/// - `require_kyc` — when `true`, only anchors that advertise `SERVICE_KYC`
///   are eligible.
/// - `jurisdiction` — when `Some`, only anchors registered in that region
///   (via `set_anchor_jurisdiction`) are eligible. `None` disables geographic
///   filtering.
/// - `fallback_chain` — ordered list of anchor addresses to try in sequence if
///   the primary selection fails. When empty, no fallback is used.
#[contracttype]
#[derive(Clone)]
pub struct RoutingOptions {
    pub request: RoutingRequest,
    pub strategy: Vec<soroban_sdk::Symbol>,
    pub min_reputation: u32,
    pub max_anchors: u32,
    pub require_kyc: bool,
    pub jurisdiction: Option<String>,
    pub fallback_chain: Vec<Address>,
}

/// Returns whether an anchor is eligible under the requested jurisdiction filter.
///
/// When `required` is `None`, all anchors pass. Otherwise the anchor must have
/// a stored jurisdiction that exactly matches `required` (case-sensitive ISO code).
pub fn anchor_matches_jurisdiction(
    required: &Option<String>,
    anchor_jurisdiction: &Option<String>,
) -> bool {
    match required {
        None => true,
        Some(req) => anchor_jurisdiction.as_ref().is_some_and(|j| j == req),
    }
}

#[cfg(test)]
mod transaction_status_tests {
    use super::*;

    #[test]
    fn terminal_statuses_are_detected() {
        assert!(TransactionStatus::Completed.is_terminal());
        assert!(TransactionStatus::Refunded.is_terminal());
        assert!(TransactionStatus::Expired.is_terminal());
        assert!(TransactionStatus::Error.is_terminal());
    }

    #[test]
    fn non_terminal_statuses_are_not_terminal() {
        assert!(!TransactionStatus::Pending.is_terminal());
        assert!(!TransactionStatus::Incomplete.is_terminal());
        assert!(!TransactionStatus::PendingExternal.is_terminal());
        assert!(!TransactionStatus::PendingAnchor.is_terminal());
        assert!(!TransactionStatus::PendingTrust.is_terminal());
        assert!(!TransactionStatus::PendingUser.is_terminal());
        assert!(!TransactionStatus::Unknown(alloc::string::String::from("custom")).is_terminal());
    }

    #[test]
    fn roundtrip_from_str_as_str() {
        let cases = [
            "pending", "incomplete", "pending_external", "pending_anchor",
            "pending_trust", "pending_user", "completed", "refunded", "expired", "error",
        ];
        for s in cases {
            assert_eq!(TransactionStatus::from_str(s).as_str(), s);
        }
    }
}

#[cfg(test)]
mod merged_response_type_tests {
    use super::*;

    #[test]
    fn canonical_response_types_include_merged_fields() {
        let deposit = DepositResponse {
            transaction_id: alloc::string::String::from("txn-123"),
            how: Some(alloc::string::String::from("bank_transfer")),
            extra_info: Some(alloc::string::String::from("follow instructions")),
            deposit_address: Some(alloc::string::String::from("GDEPOSIT...")),
            min_amount: Some(10),
            max_amount: Some(1000),
            fee_fixed: Some(2),
            fee_percent: Some(150),
            expires_at: Some(42),
            status: TransactionStatus::Pending,
            claimable_balance_supported: true,
        };

        assert_eq!(deposit.how, Some(alloc::string::String::from("bank_transfer")));
        assert_eq!(deposit.deposit_address, Some(alloc::string::String::from("GDEPOSIT...")));
        assert_eq!(deposit.expires_at, Some(42));
        assert!(deposit.claimable_balance_supported);

        let withdrawal = WithdrawalResponse {
            transaction_id: alloc::string::String::from("txn-456"),
            account_id: Some(alloc::string::String::from("GDEST...")),
            dest_account_id: Some(alloc::string::String::from("bank-account-99")),
            memo: Some(alloc::string::String::from("memo-1")),
            memo_type: Some(alloc::string::String::from("id")),
            min_amount: Some(5),
            max_amount: Some(500),
            fee_fixed: Some(1),
            fee_percent: Some(50),
            estimated_completion: Some(99),
            status: TransactionStatus::Completed,
        };

        assert_eq!(withdrawal.account_id, Some(alloc::string::String::from("GDEST...")));
        assert_eq!(withdrawal.estimated_completion, Some(99));
    }
}

#[cfg(test)]
mod jurisdiction_filter_tests {
    use super::*;
    use soroban_sdk::String;

    fn s(env: &soroban_sdk::Env, v: &str) -> String {
        String::from_str(env, v)
    }

    #[test]
    fn no_filter_accepts_any_anchor_jurisdiction() {
        let env = soroban_sdk::Env::default();
        assert!(anchor_matches_jurisdiction(&None, &None));
        assert!(anchor_matches_jurisdiction(
            &None,
            &Some(s(&env, "USA")),
        ));
    }

    #[test]
    fn filter_requires_exact_match() {
        let env = soroban_sdk::Env::default();
        let required = Some(s(&env, "USA"));
        assert!(anchor_matches_jurisdiction(
            &required,
            &Some(s(&env, "USA")),
        ));
        assert!(!anchor_matches_jurisdiction(
            &required,
            &Some(s(&env, "GBR")),
        ));
    }

    #[test]
    fn filter_excludes_anchor_without_jurisdiction() {
        let env = soroban_sdk::Env::default();
        let required = Some(s(&env, "USA"));
        assert!(!anchor_matches_jurisdiction(&required, &None));
    }

    #[test]
    fn filter_is_case_sensitive() {
        let env = soroban_sdk::Env::default();
        let required = Some(s(&env, "USA"));
        assert!(!anchor_matches_jurisdiction(
            &required,
            &Some(s(&env, "usa")),
        ));
    }
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
    /// Optional public homepage URL for the anchor (e.g. "https://anchor.example.com").
    pub homepage_url: Option<String>,
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
    /// Whether the anchor supports claimable balances as a deposit destination.
    /// Sourced from the `CLAIMABLE_BALANCE_SUPPORTED` flag in stellar.toml.
    pub claimable_balance_supported: bool,
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
#[derive(Clone, Debug, PartialEq)]
pub struct HealthStatus {
    pub anchor: Address,
    pub latency_ms: u64,
    pub failure_count: u32,
    pub availability_percent: u32,
}

// ---------------------------------------------------------------------------
// Credential management types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CredentialPolicy {
    pub attestor: Address,
    pub rotation_interval_seconds: u64,
    pub require_encryption: bool,
    pub last_rotated: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredCredential {
    pub attestor: Address,
    pub credential_type: Symbol,
    pub encrypted_value: String,
    pub expires_at: u64,
    pub updated_at: u64,
}
