#!/bin/bash
# Test service name validation in anchorkit register

echo "=== Testing anchorkit register service name validation ==="
echo ""

ADDR="GANCHOR123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"

# Test 1: Valid services
echo "Test 1: Valid services (deposits,withdrawals,kyc)"
cargo run --bin anchorkit -- register --address "$ADDR" --services "deposits,withdrawals,kyc"
RESULT1=$?
echo ""

# Test 2: Invalid service name
echo "Test 2: Invalid service name 'invalid_service' (should fail)"
OUTPUT=$(cargo run --bin anchorkit -- register --address "$ADDR" --services "invalid_service" 2>&1)
EXIT2=$?
echo "$OUTPUT"
if [ $EXIT2 -ne 0 ] && echo "$OUTPUT" | grep -q "unknown service"; then
    echo "✅ Correctly rejected invalid service with error message"
    RESULT2=0
else
    echo "❌ Expected non-zero exit and 'unknown service' message"
    RESULT2=1
fi
echo ""

# Test 3: Mix of valid and invalid services
echo "Test 3: Mix of valid and invalid services (should fail)"
OUTPUT=$(cargo run --bin anchorkit -- register --address "$ADDR" --services "deposits,foobar,kyc" 2>&1)
EXIT3=$?
echo "$OUTPUT"
if [ $EXIT3 -ne 0 ] && echo "$OUTPUT" | grep -q "foobar"; then
    echo "✅ Correctly listed invalid service name in error"
    RESULT3=0
else
    echo "❌ Expected non-zero exit and 'foobar' in error message"
    RESULT3=1
fi
echo ""

# Test 4: Error message lists valid services
echo "Test 4: Error message should list valid services"
OUTPUT=$(cargo run --bin anchorkit -- register --address "$ADDR" --services "bad_service" 2>&1)
echo "$OUTPUT"
if echo "$OUTPUT" | grep -q "deposits" && echo "$OUTPUT" | grep -q "withdrawals"; then
    echo "✅ Error message lists valid services"
    RESULT4=0
else
    echo "❌ Error message does not list valid services"
    RESULT4=1
fi
echo ""

# Test 5: No services flag (should succeed)
echo "Test 5: No --services flag (should succeed)"
cargo run --bin anchorkit -- register --address "$ADDR"
RESULT5=$?
echo ""

echo "=== Test Summary ==="
echo "Test 1 (valid services):          exit $RESULT1 (expected: 0)"
echo "Test 2 (invalid service):         exit $RESULT2 (expected: 0)"
echo "Test 3 (mixed valid/invalid):     exit $RESULT3 (expected: 0)"
echo "Test 4 (error lists valid names): exit $RESULT4 (expected: 0)"
echo "Test 5 (no services):             exit $RESULT5 (expected: 0)"

if [ $RESULT1 -eq 0 ] && [ $RESULT2 -eq 0 ] && [ $RESULT3 -eq 0 ] && [ $RESULT4 -eq 0 ] && [ $RESULT5 -eq 0 ]; then
    echo "✅ All service name validation tests passed!"
    exit 0
else
    echo "❌ Some tests failed"
    exit 1
fi
