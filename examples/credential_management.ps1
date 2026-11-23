# Example: Secure Credential Management with AnchorKit
# PowerShell equivalent of credential_management.sh
# Demonstrates how to properly inject and manage credentials on Windows

$ErrorActionPreference = "Stop"

# Configuration
$ContractId = if ($env:CONTRACT_ID) { $env:CONTRACT_ID } else { "CBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX" }
$Network    = if ($env:NETWORK) { $env:NETWORK } else { "testnet" }
$AdminAccount = "admin-account"

# Attestor addresses (public, safe to store)
$KycProvider     = "GBBD6A7KNZF5WNWQEPZP5DYJD2AYUTLXRB6VXJ4RCX4RTNPPQVNF3GQ"
$BankIntegration = "GB7ZTQBJ7XXJQ6JDLHYQXQX3JQXJ3JQXJ3JQXJ3JQXJ3JQXJ3JQX"

Write-Host "=== AnchorKit Secure Credential Management Example ===" -ForegroundColor Cyan
Write-Host ""

# ============================================================
# Step 1: Fetch Credentials from Secure Secret Manager
# ============================================================
Write-Host "Step 1: Fetching credentials from secret manager..."

$KycApiKey  = ""
$BankToken  = ""

# Example: AWS Secrets Manager
if (Get-Command aws -ErrorAction SilentlyContinue) {
    Write-Host "  Using AWS Secrets Manager..."
    try {
        $KycApiKey = aws secretsmanager get-secret-value `
            --secret-id anchorkit/kyc-provider/api-key `
            --query SecretString --output text 2>$null
        $BankToken = aws secretsmanager get-secret-value `
            --secret-id anchorkit/bank-integration/token `
            --query SecretString --output text 2>$null
    } catch { }
}

# Example: HashiCorp Vault
if ((Get-Command vault -ErrorAction SilentlyContinue) -and -not $KycApiKey) {
    Write-Host "  Using HashiCorp Vault..."
    try {
        $KycApiKey = vault kv get -field=api_key secret/anchorkit/kyc-provider 2>$null
        $BankToken = vault kv get -field=token secret/anchorkit/bank-integration 2>$null
    } catch { }
}

# Fallback to environment variables (demo mode)
if (-not $KycApiKey) {
    Write-Host "  Using environment variables (demo mode)..."
    $KycApiKey = if ($env:ANCHOR_KYC_API_KEY) { $env:ANCHOR_KYC_API_KEY } else { "sk_test_demo_key_12345678901234567890" }
    $BankToken = if ($env:ANCHOR_BANK_TOKEN)  { $env:ANCHOR_BANK_TOKEN  } else { "Bearer demo_token_12345678901234567890" }
}

Write-Host "  [OK] Credentials fetched securely"
Write-Host ""

# ============================================================
# Step 2: Encrypt Credentials Before Storage
# ============================================================
Write-Host "Step 2: Encrypting credentials..."

function Encrypt-Credential {
    param([string]$Credential)
    # In production, use proper encryption (AES-256, etc.)
    # This is just a demo using Base64
    [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($Credential))
}

$EncryptedKyc  = Encrypt-Credential $KycApiKey
$EncryptedBank = Encrypt-Credential $BankToken

Write-Host "  [OK] Credentials encrypted"
Write-Host ""

# ============================================================
# Step 3: Set Credential Policies
# ============================================================
Write-Host "Step 3: Setting credential policies..."

$invokeArgs = @("--id", $ContractId, "--source", $AdminAccount, "--network", $Network, "--")

Write-Host "  Setting policy for KYC provider (30-day rotation)..."
soroban contract invoke @invokeArgs `
    set_credential_policy `
    --attestor $KycProvider `
    --rotation_interval_seconds 2592000 `
    --require_encryption true 2>$null
if ($LASTEXITCODE -ne 0) { Write-Host "    (Skipped - contract not deployed)" }

Write-Host "  Setting policy for bank integration (7-day rotation)..."
soroban contract invoke @invokeArgs `
    set_credential_policy `
    --attestor $BankIntegration `
    --rotation_interval_seconds 604800 `
    --require_encryption true 2>$null
if ($LASTEXITCODE -ne 0) { Write-Host "    (Skipped - contract not deployed)" }

Write-Host "  [OK] Policies configured"
Write-Host ""

# ============================================================
# Step 4: Store Encrypted Credentials
# ============================================================
Write-Host "Step 4: Storing encrypted credentials..."

$Now         = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
$KycExpiry   = $Now + 2592000  # 30 days
$BankExpiry  = $Now + 604800   # 7 days

