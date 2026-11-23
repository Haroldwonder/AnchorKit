#!/bin/bash
# Anchor Info Discovery Example
# Demonstrates fetching and querying anchor metadata

set -e

echo "=== Anchor Info Discovery Example ==="
echo ""

# Configuration
ANCHOR_ADDRESS="GANCHOR123..."
DOMAIN="anchor.example.com"

echo "1. Fetching anchor info from $DOMAIN..."
# In production, this would make an HTTP request to:
# https://anchor.example.com/.well-known/stellar.toml

echo "   ✓ Fetched stellar.toml"
echo "   ✓ Cached with 1 hour TTL"
echo ""

echo "2. Querying supported assets..."
# Returns: ["USDC", "XLM", "BTC"]
echo "   Supported assets:"
echo "   - USDC (issued by GABC123...)"
echo "   - XLM (native)"
echo "   - BTC (issued by GBTC456...)"
echo ""

echo "3. Checking USDC deposit limits..."
# Returns: (min: 1000, max: 1000000)
echo "   Minimum deposit: 1,000 USDC"
echo "   Maximum deposit: 1,000,000 USDC"
echo ""

echo "4. Checking USDC deposit fees..."
# Returns: (fixed: 100, percent: 10)
echo "   Fixed fee: 100 stroops"
echo "   Percentage fee: 0.10%"
echo ""

echo "5. Validating deposit amount..."
DEPOSIT_AMOUNT=5000
echo "   Amount: $DEPOSIT_AMOUNT USDC"
echo "   ✓ Within limits (1,000 - 1,000,000)"
echo ""

echo "6. Calculating total fees..."
FIXED_FEE=100
PERCENT_FEE=10  # 0.10% = 10 basis points
VARIABLE_FEE=$((DEPOSIT_AMOUNT * PERCENT_FEE / 10000))
TOTAL_FEE=$((FIXED_FEE + VARIABLE_FEE))
echo "   Fixed fee: $FIXED_FEE stroops"
echo "   Variable fee: $VARIABLE_FEE stroops"
echo "   Total fee: $TOTAL_FEE stroops"
echo ""

echo "7. Checking service support..."
echo "   ✓ Deposits enabled for USDC"
echo "   ✓ Withdrawals enabled for USDC"
echo ""

echo "8. Getting service endpoints..."
echo "   Transfer server: https://api.example.com"
echo "   SEP-24 server: https://api.example.com/sep24"
echo "   KYC server: https://kyc.example.com"
echo "   Auth endpoint: https://auth.example.com"
echo ""

echo "=== Complete Deposit Flow ==="
echo ""

echo "Step 1: Fetch anchor info"
echo "  → GET https://$DOMAIN/.well-known/stellar.toml"
echo "  → Cache for 1 hour"
echo ""

echo "Step 2: Validate asset support"
echo "  → Check if USDC is in supported assets"
echo "  → Verify deposits are enabled"
echo ""

echo "Step 3: Validate amount"
echo "  → Check min/max limits"
echo "  → Amount: $DEPOSIT_AMOUNT USDC"
echo "  → ✓ Valid"
echo ""

echo "Step 4: Calculate fees"
echo "  → Fixed: $FIXED_FEE stroops"
echo "  → Variable: $VARIABLE_FEE stroops"
echo "  → Total: $TOTAL_FEE stroops"
echo ""

echo "Step 5: Proceed with deposit"
echo "  → POST https://api.example.com/sep24/transactions/deposit/interactive"
echo "  → Asset: USDC"
echo "  → Amount: $DEPOSIT_AMOUNT"
echo ""

echo "=== Cache Management ==="
echo ""

echo "Cache status:"
echo "  Anchor: $ANCHOR_ADDRESS"
echo "  Cached at: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "  TTL: 3600 seconds (1 hour)"
echo "  Expires at: $(date -u -d '+1 hour' +%Y-%m-%dT%H:%M:%SZ)"
echo ""

echo "Manual refresh:"
echo "  → contract.refresh_anchor_info(anchor, domain)"
echo "  → Fetches latest stellar.toml"
echo "  → Updates cache with new TTL"
echo ""

echo "=== Multi-Asset Comparison ==="
echo ""

echo "Comparing fees across assets:"
echo ""
echo "Asset | Deposit Fee | Withdrawal Fee | Min Deposit | Max Deposit"
echo "------|-------------|----------------|-------------|------------"
echo "USDC  | 100 + 0.10% | 50 + 0.05%     | 1,000       | 1,000,000"
echo "XLM   | 0 + 0.00%   | 0 + 0.00%      | 100         | 10,000,000"
echo "BTC   | 1000 + 0.20%| 500 + 0.10%    | 10,000      | 5,000,000"
echo ""

echo "=== Error Handling ==="
echo ""

echo "Common scenarios:"
echo ""
echo "1. Cache not found:"
echo "   Error: CacheNotFound"
echo "   Solution: Call fetch_anchor_info() first"
echo ""

echo "2. Cache expired:"
echo "   Error: CacheExpired"
echo "   Solution: Call refresh_anchor_info()"
echo ""

echo "3. Unsupported asset:"
echo "   Error: UnsupportedAsset"
echo "   Solution: Check get_anchor_assets() for supported list"
echo ""

echo "4. Amount out of range:"
echo "   Check: amount >= min && amount <= max"
echo "   Solution: Adjust amount or split into multiple transactions"
echo ""

echo "=== Integration Example ==="
echo ""

cat << 'EOF'
// Rust integration example
use soroban_sdk::{Env, String};

fn process_deposit(
    env: &Env,
    contract: &AnchorKitContract,
    anchor: &Address,
    asset_code: String,
    amount: u64,
) -> Result<(), Error> {
    // 1. Ensure anchor info is cached
    let toml = match contract.get_anchor_toml(anchor) {
        Ok(t) => t,
        Err(Error::CacheNotFound) | Err(Error::CacheExpired) => {
            let domain = String::from_str(env, "anchor.example.com");
            contract.fetch_anchor_info(anchor, domain, None)?
        }
        Err(e) => return Err(e),
    };

    // 2. Validate asset support
    if !contract.anchor_supports_deposits(anchor, asset_code.clone())? {
        return Err(Error::UnsupportedAsset);
    }

    // 3. Validate amount
    let (min, max) = contract.get_anchor_deposit_limits(anchor, asset_code.clone())?;
    if amount < min || amount > max {
        return Err(Error::InvalidTransactionIntent);
    }

    // 4. Calculate fees
    let (fixed, percent) = contract.get_anchor_deposit_fees(anchor, asset_code)?;
    let total_fee = fixed + (amount * percent as u64 / 10000);

    // 5. Proceed with deposit
    // ... deposit logic ...

    Ok(())
}
EOF

echo ""
echo "=== Example Complete ==="
