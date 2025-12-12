#!/usr/bin/env pwsh
# CytoScnPy Benchmark Suite using Hyperfine
# More accurate than Python-based timing with warmup, statistics, and outlier detection

$BINARY = "target\release\cytoscnpy-bin.exe"
$DATASETS = @(
    "benchmark\datasets\tiny_flask",
    "benchmark\datasets\small_requests",
    "benchmark\datasets\medium_fastapi",
    "benchmark\datasets\large_django",
    "benchmark\datasets\massive_tensorflow"
)

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "CytoScnPy Benchmark Suite (Hyperfine)" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host ""

# Check binary exists
if (-not (Test-Path $BINARY)) {
    Write-Host "Binary not found: $BINARY" -ForegroundColor Red
    Write-Host "Run 'cargo build --release' first" -ForegroundColor Yellow
    exit 1
}

# Run individual benchmarks
Write-Host "Running individual benchmarks..." -ForegroundColor Green
Write-Host ""

foreach ($dataset in $DATASETS) {
    if (Test-Path $dataset) {
        $name = Split-Path $dataset -Leaf
        Write-Host "=== $name ===" -ForegroundColor Yellow
        hyperfine --warmup 3 --runs 10 --export-json "benchmark\results_$name.json" `
            "$BINARY analyze $dataset --json" 2>&1
        Write-Host ""
    }
}

# Run comparison of all datasets
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "Comparative Benchmark (All Datasets)" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

$cmds = @()
foreach ($dataset in $DATASETS) {
    if (Test-Path $dataset) {
        $name = Split-Path $dataset -Leaf
        $cmds += "--command-name"
        $cmds += $name
        $cmds += "$BINARY analyze $dataset --json"
    }
}

hyperfine --warmup 2 --runs 5 @cmds --export-markdown "benchmark\comparison.md" --export-json "benchmark\comparison.json"

Write-Host ""
Write-Host "Results saved to:" -ForegroundColor Green
Write-Host "  - benchmark\comparison.md (Markdown table)"
Write-Host "  - benchmark\comparison.json (JSON data)"
Write-Host "  - benchmark\results_*.json (Individual results)"
