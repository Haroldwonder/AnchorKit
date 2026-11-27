//! SEP-38 Request for Quote (RFQ) Service Layer
//!
//! Provides typed normalization functions for all four SEP-38 endpoints:
//!
//! | Endpoint           | Function               |
//! |--------------------|------------------------|
//! | `GET /info`        | [`get_sep38_info`]     |
//! | `GET /prices`      | [`get_sep38_prices`]   |
//! | `GET /price`       | [`get_sep38_price`]    |
//! | `POST /quote`      | [`post_sep38_quote`]   |
//!
//! Callers supply the raw HTTP response fields via `Raw*` structs and receive
//! normalized, validated output types. No HTTP calls happen inside this module.
//!
//! The [`sep38_quote_to_contract_quote`] helper converts a normalised
//! [`Sep38QuoteResponse`] into the on-chain [`crate::types::Quote`] shape so
//! that routing code can compare SEP-38 firm quotes alongside on-chain quotes.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::errors::{Error, ErrorCode};

// ── Decimal helpers ──────────────────────────────────────────────────────────

/// Stellar uses 7 decimal places: 1 unit = 10_000_000 stroops.
const STROOP_SCALE: u64 = 10_000_000;

/// Parse a decimal string (e.g. `"1.0250000"`) into stroops (u64).
///
/// Supports up to 7 decimal places. Returns `None` for empty strings,
/// strings with more than one `.`, non-digit characters, or values that
/// would overflow `u64`.
fn parse_decimal_to_stroops(s: &str) -> Option<u64> {
    if s.is_empty() {
        return None;
    }

    let (int_part, frac_part) = match s.find('.') {
        Some(pos) => (&s[..pos], &s[pos + 1..]),
        None => (s, ""),
    };

    // Validate all characters are digits.
    if !int_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !frac_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    // Reject more than 7 fractional digits.
    if frac_part.len() > 7 {
        return None;
    }

    let int_val: u64 = if int_part.is_empty() {
        0
    } else {
        int_part.parse().ok()?
    };

    // Pad or truncate fractional part to exactly 7 digits.
    let mut frac_str = String::from(frac_part);
    while frac_str.len() < 7 {
        frac_str.push('0');
    }
    let frac_val: u64 = frac_str.parse().ok()?;

    int_val
        .checked_mul(STROOP_SCALE)?
        .checked_add(frac_val)
}

/// Parse a decimal string into basis points (u32, 1 bps = 0.01%).
///
/// Supports up to 4 decimal places. Returns `None` on parse failure or overflow.
fn parse_decimal_to_bps(s: &str) -> Option<u32> {
    if s.is_empty() {
        return None;
    }

    let (int_part, frac_part) = match s.find('.') {
        Some(pos) => (&s[..pos], &s[pos + 1..]),
        None => (s, ""),
    };

    if !int_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !frac_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if frac_part.len() > 4 {
        return None;
    }

    let int_val: u32 = if int_part.is_empty() {
        0
    } else {
        int_part.parse().ok()?
    };

    let mut frac_str = String::from(frac_part);
    while frac_str.len() < 4 {
        frac_str.push('0');
    }
    let frac_val: u32 = frac_str.parse().ok()?;

    int_val.checked_mul(10_000)?.checked_add(frac_val)
}

/// Compute fee percentage in basis points from absolute fee and trade amounts
/// (both expressed as stroop-scaled `u64`).
///
/// Returns `0` when `trade_amount` is zero to avoid division-by-zero.
/// Saturates at `u32::MAX` rather than panicking on overflow.
fn fee_to_bps(fee_stroops: u64, trade_amount_stroops: u64) -> u32 {
    if trade_amount_stroops == 0 {
        return 0;
    }
    // fee_bps = (fee / trade_amount) * 10_000
    // Use u128 intermediary to avoid overflow before the divide.
    let bps = (fee_stroops as u128)
        .saturating_mul(10_000)
        / (trade_amount_stroops as u128);
    bps.min(u32::MAX as u128) as u32
}

// ── Raw anchor response shapes ────────────────────────────────────────────────

/// A single delivery-method entry within a SEP-38 `/info` asset record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawDeliveryMethod {
    /// Machine-readable name (e.g. `"SEPA"`, `"ACH"`).
    pub name: String,
    /// Human-readable description shown to end users.
    pub description: String,
}

/// A single asset entry from the SEP-38 `/info` response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawSep38Asset {
    /// SEP-38 asset identifier: `"stellar:<code>:<issuer>"` or `"iso4217:<code>"`.
    pub asset: String,
    /// Delivery methods available when selling this asset.
    pub sell_delivery_methods: Vec<RawDeliveryMethod>,
    /// Delivery methods available when buying this asset.
    pub buy_delivery_methods: Vec<RawDeliveryMethod>,
    /// ISO 3166-1 alpha-2 country codes where this asset is available.
    pub country_codes: Vec<String>,
}

/// Raw fields from an anchor's `GET /info` response.
#[derive(Clone, Debug)]
pub struct RawSep38InfoResponse {
    pub assets: Vec<RawSep38Asset>,
}