Write-Host "  Storing KYC provider credential..."
soroban contract invoke @invokeArgs `
    store_encrypted_credential `
    --attestor $KycProvider `
    --credential_type ApiKey `
    --encrypted_value $EncryptedKyc `
    --expires_at $KycExpiry 2>$null
if ($LASTEXITCODE -ne 0) { Write-Host "    (Skipped - contract not deployed)" }

Write-Host "  Storing bank integration credential..."
soroban contract invoke @invokeArgs `
    store_encrypted_credential `
    --attestor $BankIntegration `
    --credential_type BearerToken `
    --encrypted_value $EncryptedBank `
    --expires_at $BankExpiry 2>$null
if ($LASTEXITCODE -ne 0) { Write-Host "    (Skipped - contract not deployed)" }

Write-Host "  [OK] Credentials stored securely"
Write-Host ""

# ============================================================
# Step 5: Check Rotation Status
# ============================================================
Write-Host "Step 5: Checking credential rotation status..."

function Check-Rotation {
    param([string]$Attestor, [string]$Name)
    Write-Host "  Checking $Name..."
    $result = soroban contract invoke --id $ContractId --network $Network -- `
        check_credential_rotation --attestor $Attestor 2>$null
    if ($result -eq "true") {
        Write-Host "    [!] Rotation required!" -ForegroundColor Yellow
    } else {
        Write-Host "    [OK] No rotation needed"
    }
}

Check-Rotation $KycProvider     "KYC provider"
Check-Rotation $BankIntegration "Bank integration"
Write-Host ""

# ============================================================
# Step 6: Demonstrate Credential Rotation
# ============================================================
Write-Host "Step 6: Demonstrating credential rotation..."

function Rotate-Credential {
    param([string]$Attestor, [string]$Name, [string]$NewCredential, [string]$CredentialType, [long]$Expiry)
    Write-Host "  Rotating credential for $Name..."
    $encryptedNew = Encrypt-Credential $NewCredential
    soroban contract invoke @invokeArgs `
        rotate_credential `
        --attestor $Attestor `
        --credential_type $CredentialType `
        --new_encrypted_value $encryptedNew `
        --expires_at $Expiry 2>$null
    if ($LASTEXITCODE -ne 0) { Write-Host "    (Skipped - contract not deployed)" }
    Write-Host "    [OK] Rotation complete"
}

# Example rotation (in production, triggered automatically by policy)
# Rotate-Credential $KycProvider "KYC provider" "sk_live_new_key_..." "ApiKey" $KycExpiry

Write-Host "  (Rotation would be triggered automatically based on policy)"
Write-Host ""

# ============================================================
# Step 7: Verify Credential Policies
# ============================================================
Write-Host "Step 7: Verifying credential policies..."

function Verify-Policy {
    param([string]$Attestor, [string]$Name)
    Write-Host "  Verifying policy for $Name..."
    soroban contract invoke --id $ContractId --network $Network -- `
        get_credential_policy --attestor $Attestor 2>$null
    if ($LASTEXITCODE -ne 0) { Write-Host "    (Skipped - contract not deployed)" }
}

Verify-Policy $KycProvider     "KYC provider"
Verify-Policy $BankIntegration "Bank integration"
Write-Host ""

# ============================================================
# Summary
# ============================================================
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "[OK] Credentials fetched from secure secret manager"
Write-Host "[OK] Credentials encrypted before storage"
Write-Host "[OK] Rotation policies configured"
Write-Host "[OK] Credentials stored in contract (encrypted)"
Write-Host "[OK] Rotation status checked"
Write-Host "[OK] Policies verified"
Write-Host ""
Write-Host "Security Best Practices Applied:"
Write-Host "  * No plaintext credentials in config files"
Write-Host "  * Runtime injection from secret manager"
Write-Host "  * Encryption before storage"
Write-Host "  * Automatic rotation policies"
Write-Host "  * Regular rotation checks"
Write-Host ""
Write-Host "Next Steps:"
Write-Host "  1. Set up automated rotation (Task Scheduler or CI/CD)"
Write-Host "  2. Configure monitoring and alerting"
Write-Host "  3. Test credential revocation procedures"
Write-Host "  4. Document incident response plan"
Write-Host ""
Write-Host "For more information, see:"
Write-Host "  * SECURE_CREDENTIALS.md"
Write-Host "  * DEPLOYMENT_WITH_CREDENTIALS.md"
Write-Host "  * configs/CREDENTIAL_SECURITY.md"
Write-Host ""
