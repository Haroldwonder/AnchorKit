#!/bin/bash
# Test TOML validation in anchorkit validate

set -e

echo "=== Testing anchorkit validate (TOML support) ==="
echo ""

TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

# Test 1: Valid TOML file
cat > "$TMPDIR/valid.toml" << 'EOF'
[contract]
name = "test-anchor"
version = "1.0.0"
network = "stellar-testnet"

[attestors]
threshold = 1

[[attestors.members]]
name = "attestor-1"
public_key = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"

[sessions]
timeout_seconds = 3600

[deployment]
environment = "testnet"
EOF

echo "Test 1: Valid TOML file"
cargo run --bin anchorkit -- validate "$TMPDIR/valid.toml"
RESULT1=$?
echo ""

# Test 2: Invalid TOML file (syntax error)
cat > "$TMPDIR/invalid.toml" << 'EOF'
[contract]
name = "test-anchor"
version = 1.0.0  # missing quotes — invalid TOML
EOF

echo "Test 2: Invalid TOML file (should fail with line number)"
cargo run --bin anchorkit -- validate "$TMPDIR/invalid.toml" && RESULT2=1 || RESULT2=0
echo ""

# Test 3: Valid JSON file (existing behavior preserved)
cat > "$TMPDIR/valid.json" << 'EOF'
{
  "contract": {
    "name": "test-anchor",
    "version": "1.0.0",
    "network": "stellar-testnet"
  },
  "attestors": {
    "threshold": 1,
    "members": [
      {
        "name": "attestor-1",
        "public_key": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"
      }
    ]
  },
  "sessions": {
    "timeout_seconds": 3600
  },
  "deployment": {
    "environment": "testnet"
  }
}
EOF

echo "Test 3: Valid JSON file"
cargo run --bin anchorkit -- validate "$TMPDIR/valid.json"
RESULT3=$?
echo ""

# Test 4: Mixed directory (both .toml and .json)
echo "Test 4: Validate configs/ directory (mixed JSON + TOML)"
cargo run --bin anchorkit -- validate configs
RESULT4=$?
echo ""

echo "=== Test Summary ==="
echo "Test 1 (valid TOML):    exit $RESULT1 (expected: 0)"
echo "Test 2 (invalid TOML):  exit $RESULT2 (expected: 0, meaning validate returned 1)"
echo "Test 3 (valid JSON):    exit $RESULT3 (expected: 0)"
echo "Test 4 (configs/ dir):  exit $RESULT4 (expected: 0)"

if [ $RESULT1 -eq 0 ] && [ $RESULT2 -eq 0 ] && [ $RESULT3 -eq 0 ] && [ $RESULT4 -eq 0 ]; then
    echo "✅ All TOML validation tests passed!"
    exit 0
else
    echo "❌ Some tests failed"
    exit 1
fi
