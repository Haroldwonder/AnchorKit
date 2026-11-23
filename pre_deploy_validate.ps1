# PowerShell equivalent of pre_deploy_validate.sh
# Pre-deployment validation script for Windows
# Validates all configurations before contract deployment

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$SchemaFile = Join-Path $ScriptDir "config_schema.json"
$Validator = Join-Path $ScriptDir "validate_config_strict.py"
$ConfigsDir = Join-Path $ScriptDir "configs"

Write-Output "🔍 AnchorKit Pre-Deployment Validation"
Write-Output "========================================"
Write-Output ""

# Check dependencies
try {
    $null = python --version 2>&1
} catch {
    Write-Error "❌ Python3 not found. Please install Python 3.7+ from: https://www.python.org/downloads/"
    exit 1
}

# Install required Python packages
Write-Output "📦 Checking Python dependencies..."
try {
    pip install -q jsonschema toml 2>$null
} catch {
    Write-Output "⚠️  Installing jsonschema and toml..."
    pip install jsonschema toml
}

# Validate schema file exists
if (-not (Test-Path $SchemaFile)) {
    Write-Error "❌ Schema file not found: $SchemaFile"
    exit 1
}

# Validate all config files
Write-Output ""
Write-Output "🔎 Validating configuration files..."
Write-Output ""

$Failed = 0
$Passed = 0

$configFiles = Get-ChildItem -Path $ConfigsDir -Include *.toml,*.json -File

foreach ($config in $configFiles) {
    $output = python $Validator $config.FullName $SchemaFile 2>&1

    if ($LASTEXITCODE -eq 0) {
        Write-Output "  ✓ $($config.Name)"
        $Passed++
    } else {
        Write-Error "  ✗ $($config.Name): $output"
        $Failed++
    }
}

Write-Output ""
Write-Output "========================================"
Write-Output "Results: $Passed passed, $Failed failed"
Write-Output ""

if ($Failed -gt 0) {
    Write-Error "❌ Validation failed. Fix errors before deployment."
    exit 1
} else {
    Write-Output "✅ All configurations valid. Ready for deployment."
    exit 0
}
