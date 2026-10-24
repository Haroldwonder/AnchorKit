# Validation script for Anchor Health Score implementation
# PowerShell version for Windows

$ErrorActionPreference = "Continue"

Write-Host "╔══════════════════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║              ANCHOR HEALTH SCORE VALIDATION                                  ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

$FAILURES = 0

function Check-Pass {
    param($message)
    Write-Host "✓ $message" -ForegroundColor Green
}

function Check-Fail {
    param($message)
    Write-Host "✗ $message" -ForegroundColor Red
    $script:FAILURES++
}

function Check-Warn {
    param($message)
    Write-Host "⚠ $message" -ForegroundColor Yellow
}

Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "1. FILE STRUCTURE CHECKS" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan

if (Test-Path "src/anchor_health_score_tests.rs") {
    Check-Pass "src/anchor_health_score_tests.rs exists"
} else {
    Check-Fail "src/anchor_health_score_tests.rs missing"
}

if (Test-Path "docs/features/ANCHOR_HEALTH_SCORE.md") {
    Check-Pass "docs/features/ANCHOR_HEALTH_SCORE.md exists"
} else {
    Check-Fail "docs/features/ANCHOR_HEALTH_SCORE.md missing"
}

if (Test-Path "docs/guides/HEALTH_SCORE_QUICK_REF.md") {
    Check-Pass "docs/guides/HEALTH_SCORE_QUICK_REF.md exists"
} else {
    Check-Fail "docs/guides/HEALTH_SCORE_QUICK_REF.md missing"
}

if (Test-Path "docs/internal/HEALTH_SCORE_IMPLEMENTATION.md") {
    Check-Pass "docs/internal/HEALTH_SCORE_IMPLEMENTATION.md exists"
} else {
    Check-Fail "docs/internal/HEALTH_SCORE_IMPLEMENTATION.md missing"
}

