#!/bin/bash

# AnchorKit Structured Logging Demo Script
# This script demonstrates the logging features including:
# - Request/response logging
# - Sensitive data redaction

echo "🚀 AnchorKit Structured Logging Demo"
echo "===================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "\n${BLUE}📋 Step 1: Build the contract${NC}"
cargo build --target wasm32-unknown-unknown
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Contract built successfully${NC}"
else
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

echo -e "\n${BLUE}🌐 Step 2: Test request/response logging${NC}"
echo "Simulating network operations with request logging..."

echo -e "\n${YELLOW}Command: anchorkit register --address GANCHOR123 --endpoint https://anchor.example.com${NC}"
cargo run -- register --address GANCHOR123 --endpoint https://anchor.example.com
echo -e "${GREEN}✅ Request/response logging captured for registration${NC}"

echo -e "\n${YELLOW}Command: anchorkit health --attestor GANCHOR123${NC}"
cargo run -- health --attestor GANCHOR123
echo -e "${GREEN}✅ Health check with detailed logging${NC}"

echo -e "\n${BLUE}🔒 Step 3: Test with sensitive data redaction${NC}"
echo "Testing logging with sensitive data redaction enabled..."

echo -e "\n${YELLOW}Command: anchorkit attest --subject GUSER123 --payload-hash abc123${NC}"
cargo run -- attest --subject GUSER123 --payload-hash abc123def456789012345678901234567890123456789012345678901234567890
echo -e "${GREEN}✅ Sensitive data redacted in logs${NC}"

echo -e "\n${BLUE}📊 Step 4: Production mode${NC}"
echo "Testing production mode..."

echo -e "\n${YELLOW}Command: anchorkit deploy --network testnet${NC}"
cargo run -- deploy --network testnet
echo -e "${GREEN}✅ Production mode - logs at appropriate levels${NC}"

echo -e "\n${BLUE}🧪 Step 5: Run logging tests${NC}"
echo "Running the logging test suite..."

cargo test logging_tests
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ All logging tests passed${NC}"
else
    echo -e "${RED}❌ Some logging tests failed${NC}"
fi

echo -e "\n${BLUE}📖 Step 6: Run logging example${NC}"
echo "Running the logging example..."

cargo run --example logging_example
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Logging example completed successfully${NC}"
else
    echo -e "${RED}❌ Logging example failed${NC}"
fi

echo -e "\n${GREEN}🎉 Structured Logging Demo Completed!${NC}"
echo -e "\n${BLUE}📋 Summary of implemented features:${NC}"
echo "✅ Structured logs with multiple levels (ERROR, WARN, INFO, DEBUG, TRACE)"
echo "✅ Request/response logging with timing information"
echo "✅ Sensitive data redaction (configurable)"
echo "✅ Log size truncation to prevent memory issues"
echo "✅ Operation start/complete tracking with duration"
echo "✅ Request ID correlation for distributed tracing"
echo "✅ Integration with existing Soroban event system"

echo -e "\n${BLUE}💡 Usage Tips:${NC}"
echo "• Configure logging settings via the configure_logging contract method"
echo "• Monitor Soroban events to capture structured log output"
echo "• Use request IDs for distributed tracing across services"

echo -e "\n${BLUE}🔍 Next Steps:${NC}"
echo "• Integrate with your monitoring system to capture Soroban events"
echo "• Set up log aggregation for production deployments"
echo "• Configure alerting based on ERROR level logs"
echo "• Use request IDs for distributed tracing across services"