#!/bin/bash

# AnchorKit Dynamic Anchor Routing Example
# This script demonstrates how to use the anchor routing features

set -e

echo "=== AnchorKit Dynamic Anchor Routing Demo ==="
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Step 1: Register Multiple Anchors
echo -e "${BLUE}Step 1: Registering Multiple Anchors${NC}"
echo "--------------------------------------"

echo "Registering Anchor 1 (Fast Settlement)..."
echo "  - Reputation: 85%"
echo "  - Settlement Time: 30 minutes"
echo "  - Liquidity: 75%"
echo ""

echo "Registering Anchor 2 (Best Rates)..."
echo "  - Reputation: 90%"
echo "  - Settlement Time: 1 hour"
echo "  - Liquidity: 85%"
echo ""

echo "Registering Anchor 3 (Low Fees)..."
echo "  - Reputation: 80%"
echo "  - Settlement Time: 45 minutes"
echo "  - Liquidity: 70%"
echo ""

# Step 2: Submit Quotes from Each Anchor
echo -e "${BLUE}Step 2: Submitting Quotes from Anchors${NC}"
echo "--------------------------------------"

echo "Anchor 1 Quote:"
echo "  - Rate: 1.00 USDC/USD"
echo "  - Fee: 1.0%"
echo "  - Min: 100 USDC, Max: 10,000 USDC"
echo ""

echo "Anchor 2 Quote:"
echo "  - Rate: 1.05 USDC/USD (Best Rate!)"
echo "  - Fee: 1.5%"
echo "  - Min: 100 USDC, Max: 10,000 USDC"
echo ""

echo "Anchor 3 Quote:"
echo "  - Rate: 0.98 USDC/USD"
echo "  - Fee: 0.5% (Lowest Fee!)"
echo "  - Min: 100 USDC, Max: 10,000 USDC"
echo ""

# Step 3: Route Transaction with Different Strategies
echo -e "${BLUE}Step 3: Routing Transactions${NC}"
echo "--------------------------------------"

echo -e "${GREEN}Strategy: Best Rate${NC}"
echo "Routing 1,000 USDC deposit..."
echo "✓ Selected: Anchor 2 (Rate: 1.05)"
echo "  Score: 945,000"
echo "  Alternatives: Anchor 1, Anchor 3"
echo ""

echo -e "${GREEN}Strategy: Lowest Fee${NC}"
echo "Routing 1,000 USDC deposit..."
echo "✓ Selected: Anchor 3 (Fee: 0.5%)"
echo "  Score: 995,000"
echo "  Alternatives: Anchor 1, Anchor 2"
echo ""

echo -e "${GREEN}Strategy: Fastest Settlement${NC}"
echo "Routing 1,000 USDC deposit..."
echo "✓ Selected: Anchor 1 (Settlement: 30 min)"
echo "  Score: 856,000"
echo "  Alternatives: Anchor 3, Anchor 2"
echo ""

echo -e "${GREEN}Strategy: Highest Liquidity${NC}"
echo "Routing 1,000 USDC deposit..."
echo "✓ Selected: Anchor 2 (Liquidity: 85%)"
echo "  Score: 850,000"
echo "  Alternatives: Anchor 1, Anchor 3"
echo ""

echo -e "${GREEN}Strategy: Custom (Balanced)${NC}"
echo "Routing 1,000 USDC deposit..."
echo "✓ Selected: Anchor 2 (Best Overall)"
echo "  Score: 892,500"
echo "  Alternatives: Anchor 1, Anchor 3"
echo ""

# Step 4: Health Monitoring
echo -e "${BLUE}Step 4: Health Monitoring${NC}"
echo "--------------------------------------"

echo "Updating health status for anchors..."
echo ""

echo "Anchor 1 Health:"
echo "  - Latency: 45ms"
echo "  - Failures: 1"
echo "  - Availability: 99.5%"
echo "  Status: ✓ Healthy"
echo ""

echo "Anchor 2 Health:"
echo "  - Latency: 50ms"
echo "  - Failures: 0"
echo "  - Availability: 99.8%"
echo "  Status: ✓ Healthy"
echo ""

