# Code Coverage Script for CytoScnPy
# Generates HTML and JSON coverage reports using cargo-llvm-cov

param(
    [switch]$Open,
    [switch]$Json,
    [switch]$Summary,
    [switch]$Install,
    [switch]$NoClean
)

Write-Host "🎯 CytoScnPy Code Coverage Tool" -ForegroundColor Cyan
Write-Host ""

# Check if cargo-llvm-cov is installed
$llvmCovInstalled = cargo llvm-cov --version 2>$null
if (-not $llvmCovInstalled -and -not $Install) {
    Write-Host "❌ cargo-llvm-cov is not installed!" -ForegroundColor Red
    Write-Host ""
    Write-Host "To install, run:" -ForegroundColor Yellow
    Write-Host "  .\scripts\coverage.ps1 -Install" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Or manually:" -ForegroundColor Yellow
    Write-Host "  cargo install cargo-llvm-cov" -ForegroundColor Yellow
    exit 1
}

# Install cargo-llvm-cov if requested
if ($Install) {
    Write-Host "📦 Installing cargo-llvm-cov..." -ForegroundColor Yellow
    cargo install cargo-llvm-cov
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ cargo-llvm-cov installed successfully!" -ForegroundColor Green
    } else {
        Write-Host "❌ Failed to install cargo-llvm-cov" -ForegroundColor Red
        exit 1
    }
    exit 0
}

# Navigate to cytoscnpy directory
Set-Location cytoscnpy

# Clean previous coverage data
if (-not $NoClean) {
    Write-Host "🧹 Cleaning previous coverage data..." -ForegroundColor Yellow
    cargo llvm-cov clean
} else {
    Write-Host "⏭️  Skipping clean step..." -ForegroundColor DarkGray
}

# Generate coverage
Write-Host "🔬 Running tests with coverage instrumentation..." -ForegroundColor Yellow
Write-Host ""

if ($Summary) {
    # Just show summary in terminal
    cargo llvm-cov --all-features
} elseif ($Json) {
    # Generate JSON report for CI/CD
    Write-Host "📊 Generating JSON coverage report..." -ForegroundColor Yellow
    cargo llvm-cov --all-features --json --output-path ../coverage.json
    Write-Host "✅ Coverage report saved to: coverage.json" -ForegroundColor Green
} else {
    # Generate HTML report
    Write-Host "📊 Generating HTML coverage report..." -ForegroundColor Yellow
    cargo llvm-cov --all-features --html

    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Host "✅ Coverage report generated successfully!" -ForegroundColor Green
        Write-Host ""
        Write-Host "📁 Report location: cytoscnpy\target\llvm-cov\html\index.html" -ForegroundColor Cyan

        if ($Open) {
            Write-Host "🌐 Opening coverage report in browser..." -ForegroundColor Yellow
            Start-Process (Resolve-Path "..\target\llvm-cov\html\index.html")
        } else {
            Write-Host ""
            Write-Host "To open the report, run:" -ForegroundColor Yellow
            Write-Host "  .\scripts\coverage.ps1 -Open" -ForegroundColor Yellow
        }
    } else {
        Write-Host ""
        Write-Host "❌ Coverage generation failed!" -ForegroundColor Red
        exit 1
    }
}

# Return to root directory
Set-Location ..

Write-Host ""
Write-Host "📈 Coverage analysis complete!" -ForegroundColor Green
