#!/bin/bash
# Pre-deployment validation script
# Validates all configurations before contract deployment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCHEMA_FILE="$SCRIPT_DIR/config_schema.json"
VALIDATOR="$SCRIPT_DIR/validate_config_strict.py"
CONFIGS_DIR="$SCRIPT_DIR/configs"

echo "🔍 AnchorKit Pre-Deployment Validation"
echo "========================================"
echo ""

# Check dependencies
if ! command -v python3 &> /dev/null; then
    echo "❌ Python3 not found. Please install Python 3.7+"
    exit 1
fi

# Install required Python packages
echo "📦 Checking Python dependencies..."
pip3 install -q jsonschema toml 2>/dev/null || {
    echo "⚠️  Installing jsonschema and toml..."
    pip3 install jsonschema toml
}

# Validate schema file exists
if [ ! -f "$SCHEMA_FILE" ]; then
    echo "❌ Schema file not found: $SCHEMA_FILE"
    exit 1
fi

# Validate all config files
echo ""
echo "🔎 Validating configuration files..."
echo ""

FAILED=0
PASSED=0

for config in "$CONFIGS_DIR"/*.toml "$CONFIGS_DIR"/*.json; do
    if [ -f "$config" ]; then
        echo -n "  Validating $(basename "$config")... "
        if python3 "$VALIDATOR" "$config" "$SCHEMA_FILE" > /dev/null 2>&1; then
            echo "✓"
            ((PASSED++))
        else
            echo "✗"
            python3 "$VALIDATOR" "$config" "$SCHEMA_FILE" 2>&1 | sed 's/^/    /'
            ((FAILED++))
        fi
    fi
done

echo ""
echo "========================================"
echo "Results: $PASSED passed, $FAILED failed"
echo ""

if [ $FAILED -gt 0 ]; then
    echo "❌ Validation failed. Fix errors before deployment."
    exit 1
else
    echo "✅ All configurations valid. Ready for deployment."
    exit 0
fi
