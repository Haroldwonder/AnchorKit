# PowerShell equivalent of validate_all.sh
# AnchorKit Pre-Deployment Validation for Windows

$ErrorActionPreference = "Stop"

Write-Output "🔍 AnchorKit Pre-Deployment Validation"
Write-Output "========================================"
Write-Output ""

# Check if Python is available
try {
    $pythonVersion = python --version 2>&1
    Write-Output "✅ Python found: $pythonVersion"
} catch {
    Write-Error "❌ Python3 is required but not installed. Download from: https://www.python.org/downloads/"
    exit 1
}

# Check if required Python packages are installed
Write-Output "📦 Checking Python dependencies..."
try {
    python -c "import jsonschema, toml" 2>$null
    Write-Output "✅ Python dependencies OK"
} catch {
    Write-Output "⚠️  Missing Python dependencies. Installing..."
    pip install jsonschema toml --quiet
    Write-Output "✅ Dependencies installed"
}
Write-Output ""

# Validate all configuration files
Write-Output "📋 Validating configuration files..."
$ConfigDir = "configs"
$SchemaFile = "config_schema.json"
$Failed = 0

if (-not (Test-Path $SchemaFile)) {
    Write-Error "❌ Schema file not found: $SchemaFile"
    exit 1
}

$configFiles = Get-ChildItem -Path $ConfigDir -Include *.json,*.toml -File

foreach ($configFile in $configFiles) {
    Write-Output "  Validating $($configFile.Name)..."

    $result = python validate_config_strict.py $configFile.FullName $SchemaFile 2>&1

    if ($LASTEXITCODE -eq 0) {
        Write-Output "  ✅ $($configFile.Name)"
    } else {
        Write-Error "  ❌ $($configFile.Name): $result"
        $Failed = 1
    }
}

if ($Failed -eq 1) {
    Write-Output ""
    Write-Error "❌ Configuration validation failed"
    exit 1
}

Write-Output ""
Write-Output "✅ All configurations valid"
Write-Output ""

# Run Rust tests
Write-Output "🧪 Running Rust validation tests..."
$testOutput = cargo test --quiet config 2>&1 | Out-String

if ($testOutput -match "test result: ok") {
    Write-Output "✅ Rust tests passed"
} else {
    Write-Error "❌ Rust tests failed"
    cargo test config
    exit 1
}

Write-Output ""
Write-Output "🎉 All validations passed! Ready for deployment."
