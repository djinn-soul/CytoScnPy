# Quick test with coverage display
# Run this instead of 'cargo test' to see coverage

param(
    [string]$TestName = ""
)

Write-Host "🧪 Running tests with coverage..." -ForegroundColor Cyan

# Navigate to cytoscnpy
Set-Location cytoscnpy

if ($TestName) {
    # Run specific test with coverage
    cargo llvm-cov --test $TestName
} else {
    # Run all tests with coverage summary
    cargo llvm-cov --all-features
}

Set-Location ..