echo "Anchor 3 Health:"
echo "  - Latency: 120ms"
echo "  - Failures: 5"
echo "  - Availability: 94.2%"
echo "  Status: ${YELLOW}⚠ Degraded (Below 95% threshold)${NC}"
echo ""

# Step 5: Dynamic Anchor Switching
echo -e "${BLUE}Step 5: Dynamic Anchor Switching${NC}"
echo "--------------------------------------"

echo "Current anchor: Anchor 1"
echo "Checking if switch is recommended..."
echo ""

echo "New quotes received:"
echo "  - Anchor 2: Rate improved to 1.08 (+2.9%)"
echo "  - Anchor 3: Temporarily unavailable"
echo ""

echo "Re-routing transaction..."
echo "✓ Switch recommended: Anchor 1 → Anchor 2"
echo "  Improvement: 15.2%"
echo "  Reason: Significantly better rate"
echo ""

# Step 6: KYC-Required Routing
echo -e "${BLUE}Step 6: KYC-Required Transaction Routing${NC}"
echo "--------------------------------------"

echo "Transaction requires KYC compliance..."
echo "Filtering anchors..."
echo ""

echo "Anchor 1: No KYC support - ${YELLOW}Filtered out${NC}"
echo "Anchor 2: KYC supported - ✓ Eligible"
echo "Anchor 3: No KYC support - ${YELLOW}Filtered out${NC}"
echo ""

echo "✓ Selected: Anchor 2 (Only KYC-compliant option)"
echo ""

# Step 7: Reputation-Based Filtering
echo -e "${BLUE}Step 7: Reputation-Based Filtering${NC}"
echo "--------------------------------------"

echo "Setting minimum reputation threshold: 85%"
echo "Filtering anchors..."
echo ""

echo "Anchor 1: 85% reputation - ✓ Eligible"
echo "Anchor 2: 90% reputation - ✓ Eligible"
echo "Anchor 3: 80% reputation - ${YELLOW}Filtered out${NC}"
echo ""

echo "✓ Routing among 2 eligible anchors"
echo ""

# Step 8: Anchor Deactivation
echo -e "${BLUE}Step 8: Anchor Deactivation${NC}"
echo "--------------------------------------"

echo "Deactivating Anchor 3 due to poor performance..."
echo "✓ Anchor 3 deactivated"
echo ""

echo "Active anchors: 2"
echo "  - Anchor 1: Active"
echo "  - Anchor 2: Active"
echo "  - Anchor 3: ${YELLOW}Inactive${NC}"
echo ""

# Step 9: List All Anchors
echo -e "${BLUE}Step 9: Anchor Registry${NC}"
echo "--------------------------------------"

echo "Total registered anchors: 3"
echo "Active anchors: 2"
echo ""

echo "Anchor Registry:"
echo "┌─────────┬────────────┬────────────┬───────────┬──────────┐"
echo "│ Anchor  │ Reputation │ Settlement │ Liquidity │ Status   │"
echo "├─────────┼────────────┼────────────┼───────────┼──────────┤"
echo "│ Anchor1 │    85%     │   30 min   │    75%    │ Active   │"
echo "│ Anchor2 │    90%     │   60 min   │    85%    │ Active   │"
echo "│ Anchor3 │    80%     │   45 min   │    70%    │ Inactive │"
echo "└─────────┴────────────┴────────────┴───────────┴──────────┘"
echo ""

# Summary
echo -e "${GREEN}=== Demo Complete ===${NC}"
echo ""
echo "Key Features Demonstrated:"
echo "  ✓ Multiple anchor registration"
echo "  ✓ Dynamic routing with multiple strategies"
echo "  ✓ Health monitoring and filtering"
echo "  ✓ Automatic anchor switching"
echo "  ✓ KYC-compliant routing"
echo "  ✓ Reputation-based filtering"
echo "  ✓ Anchor activation control"
echo ""

echo "Next Steps:"
echo "  1. Review ANCHOR_ROUTING.md for detailed documentation"
echo "  2. Run tests: cargo test anchor_router_tests"
echo "  3. Integrate routing into your application"
echo ""

echo "For more information, see:"
echo "  - Documentation: ANCHOR_ROUTING.md"
echo "  - Source code: src/anchor_router.rs"
echo "  - Tests: src/anchor_router_tests.rs"
echo ""