/// A single price entry from the SEP-38 `GET /prices` response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawSep38Price {
    /// SEP-38 asset identifier of the asset being priced.
    pub asset: String,
    /// Exchange price as a decimal string (units of `sell_asset` per 1 `buy_asset`).
    pub price: String,
    /// Number of decimals used in the price.
    pub decimals: u32,
}

/// Raw fields from an anchor's `GET /prices` response.
#[derive(Clone, Debug)]
pub struct RawSep38PricesResponse {
    /// List of assets the anchor will buy, with their prices.
    pub buy_assets: Vec<RawSep38Price>,
}

/// A single fee-line-item within a SEP-38 price or quote response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawFeeDetail {
    pub name: String,
    pub amount: String,
    pub description: Option<String>,
}

/// The fee breakdown from a SEP-38 price or quote response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawSep38Fee {
    /// Total fee amount as a decimal string.
    pub total: String,
    /// Asset code the fee is charged in.
    pub asset: String,
    /// Individual fee line items.
    pub details: Vec<RawFeeDetail>,
}

/// Raw fields from an anchor's `GET /price` response.
///
/// Fields are decimal strings as returned by the anchor before any parsing.
#[derive(Clone, Debug)]
pub struct RawSep38PriceResponse {
    /// Combined price including fees, as a decimal string.
    pub total_price: String,
    /// Mid-market price excluding fees, as a decimal string.
    pub price: String,
    /// SEP-38 asset identifier for the asset being sold.
    pub sell_asset: String,
    /// Amount sold as a decimal string.
    pub sell_amount: String,
    /// SEP-38 asset identifier for the asset being bought.
    pub buy_asset: String,
    /// Amount bought as a decimal string.
    pub buy_amount: String,
    /// Optional fee breakdown.
    pub fee: Option<RawSep38Fee>,
    /// Optional expiry time (Unix seconds). If `None`, the price is indicative only.
    pub expires_at: Option<u64>,
}

/// Raw fields from an anchor's `POST /quote` response.
///
/// Unlike `/price`, a `POST /quote` returns a firm, committed quote with an ID.
#[derive(Clone, Debug)]
pub struct RawSep38QuoteResponse {
    /// Unique anchor-assigned quote identifier.
    pub id: String,
    /// Unix timestamp (seconds) when this firm quote expires.
    pub expires_at: u64,
    /// Combined price including fees, as a decimal string.
    pub total_price: String,
    /// Mid-market price excluding fees, as a decimal string.
    pub price: String,
    /// SEP-38 asset identifier for the asset being sold.
    pub sell_asset: String,
    /// Amount sold as a decimal string.
    pub sell_amount: String,
    /// SEP-38 asset identifier for the asset being bought.
    pub buy_asset: String,
    /// Amount bought as a decimal string.
    pub buy_amount: String,
    /// Optional fee breakdown.
    pub fee: Option<RawSep38Fee>,
}

// ── Normalized output types ───────────────────────────────────────────────────

/// Validated delivery method from a SEP-38 `/info` response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep38DeliveryMethod {
    pub name: String,
    pub description: String,
}

/// Validated asset entry from a SEP-38 `/info` response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep38Asset {
    pub asset: String,
    pub sell_delivery_methods: Vec<Sep38DeliveryMethod>,
    pub buy_delivery_methods: Vec<Sep38DeliveryMethod>,
    pub country_codes: Vec<String>,
}

/// Normalized response from `GET /info`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep38InfoResponse {
    pub assets: Vec<Sep38Asset>,
}

/// Validated price entry from a SEP-38 `/prices` response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep38Price {
    /// SEP-38 asset identifier.
    pub asset: String,
    /// Exchange price in stroops (10^-7 units).
    pub price_stroops: u64,
    /// Number of significant decimal places reported by the anchor.
    pub decimals: u32,
}

/// Normalized response from `GET /prices`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep38PricesResponse {
    pub buy_assets: Vec<Sep38Price>,
}

/// Validated fee line item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep38FeeDetail {
    pub name: String,
    pub amount_stroops: u64,
    pub description: Option<String>,
}

/// Validated fee breakdown.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep38Fee {
    pub total_stroops: u64,
    pub asset: String,
    pub details: Vec<Sep38FeeDetail>,
}

/// Normalized response from `GET /price`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep38PriceResponse {
    /// Total price (including fees) in stroops.
    pub total_price_stroops: u64,
    /// Mid-market price (excluding fees) in stroops.
    pub price_stroops: u64,
    pub sell_asset: String,
    pub sell_amount_stroops: u64,
    pub buy_asset: String,
    pub buy_amount_stroops: u64,
    pub fee: Option<Sep38Fee>,
    /// Expiry Unix timestamp, if the anchor provided one.
    pub expires_at: Option<u64>,
}

/// Normalized response from `POST /quote` — a firm, committed quote.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sep38QuoteResponse {
    /// Anchor-assigned quote ID.
    pub id: String,
    /// Unix timestamp (seconds) when this firm quote expires.
    pub expires_at: u64,
    /// Total price (including fees) in stroops.
    pub total_price_stroops: u64,
    /// Mid-market price (excluding fees) in stroops.
    pub price_stroops: u64,
    pub sell_asset: String,
    pub sell_amount_stroops: u64,
    pub buy_asset: String,
    pub buy_amount_stroops: u64,
    pub fee: Option<Sep38Fee>,
}

