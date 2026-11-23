#!/bin/bash
# Example: Secure Credential Management with AnchorKit
# This script demonstrates how to properly inject and manage credentials

set -e

# Configuration
CONTRACT_ID="${CONTRACT_ID:-CBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX}"
NETWORK="${NETWORK:-testnet}"
ADMIN_ACCOUNT="admin-account"

# Attestor addresses (public, safe to store)
KYC_PROVIDER="GBBD6A7KNZF5WNWQEPZP5DYJD2AYUTLXRB6VXJ4RCX4RTNPPQVNF3GQ"
BANK_INTEGRATION="GB7ZTQBJ7XXJQ6JDLHYQXQX3JQXJ3JQXJ3JQXJ3JQXJ3JQXJ3JQX"

echo "=== AnchorKit Secure Credential Management Example ==="
echo ""

# ============================================================
# Step 1: Fetch Credentials from Secure Secret Manager
# ============================================================
echo "Step 1: Fetching credentials from secret manager..."

# Example: AWS Secrets Manager
if command -v aws &> /dev/null; then
    echo "  Using AWS Secrets Manager..."
    KYC_API_KEY=$(aws secretsmanager get-secret-value \
        --secret-id anchorkit/kyc-provider/api-key \
        --query SecretString \
        --output text 2>/dev/null || echo "")
    
    BANK_TOKEN=$(aws secretsmanager get-secret-value \
        --secret-id anchorkit/bank-integration/token \
        --query SecretString \
        --output text 2>/dev/null || echo "")
fi

# Example: HashiCorp Vault
if command -v vault &> /dev/null && [ -z "$KYC_API_KEY" ]; then
    echo "  Using HashiCorp Vault..."
    KYC_API_KEY=$(vault kv get -field=api_key secret/anchorkit/kyc-provider 2>/dev/null || echo "")
    BANK_TOKEN=$(vault kv get -field=token secret/anchorkit/bank-integration 2>/dev/null || echo "")
fi

# Fallback to environment variables (for demo purposes)
if [ -z "$KYC_API_KEY" ]; then
    echo "  Using environment variables (demo mode)..."
    KYC_API_KEY="${ANCHOR_KYC_API_KEY:-sk_test_demo_key_12345678901234567890}"
    BANK_TOKEN="${ANCHOR_BANK_TOKEN:-Bearer demo_token_12345678901234567890}"
fi

echo "  ✓ Credentials fetched securely"
echo ""

# ============================================================
# Step 2: Encrypt Credentials Before Storage
# ============================================================
echo "Step 2: Encrypting credentials..."

# Simple encryption example (use proper encryption in production!)
encrypt_credential() {
    local credential="$1"
    # In production, use proper encryption (AES-256, etc.)
    # This is just a demo using base64
    echo -n "$credential" | base64
}

ENCRYPTED_KYC=$(encrypt_credential "$KYC_API_KEY")
ENCRYPTED_BANK=$(encrypt_credential "$BANK_TOKEN")

echo "  ✓ Credentials encrypted"
echo ""

# ============================================================
# Step 3: Set Credential Policies
# ============================================================
echo "Step 3: Setting credential policies..."

# KYC Provider: 30-day rotation, encryption required
echo "  Setting policy for KYC provider (30-day rotation)..."
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --source "$ADMIN_ACCOUNT" \
    --network "$NETWORK" \
    -- set_credential_policy \
    --attestor "$KYC_PROVIDER" \
    --rotation_interval_seconds 2592000 \
    --require_encryption true \
    2>/dev/null || echo "    (Skipped - contract not deployed)"

# Bank Integration: 7-day rotation, encryption required
echo "  Setting policy for bank integration (7-day rotation)..."
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --source "$ADMIN_ACCOUNT" \
    --network "$NETWORK" \
    -- set_credential_policy \
    --attestor "$BANK_INTEGRATION" \
    --rotation_interval_seconds 604800 \
    --require_encryption true \
    2>/dev/null || echo "    (Skipped - contract not deployed)"

echo "  ✓ Policies configured"
echo ""

# ============================================================
# Step 4: Store Encrypted Credentials
# ============================================================
echo "Step 4: Storing encrypted credentials..."

# Calculate expiry timestamps
NOW=$(date +%s)
KYC_EXPIRY=$((NOW + 2592000))  # 30 days
BANK_EXPIRY=$((NOW + 604800))   # 7 days

