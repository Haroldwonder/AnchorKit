#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "🔍 AnchorKit Pre-Deployment Validation"
echo "========================================"
echo ""

# Check if Python is available
if ! command -v python3 &> /dev/null; then
    echo "❌ Python3 is required but not installed"
    exit 1
fi

# Check if required Python packages are installed
echo "📦 Checking Python dependencies..."
python3 -c "import jsonschema, toml" 2>/dev/null || {
    echo "❌ Missing Python dependencies. Installing..."
    pip3 install jsonschema toml --quiet
}
echo "✅ Python dependencies OK"
echo ""

# Validate all configuration files
echo "📋 Validating configuration files..."
CONFIG_DIR="$ROOT_DIR/configs"
SCHEMA_FILE="$ROOT_DIR/config_schema.json"
FAILED=0

if [ ! -f "$SCHEMA_FILE" ]; then
    echo "❌ Schema file not found: $SCHEMA_FILE"
    exit 1
fi

for config_file in "$CONFIG_DIR"/*.json "$CONFIG_DIR"/*.toml; do
    if [ -f "$config_file" ]; then
        echo -n "  Validating $(basename "$config_file")... "
        if python3 "$SCRIPT_DIR/validate_config_strict.py" "$config_file" "$SCHEMA_FILE" > /dev/null 2>&1; then
            echo "✅"
        else
            echo "❌"
            python3 "$SCRIPT_DIR/validate_config_strict.py" "$config_file" "$SCHEMA_FILE"
            FAILED=1
        fi
    fi
done

if [ $FAILED -eq 1 ]; then
    echo ""
    echo "❌ Configuration validation failed"
    exit 1
fi

echo ""
echo "✅ All configurations valid"
echo ""

# Run Rust tests
echo "🧪 Running Rust validation tests..."
if cargo test --quiet config 2>&1 | grep -q "test result: ok"; then
    echo "✅ Rust tests passed"
else
    echo "❌ Rust tests failed"
    cargo test config
    exit 1
fi

echo ""
echo "🎉 All validations passed! Ready for deployment."
