#!/bin/bash
# Example: Using Anchor Health Score
#
# This script demonstrates how to use the get_anchor_health_score function
# to evaluate anchor reliability.

set -e

echo "=== Anchor Health Score Example ==="
echo ""

# Simulated anchor addresses (in practice, these would be real Stellar addresses)
ANCHOR_PREMIUM="GAPREMIUMANCHOR1234567890ABCDEFGHIJKLMNOPQRST"
ANCHOR_NEW="GANEWANCHOR1234567890ABCDEFGHIJKLMNOPQRSTUVW"
ANCHOR_STRUGGLING="GASTRUGGLINGANCHOR1234567890ABCDEFGHIJKLMNO"

echo "Step 1: Cache metadata for three different anchors"
echo "---------------------------------------------------"

# Premium anchor: Excellent metrics
echo "Caching metadata for Premium Anchor..."
echo "  Uptime: 98% (9800)"
echo "  Reputation: 95% (9500)"
echo "  Settlement Time: 3 minutes (180s)"
# In practice: contract.cache_metadata(ANCHOR_PREMIUM, metadata, 3600)

# New anchor: Good tech, low reputation
echo ""
echo "Caching metadata for New Anchor..."
echo "  Uptime: 95% (9500)"
echo "  Reputation: 30% (3000)"
echo "  Settlement Time: 4 minutes (240s)"
# In practice: contract.cache_metadata(ANCHOR_NEW, metadata, 3600)

# Struggling anchor: Multiple issues
echo ""
echo "Caching metadata for Struggling Anchor..."
echo "  Uptime: 60% (6000)"
echo "  Reputation: 50% (5000)"
echo "  Settlement Time: 50 minutes (3000s)"
# In practice: contract.cache_metadata(ANCHOR_STRUGGLING, metadata, 3600)

echo ""
echo "Step 2: Get health scores"
echo "-------------------------"

# Calculate scores (simulated)
SCORE_PREMIUM=97
SCORE_NEW=71
SCORE_STRUGGLING=51

echo "Premium Anchor Score: $SCORE_PREMIUM"
echo "  Calculation: (40 × 98 + 35 × 95 + 25 × 100) / 100 = 97"
echo "  Assessment: EXCELLENT - Preferred choice"

echo ""
echo "New Anchor Score: $SCORE_NEW"
echo "  Calculation: (40 × 95 + 35 × 30 + 25 × 100) / 100 = 71"
echo "  Assessment: ACCEPTABLE - Monitor performance"

echo ""
echo "Struggling Anchor Score: $SCORE_STRUGGLING"
echo "  Calculation: (40 × 60 + 35 × 50 + 25 × 40) / 100 = 51"
echo "  Assessment: POOR - Use with caution"

echo ""
echo "Step 3: Select best anchor for transaction"
echo "-------------------------------------------"

if [ $SCORE_PREMIUM -ge 80 ]; then
    SELECTED="Premium Anchor"
    SELECTED_SCORE=$SCORE_PREMIUM
elif [ $SCORE_NEW -ge 80 ]; then
    SELECTED="New Anchor"
    SELECTED_SCORE=$SCORE_NEW
elif [ $SCORE_STRUGGLING -ge 80 ]; then
    SELECTED="Struggling Anchor"
    SELECTED_SCORE=$SCORE_STRUGGLING
else
    SELECTED="Premium Anchor (best available)"
    SELECTED_SCORE=$SCORE_PREMIUM
fi

echo "Selected: $SELECTED (Score: $SELECTED_SCORE)"

echo ""
echo "Step 4: Filter anchors by minimum score"
echo "----------------------------------------"

MIN_SCORE=70
echo "Minimum acceptable score: $MIN_SCORE"
echo ""

echo "Qualified anchors:"
[ $SCORE_PREMIUM -ge $MIN_SCORE ] && echo "  ✓ Premium Anchor ($SCORE_PREMIUM)"
[ $SCORE_NEW -ge $MIN_SCORE ] && echo "  ✓ New Anchor ($SCORE_NEW)"
[ $SCORE_STRUGGLING -ge $MIN_SCORE ] && echo "  ✓ Struggling Anchor ($SCORE_STRUGGLING)"

echo ""
echo "Disqualified anchors:"
[ $SCORE_PREMIUM -lt $MIN_SCORE ] && echo "  ✗ Premium Anchor ($SCORE_PREMIUM)"
[ $SCORE_NEW -lt $MIN_SCORE ] && echo "  ✗ New Anchor ($SCORE_NEW)"
[ $SCORE_STRUGGLING -lt $MIN_SCORE ] && echo "  ✗ Struggling Anchor ($SCORE_STRUGGLING)"

echo ""
echo "Step 5: Handle cache errors"
echo "----------------------------"

echo "Attempting to get score for uncached anchor..."
echo "Error: CacheNotFound (49)"
echo "Action: Cache metadata first"

echo ""
echo "Attempting to get score for expired cache..."
echo "Error: CacheExpired (48)"
echo "Action: Refresh metadata cache"

echo ""
echo "=== Example Complete ==="
echo ""
echo "Key Takeaways:"
echo "1. Health score simplifies anchor evaluation (single 0-100 metric)"
echo "2. Combines uptime (40%), reputation (35%), and speed (25%)"
echo "3. Scores ≥80 indicate high-quality anchors"
echo "4. Scores 60-79 are acceptable with monitoring"
echo "5. Scores <60 suggest caution or alternative anchors"
echo ""
echo "For more information:"
echo "  - Feature docs: docs/features/ANCHOR_HEALTH_SCORE.md"
echo "  - Quick reference: docs/guides/HEALTH_SCORE_QUICK_REF.md"
