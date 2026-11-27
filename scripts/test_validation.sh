#!/bin/bash
# Test script for enhanced validation

set -e

echo "=== Testing Enhanced Configuration Validation ==="
echo ""

# Build the binary first
echo "Building anchorkit binary..."
cargo build --release
if [ $? -ne 0 ]; then
    echo "❌ Failed to build anchorkit binary"
    exit 1
fi
echo "✅ Build successful"
echo ""

BINARY="./target/release/anchorkit"
if [ ! -f "$BINARY" ]; then
    echo "❌ Binary not found at $BINARY"
    exit 1
fi

TESTS_PASSED=0
TESTS_FAILED=0

# Test 1: Testing valid configuration
echo "1. Testing valid configuration..."
OUTPUT=$($BINARY validate configs/testnet-example.json 2>&1)
RESULT=$?
if [ $RESULT -eq 0 ]; then
    echo "✅ Test 1 passed (exit code: 0)"
    ((TESTS_PASSED++))
else
    echo "❌ Test 1 failed (exit code: $RESULT)"
    echo "Output: $OUTPUT"
    ((TESTS_FAILED++))
fi
echo ""

# Test 2: Testing invalid configuration (should fail)
echo "2. Testing invalid configuration (missing fields)..."
OUTPUT=$($BINARY validate tests/fixtures/invalid_configs/test-invalid.json 2>&1)
RESULT=$?
if [ $RESULT -ne 0 ]; then
    echo "✅ Test 2 passed (exit code: $RESULT, expected non-zero)"
    ((TESTS_PASSED++))
else
    echo "❌ Test 2 failed (expected non-zero exit code, got 0)"
    echo "Output: $OUTPUT"
    ((TESTS_FAILED++))
fi
echo ""

# Test 3: Testing all configs in directory
echo "3. Testing all configs in directory..."
OUTPUT=$($BINARY validate configs/ 2>&1)
RESULT=$?
if [ $RESULT -eq 0 ]; then
    echo "✅ Test 3 passed (exit code: 0)"
    ((TESTS_PASSED++))
else
    echo "❌ Test 3 failed (exit code: $RESULT)"
    echo "Output: $OUTPUT"
    ((TESTS_FAILED++))
fi
echo ""

# Summary
echo "=== Test Summary ==="
echo "Tests passed: $TESTS_PASSED"
echo "Tests failed: $TESTS_FAILED"

if [ $TESTS_FAILED -eq 0 ]; then
    echo "✅ All tests passed!"
    exit 0
else
    echo "❌ Some tests failed"
    exit 1
fi
