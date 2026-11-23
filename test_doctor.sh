#!/bin/bash

echo "=== Testing AnchorKit Doctor Command ==="
echo ""

echo "Test 1: Running doctor without environment variables"
echo "Expected: Some checks should fail"
echo "---"
cargo run --bin anchorkit -- doctor
RESULT1=$?
echo ""

echo "Test 2: Running doctor with environment variables set"
echo "Expected: All checks should pass"
echo "---"
STELLAR_SECRET_KEY=test_key \
ANCHORKIT_RPC_URL=https://soroban-testnet.stellar.org \
cargo run --bin anchorkit -- doctor
RESULT2=$?
echo ""

echo "Test 3: Rust version check output"
echo "Expected: Rust version line present and meets minimum (1.56+)"
echo "---"
OUTPUT=$(cargo run --bin anchorkit -- doctor 2>&1)
echo "$OUTPUT"
if echo "$OUTPUT" | grep -qE "✔ Rust toolchain detected|✖ Rust [0-9]+\.[0-9]+ detected but"; then
    echo "✅ Rust version check is present"
    RESULT3=0
else
    echo "❌ Rust version check output not found"
    RESULT3=1
fi
echo ""

echo "=== Test Summary ==="
echo "Test 1 exit code: $RESULT1 (expected: 1)"
echo "Test 2 exit code: $RESULT2 (expected: 0)"
echo "Test 3 exit code: $RESULT3 (expected: 0)"

if [ $RESULT1 -eq 1 ] && [ $RESULT2 -eq 0 ] && [ $RESULT3 -eq 0 ]; then
    echo "✅ All tests passed!"
    exit 0
else
    echo "❌ Some tests failed"
    exit 1
fi