// ── Internal helpers ─────────────────────────────────────────────────────────

fn normalize_delivery_method(raw: RawDeliveryMethod) -> Result<Sep38DeliveryMethod, Error> {
    if raw.name.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "delivery method name must not be empty",
            "name",
        ));
    }
    Ok(Sep38DeliveryMethod {
        name: raw.name,
        description: raw.description,
    })
}

fn normalize_fee(raw: RawSep38Fee) -> Result<Sep38Fee, Error> {
    if raw.asset.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "fee asset must not be empty",
            "fee.asset",
        ));
    }
    let total_stroops = parse_decimal_to_stroops(&raw.total).ok_or_else(|| {
        Error::with_context(
            ErrorCode::InvalidQuote,
            "fee total is not a valid decimal amount",
            &raw.total,
        )
    })?;
    let mut details = Vec::new();
    for d in raw.details {
        if d.name.is_empty() {
            return Err(Error::with_context(
                ErrorCode::ValidationError,
                "fee detail name must not be empty",
                "fee.details[].name",
            ));
        }
        let amount_stroops = parse_decimal_to_stroops(&d.amount).ok_or_else(|| {
            Error::with_context(
                ErrorCode::InvalidQuote,
                "fee detail amount is not a valid decimal",
                &d.amount,
            )
        })?;
        details.push(Sep38FeeDetail {
            name: d.name,
            amount_stroops,
            description: d.description,
        });
    }
    Ok(Sep38Fee {
        total_stroops,
        asset: raw.asset,
        details,
    })
}

// ── Public service functions ──────────────────────────────────────────────────

/// Normalize a raw `GET /info` response into a typed [`Sep38InfoResponse`].
///
/// Returns `Err(ErrorCode::ValidationError)` when:
/// - The assets list is empty.
/// - Any asset identifier is an empty string.
/// - Any delivery-method name is empty.
pub fn get_sep38_info(raw: RawSep38InfoResponse) -> Result<Sep38InfoResponse, Error> {
    if raw.assets.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "SEP-38 /info returned no assets",
            "assets",
        ));
    }

    let mut assets = Vec::new();
    for a in raw.assets {
        if a.asset.is_empty() {
            return Err(Error::with_context(
                ErrorCode::ValidationError,
                "asset identifier must not be empty",
                "asset",
            ));
        }
        let mut sell_methods = Vec::new();
        for m in a.sell_delivery_methods {
            sell_methods.push(normalize_delivery_method(m)?);
        }
        let mut buy_methods = Vec::new();
        for m in a.buy_delivery_methods {
            buy_methods.push(normalize_delivery_method(m)?);
        }
        assets.push(Sep38Asset {
            asset: a.asset,
            sell_delivery_methods: sell_methods,
            buy_delivery_methods: buy_methods,
            country_codes: a.country_codes,
        });
    }

    Ok(Sep38InfoResponse { assets })
}

/// Normalize a raw `GET /prices` response into a typed [`Sep38PricesResponse`].
///
/// Returns `Err(ErrorCode::ValidationError)` when the buy-assets list is empty
/// or any asset identifier is empty.
/// Returns `Err(ErrorCode::InvalidQuote)` when any price string cannot be parsed.
pub fn get_sep38_prices(raw: RawSep38PricesResponse) -> Result<Sep38PricesResponse, Error> {
    if raw.buy_assets.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "SEP-38 /prices returned no buy_assets",
            "buy_assets",
        ));
    }

    let mut buy_assets = Vec::new();
    for p in raw.buy_assets {
        if p.asset.is_empty() {
            return Err(Error::with_context(
                ErrorCode::ValidationError,
                "buy_asset identifier must not be empty",
                "asset",
            ));
        }
        let price_stroops = parse_decimal_to_stroops(&p.price).ok_or_else(|| {
            Error::with_context(
                ErrorCode::InvalidQuote,
                "price is not a valid decimal amount",
                &p.price,
            )
        })?;
        if price_stroops == 0 {
            return Err(Error::with_context(
                ErrorCode::InvalidAmount,
                "price must be greater than zero",
                &p.price,
            ));
        }
        buy_assets.push(Sep38Price {
            asset: p.asset,
            price_stroops,
            decimals: p.decimals,
        });
    }

    Ok(Sep38PricesResponse { buy_assets })
}

