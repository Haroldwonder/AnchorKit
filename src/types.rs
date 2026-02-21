use soroban_sdk::{contracttype, Address, Bytes, BytesN, Env, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attestation {
    pub id: u64,
    pub issuer: Address,
    pub subject: Address,
    pub timestamp: u64,
    pub payload_hash: BytesN<32>,
    pub signature: Bytes,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Endpoint {
    pub url: String,
    pub attestor: Address,
    pub is_active: bool,
}

/// Supported service types for anchors
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ServiceType {
    Deposits = 1,
    Withdrawals = 2,
    Quotes = 3,
    KYC = 4,
}

/// Configuration of supported services for an anchor
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorServices {
    pub anchor: Address,
    pub services: Vec<ServiceType>,
}

/// Quote data structure for rate comparison
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuoteData {
    pub anchor: Address,
    pub base_asset: String,
    pub quote_asset: String,
    pub rate: u64,           // 10000 = 1.0
    pub fee_percentage: u32, // Fee in basis points
    pub minimum_amount: u64,
    pub maximum_amount: u64,
    pub valid_until: u64,
    pub quote_id: u64,
}

/// Rate comparison result
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateComparison {
    pub best_quote: QuoteData,
    pub all_quotes: Vec<QuoteData>,
    pub comparison_timestamp: u64,
}

/// Quote request parameters
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuoteRequest {
    pub base_asset: String,
    pub quote_asset: String,
    pub amount: u64,
    pub operation_type: ServiceType,
}

/// High-level input that drives secure, compliant transaction intent construction.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionIntentBuilder {
    pub anchor: Address,
    pub request: QuoteRequest,
    pub quote_id: u64,
    pub require_kyc: bool,
    pub session_id: u64,
    pub ttl_seconds: u64,
}

impl TransactionIntentBuilder {
    /// Creates a builder with safe defaults:
    /// - No quote (`quote_id = 0`)
    /// - No session (`session_id = 0`)
    /// - KYC not required
    /// - 5 minute TTL
    pub fn new(_env: &Env, anchor: Address, request: QuoteRequest) -> Self {
        Self {
            anchor,
            request,
            quote_id: 0,
            require_kyc: false,
            session_id: 0,
            ttl_seconds: 300,
        }
    }

    pub fn with_quote_id(mut self, quote_id: u64) -> Self {
        self.quote_id = quote_id;
        self
    }

    pub fn require_kyc(mut self) -> Self {
        self.require_kyc = true;
        self
    }

    pub fn with_session(mut self, session_id: u64) -> Self {
        self.session_id = session_id;
        self
    }

    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl_seconds = ttl_seconds;
        self
    }
}

/// Fully validated transaction intent produced by the high-level builder.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionIntent {
    pub intent_id: u64,
    pub anchor: Address,
    pub request: QuoteRequest,
    pub quote_id: u64,
    pub has_quote: bool,
    pub rate: u64,
    pub fee_percentage: u32,
    pub requires_kyc: bool,
    pub session_id: u64,
    pub created_at: u64,
    pub expires_at: u64,
}

/// Represents a reproducible interaction session.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractionSession {
    pub session_id: u64,
    pub initiator: Address,
    pub created_at: u64,
    pub operation_count: u64,
    pub nonce: u64,
}

/// Context for each operation within a session.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationContext {
    pub session_id: u64,
    pub operation_index: u64,
    pub operation_type: String,
    pub timestamp: u64,
    pub status: String,
    pub result_data: u64,
}

/// Full audit log entry for reproducibility.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditLog {
    pub log_id: u64,
    pub session_id: u64,
    pub operation: OperationContext,
    pub actor: Address,
}

/// Routing criteria for selecting anchors
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoutingStrategy {
    BestRate,           // Choose anchor with best exchange rate
    LowestFee,          // Choose anchor with lowest fees
    FastestSettlement,  // Choose anchor with fastest settlement time
    HighestLiquidity,   // Choose anchor with highest liquidity
    Custom,             // Custom scoring logic
}

/// Anchor metadata for routing decisions
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorMetadata {
    pub anchor: Address,
    pub reputation_score: u32,      // 0-10000 (100.00%)
    pub average_settlement_time: u64, // seconds
    pub liquidity_score: u32,       // 0-10000 (100.00%)
    pub uptime_percentage: u32,     // 0-10000 (100.00%)
    pub total_volume: u64,          // historical volume
    pub is_active: bool,
}

/// Routing request parameters
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutingRequest {
    pub request: QuoteRequest,
    pub strategy: RoutingStrategy,
    pub max_anchors: u32,           // Maximum number of anchors to consider
    pub require_kyc: bool,
    pub min_reputation: u32,        // Minimum reputation score (0-10000)
}

/// Routing result with selected anchor and alternatives
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutingResult {
    pub selected_anchor: Address,
    pub selected_quote: QuoteData,
    pub score: u64,                 // Routing score for selected anchor
    pub alternatives: Vec<AnchorOption>,
    pub routing_timestamp: u64,
}

/// Alternative anchor option
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorOption {
    pub anchor: Address,
    pub quote: QuoteData,
    pub score: u64,
    pub metadata: AnchorMetadata,
}