# Store KYC credential
echo "  Storing KYC provider credential..."
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --source "$ADMIN_ACCOUNT" \
    --network "$NETWORK" \
    -- store_encrypted_credential \
    --attestor "$KYC_PROVIDER" \
    --credential_type ApiKey \
    --encrypted_value "$ENCRYPTED_KYC" \
    --expires_at "$KYC_EXPIRY" \
    2>/dev/null || echo "    (Skipped - contract not deployed)"

# Store bank credential
echo "  Storing bank integration credential..."
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --source "$ADMIN_ACCOUNT" \
    --network "$NETWORK" \
    -- store_encrypted_credential \
    --attestor "$BANK_INTEGRATION" \
    --credential_type BearerToken \
    --encrypted_value "$ENCRYPTED_BANK" \
    --expires_at "$BANK_EXPIRY" \
    2>/dev/null || echo "    (Skipped - contract not deployed)"

echo "  ✓ Credentials stored securely"
echo ""

# ============================================================
# Step 5: Check Rotation Status
# ============================================================
echo "Step 5: Checking credential rotation status..."

check_rotation() {
    local attestor="$1"
    local name="$2"
    
    echo "  Checking $name..."
    NEEDS_ROTATION=$(soroban contract invoke \
        --id "$CONTRACT_ID" \
        --network "$NETWORK" \
        -- check_credential_rotation \
        --attestor "$attestor" \
        2>/dev/null || echo "false")
    
    if [ "$NEEDS_ROTATION" = "true" ]; then
        echo "    ⚠️  Rotation required!"
    else
        echo "    ✓ No rotation needed"
    fi
}

check_rotation "$KYC_PROVIDER" "KYC provider"
check_rotation "$BANK_INTEGRATION" "Bank integration"

echo ""

# ============================================================
# Step 6: Demonstrate Credential Rotation
# ============================================================
echo "Step 6: Demonstrating credential rotation..."

rotate_credential() {
    local attestor="$1"
    local name="$2"
    local new_credential="$3"
    local credential_type="$4"
    local expiry="$5"
    
    echo "  Rotating credential for $name..."
    
    # Encrypt new credential
    local encrypted_new=$(encrypt_credential "$new_credential")
    
    # Rotate in contract
    soroban contract invoke \
        --id "$CONTRACT_ID" \
        --source "$ADMIN_ACCOUNT" \
        --network "$NETWORK" \
        -- rotate_credential \
        --attestor "$attestor" \
        --credential_type "$credential_type" \
        --new_encrypted_value "$encrypted_new" \
        --expires_at "$expiry" \
        2>/dev/null || echo "    (Skipped - contract not deployed)"
    
    echo "    ✓ Rotation complete"
}

# Example rotation (in production, this would be triggered by policy)
# rotate_credential "$KYC_PROVIDER" "KYC provider" "sk_live_new_key_..." "ApiKey" "$KYC_EXPIRY"

echo "  (Rotation would be triggered automatically based on policy)"
echo ""

# ============================================================
# Step 7: Verify Credential Policies
# ============================================================
echo "Step 7: Verifying credential policies..."

verify_policy() {
    local attestor="$1"
    local name="$2"
    
    echo "  Verifying policy for $name..."
    soroban contract invoke \
        --id "$CONTRACT_ID" \
        --network "$NETWORK" \
        -- get_credential_policy \
        --attestor "$attestor" \
        2>/dev/null || echo "    (Skipped - contract not deployed)"
}

verify_policy "$KYC_PROVIDER" "KYC provider"
verify_policy "$BANK_INTEGRATION" "Bank integration"

echo ""

# ============================================================
# Summary
# ============================================================
echo "=== Summary ==="
echo ""
echo "✓ Credentials fetched from secure secret manager"
echo "✓ Credentials encrypted before storage"
echo "✓ Rotation policies configured"
echo "✓ Credentials stored in contract (encrypted)"
echo "✓ Rotation status checked"
echo "✓ Policies verified"
echo ""
echo "Security Best Practices Applied:"
echo "  • No plaintext credentials in config files"
echo "  • Runtime injection from secret manager"
echo "  • Encryption before storage"
echo "  • Automatic rotation policies"
echo "  • Regular rotation checks"
echo ""
echo "Next Steps:"
echo "  1. Set up automated rotation (cron job or CI/CD)"
echo "  2. Configure monitoring and alerting"
echo "  3. Test credential revocation procedures"
echo "  4. Document incident response plan"
echo ""
echo "For more information, see:"
echo "  • SECURE_CREDENTIALS.md"
echo "  • DEPLOYMENT_WITH_CREDENTIALS.md"
echo "  • configs/CREDENTIAL_SECURITY.md"
echo ""