/// Normalize a raw `GET /price` (indicative price) response.
///
/// Returns `Err(ErrorCode::ValidationError)` when any required string is empty.
/// Returns `Err(ErrorCode::InvalidAmount)` when any amount parses to zero.
/// Returns `Err(ErrorCode::InvalidQuote)` when any decimal string is malformed.
/// Returns `Err(ErrorCode::StaleQuote)` when `expires_at` is in the past
/// relative to `current_time`.
pub fn get_sep38_price(
    raw: RawSep38PriceResponse,
    current_time: u64,
) -> Result<Sep38PriceResponse, Error> {
    if raw.sell_asset.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "sell_asset must not be empty",
            "sell_asset",
        ));
    }
    if raw.buy_asset.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "buy_asset must not be empty",
            "buy_asset",
        ));
    }
    if raw.sell_asset == raw.buy_asset {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "sell_asset and buy_asset must differ",
            &raw.sell_asset,
        ));
    }

    // Check expiry before parsing amounts.
    if let Some(expires_at) = raw.expires_at {
        if expires_at <= current_time {
            return Err(Error::from_code(ErrorCode::StaleQuote));
        }
    }

    let total_price_stroops = parse_decimal_to_stroops(&raw.total_price).ok_or_else(|| {
        Error::with_context(
            ErrorCode::InvalidQuote,
            "total_price is not a valid decimal amount",
            &raw.total_price,
        )
    })?;
    let price_stroops = parse_decimal_to_stroops(&raw.price).ok_or_else(|| {
        Error::with_context(
            ErrorCode::InvalidQuote,
            "price is not a valid decimal amount",
            &raw.price,
        )
    })?;
    let sell_amount_stroops = parse_decimal_to_stroops(&raw.sell_amount).ok_or_else(|| {
        Error::with_context(
            ErrorCode::InvalidQuote,
            "sell_amount is not a valid decimal amount",
            &raw.sell_amount,
        )
    })?;
    let buy_amount_stroops = parse_decimal_to_stroops(&raw.buy_amount).ok_or_else(|| {
        Error::with_context(
            ErrorCode::InvalidQuote,
            "buy_amount is not a valid decimal amount",
            &raw.buy_amount,
        )
    })?;

    if sell_amount_stroops == 0 {
        return Err(Error::with_context(
            ErrorCode::InvalidAmount,
            "sell_amount must be greater than zero",
            &raw.sell_amount,
        ));
    }
    if buy_amount_stroops == 0 {
        return Err(Error::with_context(
            ErrorCode::InvalidAmount,
            "buy_amount must be greater than zero",
            &raw.buy_amount,
        ));
    }

    let fee = match raw.fee {
        Some(f) => Some(normalize_fee(f)?),
        None => None,
    };

    Ok(Sep38PriceResponse {
        total_price_stroops,
        price_stroops,
        sell_asset: raw.sell_asset,
        sell_amount_stroops,
        buy_asset: raw.buy_asset,
        buy_amount_stroops,
        fee,
        expires_at: raw.expires_at,
    })
}

/// Normalize a raw `POST /quote` (firm quote) response.
///
/// Returns `Err(ErrorCode::ValidationError)` when the quote `id` is empty or
/// when required asset identifiers are missing.
/// Returns `Err(ErrorCode::StaleQuote)` when `expires_at <= current_time`.
/// Returns `Err(ErrorCode::InvalidQuote)` when any decimal amount is malformed.
/// Returns `Err(ErrorCode::InvalidAmount)` when any amount parses to zero.
pub fn post_sep38_quote(
    raw: RawSep38QuoteResponse,
    current_time: u64,
) -> Result<Sep38QuoteResponse, Error> {
    if raw.id.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "quote id must not be empty",
            "id",
        ));
    }
    if raw.sell_asset.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "sell_asset must not be empty",
            "sell_asset",
        ));
    }
    if raw.buy_asset.is_empty() {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "buy_asset must not be empty",
            "buy_asset",
        ));
    }
    if raw.sell_asset == raw.buy_asset {
        return Err(Error::with_context(
            ErrorCode::ValidationError,
            "sell_asset and buy_asset must differ",
            &raw.sell_asset,
        ));
    }
    if raw.expires_at <= current_time {
        return Err(Error::from_code(ErrorCode::StaleQuote));
    }

    let total_price_stroops = parse_decimal_to_stroops(&raw.total_price).ok_or_else(|| {
        Error::with_context(
            ErrorCode::InvalidQuote,
            "total_price is not a valid decimal amount",
            &raw.total_price,
        )
    })?;
    let price_stroops = parse_decimal_to_stroops(&raw.price).ok_or_else(|| {
        Error::with_context(
            ErrorCode::InvalidQuote,
            "price is not a valid decimal amount",
            &raw.price,
        )
    })?;
    let sell_amount_stroops = parse_decimal_to_stroops(&raw.sell_amount).ok_or_else(|| {
        Error::with_context(
            ErrorCode::InvalidQuote,
            "sell_amount is not a valid decimal amount",
            &raw.sell_amount,
        )
    })?;
    let buy_amount_stroops = parse_decimal_to_stroops(&raw.buy_amount).ok_or_else(|| {
        Error::with_context(
            ErrorCode::InvalidQuote,
            "buy_amount is not a valid decimal amount",
            &raw.buy_amount,
        )
    })?;

    if sell_amount_stroops == 0 {
        return Err(Error::with_context(
            ErrorCode::InvalidAmount,
            "sell_amount must be greater than zero",
            &raw.sell_amount,
        ));
    }
    if buy_amount_stroops == 0 {
        return Err(Error::with_context(
            ErrorCode::InvalidAmount,
            "buy_amount must be greater than zero",
            &raw.buy_amount,
        ));
    }

    let fee = match raw.fee {
        Some(f) => Some(normalize_fee(f)?),
        None => None,
    };

    Ok(Sep38QuoteResponse {
        id: raw.id,
        expires_at: raw.expires_at,
        total_price_stroops,
        price_stroops,
        sell_asset: raw.sell_asset,
        sell_amount_stroops,
        buy_asset: raw.buy_asset,
        buy_amount_stroops,
        fee,
    })
}