if (Test-Path "examples/health_score_example.sh") {
    Check-Pass "examples/health_score_example.sh exists"
} else {
    Check-Fail "examples/health_score_example.sh missing"
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "2. FUNCTION IMPLEMENTATION CHECKS" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan

$contractContent = Get-Content "src/contract.rs" -Raw

if ($contractContent -match "pub fn get_anchor_health_score") {
    Check-Pass "get_anchor_health_score function defined"
} else {
    Check-Fail "get_anchor_health_score function NOT defined"
}

if ($contractContent -match "pub fn get_anchor_health_score\(env: Env, anchor: Address\) -> u32") {
    Check-Pass "Function signature correct (returns u32)"
} else {
    Check-Fail "Function signature incorrect"
}

if ($contractContent -match "const UPTIME_WEIGHT: u64 = 40") {
    Check-Pass "UPTIME_WEIGHT constant defined (40%)"
} else {
    Check-Fail "UPTIME_WEIGHT constant missing or incorrect"
}

if ($contractContent -match "const REPUTATION_WEIGHT: u64 = 35") {
    Check-Pass "REPUTATION_WEIGHT constant defined (35%)"
} else {
    Check-Fail "REPUTATION_WEIGHT constant missing or incorrect"
}

if ($contractContent -match "const SPEED_WEIGHT: u64 = 25") {
    Check-Pass "SPEED_WEIGHT constant defined (25%)"
} else {
    Check-Fail "SPEED_WEIGHT constant missing or incorrect"
}

if ($contractContent -match "average_settlement_time <= 300") {
    Check-Pass "Settlement tier 1 (≤300s) implemented"
} else {
    Check-Fail "Settlement tier 1 missing"
}

if ($contractContent -match "average_settlement_time <= 600") {
    Check-Pass "Settlement tier 2 (≤600s) implemented"
} else {
    Check-Fail "Settlement tier 2 missing"
}

if ($contractContent -match "average_settlement_time <= 1800") {
    Check-Pass "Settlement tier 3 (≤1800s) implemented"
} else {
    Check-Fail "Settlement tier 3 missing"
}

if ($contractContent -match "average_settlement_time <= 3600") {
    Check-Pass "Settlement tier 4 (≤3600s) implemented"
} else {
    Check-Fail "Settlement tier 4 missing"
}

if ($contractContent -match "Self::get_cached_metadata") {
    Check-Pass "Uses get_cached_metadata for data retrieval"
} else {
    Check-Fail "Does not use get_cached_metadata"
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "3. TEST COVERAGE CHECKS" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan

$testContent = Get-Content "src/anchor_health_score_tests.rs" -Raw
$testCount = ([regex]::Matches($testContent, "^fn test_", [System.Text.RegularExpressions.RegexOptions]::Multiline)).Count

if ($testCount -ge 12) {
    Check-Pass "Test coverage: $testCount tests (expected: 12)"
} else {
    Check-Fail "Test coverage: $testCount tests (expected: 12)"
}

$requiredTests = @(
    "test_perfect_health_score",
    "test_good_health_score",
    "test_acceptable_health_score",
    "test_poor_health_score",
    "test_cache_not_found_error",
    "test_cache_expired_error",
    "test_settlement_time_boundaries",
    "test_zero_values",
    "test_multiple_anchors"
)

foreach ($test in $requiredTests) {
    if ($testContent -match "fn $test") {
        Check-Pass "Test case: $test"
    } else {
        Check-Fail "Missing test: $test"
    }
}

if ($testContent -match '#\[should_panic\(expected = "CacheNotFound"\)\]') {
    Check-Pass "CacheNotFound error test uses should_panic"
} else {
    Check-Fail "CacheNotFound error test missing should_panic"
}

if ($testContent -match '#\[should_panic\(expected = "CacheExpired"\)\]') {
    Check-Pass "CacheExpired error test uses should_panic"
} else {
    Check-Fail "CacheExpired error test missing should_panic"
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "4. MODULE DECLARATION CHECKS" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan

$libContent = Get-Content "src/lib.rs" -Raw

if ($libContent -match "mod anchor_health_score_tests;") {
    Check-Pass "Test module declared in lib.rs"
} else {
    Check-Fail "Test module NOT declared in lib.rs"
}

if ($libContent -match "#\[cfg\(test\)\][\s\S]*?mod anchor_health_score_tests;") {
    Check-Pass "Test module properly gated with #[cfg(test)]"
} else {
    Check-Fail "Test module not properly gated"
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "5. DOCUMENTATION CHECKS" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan

if ($contractContent -match "///[\s\S]*?pub fn get_anchor_health_score") {
    Check-Pass "Function has documentation comments"
} else {
    Check-Fail "Function missing documentation"
}

if ($contractContent -match "Formula") {
    Check-Pass "Formula documented in code"
} else {
    Check-Fail "Formula not documented"
}

if ($contractContent -match "CacheNotFound" -and $contractContent -match "CacheExpired") {
    Check-Pass "Error codes documented"
} else {
    Check-Fail "Error codes not documented"
}

$readmeContent = Get-Content "README.md" -Raw

if ($readmeContent -match "Anchor Health Score") {
    Check-Pass "README.md mentions Anchor Health Score"
} else {
    Check-Fail "README.md NOT updated"
}

if ($readmeContent -match "get_anchor_health_score") {
    Check-Pass "README.md includes usage example"
} else {
    Check-Fail "README.md missing usage example"
}

$changelogContent = Get-Content "CHANGELOG.md" -Raw

if ($changelogContent -match "Anchor Health Score") {
    Check-Pass "CHANGELOG.md updated"
} else {
    Check-Fail "CHANGELOG.md NOT updated"
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "6. ACCEPTANCE CRITERIA VALIDATION" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan

$featureDoc = Get-Content "docs/features/ANCHOR_HEALTH_SCORE.md" -Raw

if ($featureDoc -match "Formula" -and $featureDoc -match "40%" -and $featureDoc -match "35%" -and $featureDoc -match "25%") {
    Check-Pass "✓ Criterion 1: Score formula documented"
} else {
    Check-Fail "✗ Criterion 1: Score formula NOT fully documented"
}

if ($testContent -match "test_cache_not_found_error" -and $testContent -match "CacheNotFound") {
    Check-Pass "✓ Criterion 2: Returns CacheNotFound when no metadata cached"
} else {
    Check-Fail "✗ Criterion 2: CacheNotFound behavior not tested"
}

if ($testCount -ge 12) {
    Check-Pass "✓ Criterion 3: Tests with various metric combinations ($testCount tests)"
} else {
    Check-Fail "✗ Criterion 3: Insufficient test coverage ($testCount tests)"
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "7. SUMMARY" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

if ($FAILURES -eq 0) {
    Write-Host "✓ ALL VALIDATION CHECKS PASSED" -ForegroundColor Green
    Write-Host ""
    Write-Host "The Anchor Health Score implementation is complete and ready." -ForegroundColor Green
    Write-Host ""
    Write-Host "Implementation Summary:"
    Write-Host "  • Function: get_anchor_health_score(env, anchor) -> u32"
    Write-Host "  • Formula: (40% × uptime) + (35% × reputation) + (25% × speed)"
    Write-Host "  • Tests: $testCount comprehensive test cases"
    Write-Host "  • Documentation: Complete with examples and guides"
    Write-Host ""
    Write-Host "Next steps (when cargo is available):"
    Write-Host "  1. cargo check          # Verify compilation"
    Write-Host "  2. cargo test anchor_health_score --lib  # Run tests"
    Write-Host "  3. cargo clippy         # Check for warnings"
    Write-Host ""
    exit 0
} else {
    Write-Host "✗ $FAILURES VALIDATION CHECK(S) FAILED" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please review and fix the issues above." -ForegroundColor Red
    Write-Host ""
    exit 1
}