/// Convert a normalised [`Sep38QuoteResponse`] into the on-chain
/// [`crate::types::Quote`] contract shape.
///
/// This bridges off-chain SEP-38 firm quotes into the routing layer so that
/// callers can compare anchor-submitted on-chain quotes alongside SEP-38 quotes
/// using existing routing strategies.
///
/// # Field mapping
///
/// | `Quote` field       | Source                                                      |
/// |---------------------|-------------------------------------------------------------|
/// | `quote_id`          | 0 — not assigned until submitted on-chain                   |
/// | `anchor`            | `anchor` parameter (caller-supplied Soroban `Address`)      |
/// | `base_asset`        | `sep38_quote.sell_asset`                                    |
/// | `quote_asset`       | `sep38_quote.buy_asset`                                     |
/// | `rate`              | `sep38_quote.price_stroops`                                 |
/// | `fee_percentage`    | fee total / sell_amount in bps; 0 if no fee field present   |
/// | `minimum_amount`    | 0 — SEP-38 quotes are for a fixed amount, no range          |
/// | `maximum_amount`    | `sep38_quote.sell_amount_stroops`                           |
/// | `valid_until`       | `sep38_quote.expires_at`                                    |
///
/// Returns `Err(ErrorCode::InvalidQuote)` when the price is zero.
pub fn sep38_quote_to_contract_quote(
    sep38_quote: &Sep38QuoteResponse,
    anchor: soroban_sdk::Address,
    env: &soroban_sdk::Env,
) -> Result<crate::types::Quote, Error> {
    if sep38_quote.price_stroops == 0 {
        return Err(Error::from_code(ErrorCode::InvalidQuote));
    }

    let fee_bps = match &sep38_quote.fee {
        Some(f) => fee_to_bps(f.total_stroops, sep38_quote.sell_amount_stroops),
        None => 0,
    };

    let base_asset =
        soroban_sdk::String::from_str(env, sep38_quote.sell_asset.as_str());
    let quote_asset =
        soroban_sdk::String::from_str(env, sep38_quote.buy_asset.as_str());

    Ok(crate::types::Quote {
        quote_id: 0,
        anchor,
        base_asset,
        quote_asset,
        rate: sep38_quote.price_stroops,
        fee_percentage: fee_bps,
        minimum_amount: 0,
        maximum_amount: sep38_quote.sell_amount_stroops,
        valid_until: sep38_quote.expires_at,
    })
}

/// Map an HTTP status code from any SEP-38 endpoint to an SDK error.
///
/// | HTTP status | Error                  |
/// |-------------|------------------------|
/// | 404         | `QuoteNotFound`        |
/// | 408         | `TransportTimeout`     |
/// | 429         | `RateLimitExceeded`    |
/// | 5xx         | `ServerError`          |
/// | 2xx         | calls `response_fn()`  |
/// | other       | `ValidationError`      |
pub fn handle_sep38_http_status<T>(
    http_status: u32,
    response_fn: impl FnOnce() -> Result<T, Error>,
) -> Result<T, Error> {
    match http_status {
        404 => Err(Error::from_code(ErrorCode::QuoteNotFound)),
        408 => Err(Error::from_code(ErrorCode::TransportTimeout)),
        429 => Err(Error::rate_limit_exceeded()),
        500..=599 => Err(Error::from_code(ErrorCode::ServerError)),
        200..=299 => response_fn(),
        _ => Err(Error::with_context(
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
    use crate::errors::ErrorCode;

    // ── parse_decimal_to_stroops ─────────────────────────────────────────────

    #[test]
    fn parse_stroops_integer_only() {
        assert_eq!(parse_decimal_to_stroops("1"), Some(10_000_000));
    }

    #[test]
    fn parse_stroops_with_fraction() {
        assert_eq!(parse_decimal_to_stroops("1.025"), Some(10_250_000));
    }

    #[test]
    fn parse_stroops_full_seven_decimals() {
        assert_eq!(parse_decimal_to_stroops("1.0000001"), Some(10_000_001));
    }

    #[test]
    fn parse_stroops_zero() {
        assert_eq!(parse_decimal_to_stroops("0"), Some(0));
    }

    #[test]
    fn parse_stroops_empty_fails() {
        assert_eq!(parse_decimal_to_stroops(""), None);
    }

    #[test]
    fn parse_stroops_too_many_decimals_fails() {
        assert_eq!(parse_decimal_to_stroops("1.00000001"), None);
    }

    #[test]
    fn parse_stroops_non_digit_fails() {
        assert_eq!(parse_decimal_to_stroops("1.0x"), None);
    }

    // ── fee_to_bps ───────────────────────────────────────────────────────────

    #[test]
    fn fee_to_bps_exact() {
        // 1% fee: fee = 100_000 stroops on 10_000_000 stroops → 100 bps
        assert_eq!(fee_to_bps(100_000, 10_000_000), 100);
    }

    #[test]
    fn fee_to_bps_zero_trade_amount() {
        assert_eq!(fee_to_bps(500, 0), 0);
    }

    #[test]
    fn fee_to_bps_zero_fee() {
        assert_eq!(fee_to_bps(0, 10_000_000), 0);
    }

    // ── get_sep38_info ───────────────────────────────────────────────────────

    fn sample_asset(id: &str) -> RawSep38Asset {
        RawSep38Asset {
            asset: String::from(id),
            sell_delivery_methods: alloc::vec![RawDeliveryMethod {
                name: String::from("SEPA"),
                description: String::from("SEPA transfer"),
            }],
            buy_delivery_methods: alloc::vec![],
            country_codes: alloc::vec![String::from("DE")],
        }
    }

    #[test]
    fn get_info_valid() {
        let raw = RawSep38InfoResponse {
            assets: alloc::vec![sample_asset("iso4217:EUR")],
        };
        let res = get_sep38_info(raw).unwrap();
        assert_eq!(res.assets.len(), 1);
        assert_eq!(res.assets[0].asset, "iso4217:EUR");
        assert_eq!(res.assets[0].sell_delivery_methods.len(), 1);
        assert_eq!(res.assets[0].sell_delivery_methods[0].name, "SEPA");
    }

    #[test]
    fn get_info_empty_assets_fails() {
        let raw = RawSep38InfoResponse { assets: alloc::vec![] };
        let err = get_sep38_info(raw).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn get_info_empty_asset_id_fails() {
        let mut a = sample_asset("");
        a.asset = String::from("");
        let raw = RawSep38InfoResponse { assets: alloc::vec![a] };
        let err = get_sep38_info(raw).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn get_info_empty_delivery_method_name_fails() {
        let raw = RawSep38InfoResponse {
            assets: alloc::vec![RawSep38Asset {
                asset: String::from("iso4217:USD"),
                sell_delivery_methods: alloc::vec![RawDeliveryMethod {
                    name: String::from(""),
                    description: String::from("desc"),
                }],
                buy_delivery_methods: alloc::vec![],
                country_codes: alloc::vec![],
            }],
        };
        let err = get_sep38_info(raw).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    // ── get_sep38_prices ─────────────────────────────────────────────────────

    #[test]
    fn get_prices_valid() {
        let raw = RawSep38PricesResponse {
            buy_assets: alloc::vec![RawSep38Price {
                asset: String::from("stellar:USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
                price: String::from("1.0250000"),
                decimals: 7,
            }],
        };
        let res = get_sep38_prices(raw).unwrap();
        assert_eq!(res.buy_assets.len(), 1);
        assert_eq!(res.buy_assets[0].price_stroops, 10_250_000);
        assert_eq!(res.buy_assets[0].decimals, 7);
    }

    #[test]
    fn get_prices_empty_buy_assets_fails() {
        let raw = RawSep38PricesResponse { buy_assets: alloc::vec![] };
        let err = get_sep38_prices(raw).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn get_prices_empty_asset_id_fails() {
        let raw = RawSep38PricesResponse {
            buy_assets: alloc::vec![RawSep38Price {
                asset: String::from(""),
                price: String::from("1.0"),
                decimals: 7,
            }],
        };
        let err = get_sep38_prices(raw).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn get_prices_zero_price_fails() {
        let raw = RawSep38PricesResponse {
            buy_assets: alloc::vec![RawSep38Price {
                asset: String::from("iso4217:USD"),
                price: String::from("0"),
                decimals: 7,
            }],
        };
        let err = get_sep38_prices(raw).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidAmount);
    }

    #[test]
    fn get_prices_malformed_price_fails() {
        let raw = RawSep38PricesResponse {
            buy_assets: alloc::vec![RawSep38Price {
                asset: String::from("iso4217:USD"),
                price: String::from("bad"),
                decimals: 7,
            }],
        };
        let err = get_sep38_prices(raw).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidQuote);
    }

    // ── get_sep38_price ──────────────────────────────────────────────────────

    fn sample_price_raw() -> RawSep38PriceResponse {
        RawSep38PriceResponse {
            total_price: String::from("1.0250000"),
            price: String::from("1.0200000"),
            sell_asset: String::from("iso4217:USD"),
            sell_amount: String::from("100.0000000"),
            buy_asset: String::from("stellar:USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
            buy_amount: String::from("97.5609756"),
            fee: None,
            expires_at: Some(9999999),
        }
    }

    #[test]
    fn get_price_valid() {
        let res = get_sep38_price(sample_price_raw(), 1000).unwrap();
        assert_eq!(res.sell_asset, "iso4217:USD");
        assert_eq!(res.sell_amount_stroops, 1_000_000_000);
        assert_eq!(res.total_price_stroops, 10_250_000);
        assert_eq!(res.expires_at, Some(9999999));
    }

    #[test]
    fn get_price_expired_fails() {
        let mut raw = sample_price_raw();
        raw.expires_at = Some(500);
        let err = get_sep38_price(raw, 1000).unwrap_err();
        assert_eq!(err.code, ErrorCode::StaleQuote);
    }

    #[test]
    fn get_price_no_expiry_allowed() {
        let mut raw = sample_price_raw();
        raw.expires_at = None;
        let res = get_sep38_price(raw, 1000);
        assert!(res.is_ok());
    }

    #[test]
    fn get_price_empty_sell_asset_fails() {
        let mut raw = sample_price_raw();
        raw.sell_asset = String::from("");
        let err = get_sep38_price(raw, 1000).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn get_price_empty_buy_asset_fails() {
        let mut raw = sample_price_raw();
        raw.buy_asset = String::from("");
        let err = get_sep38_price(raw, 1000).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn get_price_same_assets_fails() {
        let mut raw = sample_price_raw();
        raw.buy_asset = String::from("iso4217:USD");
        let err = get_sep38_price(raw, 1000).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn get_price_zero_sell_amount_fails() {
        let mut raw = sample_price_raw();
        raw.sell_amount = String::from("0");
        let err = get_sep38_price(raw, 1000).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidAmount);
    }

    #[test]
    fn get_price_zero_buy_amount_fails() {
        let mut raw = sample_price_raw();
        raw.buy_amount = String::from("0");
        let err = get_sep38_price(raw, 1000).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidAmount);
    }

    #[test]
    fn get_price_with_fee() {
        let mut raw = sample_price_raw();
        raw.fee = Some(RawSep38Fee {
            total: String::from("2.5000000"),
            asset: String::from("iso4217:USD"),
            details: alloc::vec![RawFeeDetail {
                name: String::from("Service fee"),
                amount: String::from("2.5000000"),
                description: Some(String::from("Anchor service fee")),
            }],
        });
        let res = get_sep38_price(raw, 1000).unwrap();
        let fee = res.fee.unwrap();
        assert_eq!(fee.total_stroops, 25_000_000);
        assert_eq!(fee.asset, "iso4217:USD");
        assert_eq!(fee.details.len(), 1);
        assert_eq!(fee.details[0].amount_stroops, 25_000_000);
    }

    // ── post_sep38_quote ─────────────────────────────────────────────────────

    fn sample_quote_raw() -> RawSep38QuoteResponse {
        RawSep38QuoteResponse {
            id: String::from("quote-abc-123"),
            expires_at: 9999999,
            total_price: String::from("1.0250000"),
            price: String::from("1.0200000"),
            sell_asset: String::from("iso4217:USD"),
            sell_amount: String::from("100.0000000"),
            buy_asset: String::from("stellar:USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
            buy_amount: String::from("97.5609756"),
            fee: None,
        }
    }

    #[test]
    fn post_quote_valid() {
        let res = post_sep38_quote(sample_quote_raw(), 1000).unwrap();
        assert_eq!(res.id, "quote-abc-123");
        assert_eq!(res.expires_at, 9999999);
        assert_eq!(res.sell_asset, "iso4217:USD");
        assert_eq!(res.price_stroops, 10_200_000);
        assert_eq!(res.sell_amount_stroops, 1_000_000_000);
    }

    #[test]
    fn post_quote_expired_fails() {
        let mut raw = sample_quote_raw();
        raw.expires_at = 500;
        let err = post_sep38_quote(raw, 1000).unwrap_err();
        assert_eq!(err.code, ErrorCode::StaleQuote);
    }

    #[test]
    fn post_quote_expires_at_equal_to_current_time_fails() {
        let mut raw = sample_quote_raw();
        raw.expires_at = 1000;
        let err = post_sep38_quote(raw, 1000).unwrap_err();
        assert_eq!(err.code, ErrorCode::StaleQuote);
    }

    #[test]
    fn post_quote_empty_id_fails() {
        let mut raw = sample_quote_raw();
        raw.id = String::from("");
        let err = post_sep38_quote(raw, 1000).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn post_quote_empty_sell_asset_fails() {
        let mut raw = sample_quote_raw();
        raw.sell_asset = String::from("");
        let err = post_sep38_quote(raw, 1000).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn post_quote_empty_buy_asset_fails() {
        let mut raw = sample_quote_raw();
        raw.buy_asset = String::from("");
        let err = post_sep38_quote(raw, 1000).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn post_quote_same_assets_fails() {
        let mut raw = sample_quote_raw();
        raw.buy_asset = String::from("iso4217:USD");
        let err = post_sep38_quote(raw, 1000).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    #[test]
    fn post_quote_malformed_price_fails() {
        let mut raw = sample_quote_raw();
        raw.price = String::from("not-a-number");
        let err = post_sep38_quote(raw, 1000).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidQuote);
    }

    #[test]
    fn post_quote_zero_sell_amount_fails() {
        let mut raw = sample_quote_raw();
        raw.sell_amount = String::from("0");
        let err = post_sep38_quote(raw, 1000).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidAmount);
    }

    #[test]
    fn post_quote_with_fee() {
        let mut raw = sample_quote_raw();
        raw.fee = Some(RawSep38Fee {
            total: String::from("1.0000000"),
            asset: String::from("iso4217:USD"),
            details: alloc::vec![],
        });
        let res = post_sep38_quote(raw, 1000).unwrap();
        let fee = res.fee.unwrap();
        assert_eq!(fee.total_stroops, 10_000_000);
    }

    // ── sep38_quote_to_contract_quote ────────────────────────────────────────

    #[test]
    fn contract_quote_conversion_roundtrip() {
        use soroban_sdk::{testutils::Address as _, Address, Env};

        let env = Env::default();
        let anchor = Address::generate(&env);

        let sep38 = Sep38QuoteResponse {
            id: String::from("q1"),
            expires_at: 9999999,
            total_price_stroops: 10_250_000,
            price_stroops: 10_200_000,
            sell_asset: String::from("iso4217:USD"),
            sell_amount_stroops: 1_000_000_000,
            buy_asset: String::from("stellar:USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
            buy_amount_stroops: 975_609_756,
            fee: None,
        };

        let q = sep38_quote_to_contract_quote(&sep38, anchor.clone(), &env).unwrap();
        assert_eq!(q.anchor, anchor);
        assert_eq!(q.rate, 10_200_000);
        assert_eq!(q.fee_percentage, 0); // no fee
        assert_eq!(q.maximum_amount, 1_000_000_000);
        assert_eq!(q.valid_until, 9999999);
        assert_eq!(q.minimum_amount, 0);
        assert_eq!(q.quote_id, 0);
    }

    #[test]
    fn contract_quote_conversion_with_fee_bps() {
        use soroban_sdk::{testutils::Address as _, Address, Env};

        let env = Env::default();
        let anchor = Address::generate(&env);

        // 1% fee: 10_000_000 fee stroops on 1_000_000_000 sell_amount = 100 bps
        let sep38 = Sep38QuoteResponse {
            id: String::from("q2"),
            expires_at: 9999999,
            total_price_stroops: 10_250_000,
            price_stroops: 10_200_000,
            sell_asset: String::from("iso4217:USD"),
            sell_amount_stroops: 1_000_000_000,
            buy_asset: String::from("stellar:USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
            buy_amount_stroops: 975_609_756,
            fee: Some(Sep38Fee {
                total_stroops: 10_000_000, // 1 USD
                asset: String::from("iso4217:USD"),
                details: alloc::vec![],
            }),
        };

        let q = sep38_quote_to_contract_quote(&sep38, anchor, &env).unwrap();
        assert_eq!(q.fee_percentage, 100); // 1% = 100 bps
    }

    #[test]
    fn contract_quote_zero_price_fails() {
        use soroban_sdk::{testutils::Address as _, Address, Env};

        let env = Env::default();
        let anchor = Address::generate(&env);

        let sep38 = Sep38QuoteResponse {
            id: String::from("q3"),
            expires_at: 9999999,
            total_price_stroops: 0,
            price_stroops: 0,
            sell_asset: String::from("iso4217:USD"),
            sell_amount_stroops: 1_000_000_000,
            buy_asset: String::from("stellar:USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
            buy_amount_stroops: 975_609_756,
            fee: None,
        };

        let err = sep38_quote_to_contract_quote(&sep38, anchor, &env).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidQuote);
    }

    // ── handle_sep38_http_status ─────────────────────────────────────────────

    #[test]
    fn http_200_calls_response_fn() {
        let result: Result<u32, Error> =
            handle_sep38_http_status(200, || Ok(42));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn http_404_returns_quote_not_found() {
        let result: Result<u32, Error> =
            handle_sep38_http_status(404, || Ok(0));
        assert_eq!(result.unwrap_err().code, ErrorCode::QuoteNotFound);
    }

    #[test]
    fn http_408_returns_transport_timeout() {
        let result: Result<u32, Error> =
            handle_sep38_http_status(408, || Ok(0));
        assert_eq!(result.unwrap_err().code, ErrorCode::TransportTimeout);
    }

    #[test]
    fn http_429_returns_rate_limit_exceeded() {
        let result: Result<u32, Error> =
            handle_sep38_http_status(429, || Ok(0));
        assert_eq!(result.unwrap_err().code, ErrorCode::RateLimitExceeded);
    }

    #[test]
    fn http_500_returns_server_error() {
        let result: Result<u32, Error> =
            handle_sep38_http_status(500, || Ok(0));
        assert_eq!(result.unwrap_err().code, ErrorCode::ServerError);
    }

    #[test]
    fn http_503_returns_server_error() {
        let result: Result<u32, Error> =
            handle_sep38_http_status(503, || Ok(0));
        assert_eq!(result.unwrap_err().code, ErrorCode::ServerError);
    }

    #[test]
    fn http_400_returns_validation_error() {
        let result: Result<u32, Error> =
            handle_sep38_http_status(400, || Ok(0));
        assert_eq!(result.unwrap_err().code, ErrorCode::ValidationError);
    }
}
