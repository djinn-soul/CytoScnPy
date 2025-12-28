$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

# PGO Build Script for CytoScnPy
# Based on: https://doc.rust-lang.org/rustc/profile-guided-optimization.html

$projectRoot = (Resolve-Path "$PSScriptRoot\..").Path
$pgoDataDir = Join-Path $projectRoot "target\pgo-data"
# LLVM tools on Windows work better with forward slashes
$pgoDataDirUnix = $pgoDataDir -replace '\\', '/'

Write-Host "🚀 Starting PGO Build Process..." -ForegroundColor Cyan
Write-Host "   Project root: $projectRoot"
Write-Host "   PGO data dir: $pgoDataDir"

# 1. Install llvm-tools-preview if missing
Write-Host "Checking for llvm-tools-preview..."
rustup component add llvm-tools-preview

# 2. Cleanup previous PGO data
if (Test-Path $pgoDataDir) {
    Remove-Item -Path $pgoDataDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $pgoDataDir | Out-Null
cargo clean

# 3. Build with instrumentation
Write-Host "🏗️  Building with instrumentation..." -ForegroundColor Cyan
# Use forward slashes in the RUSTFLAGS path for LLVM
# -Ccodegen-units=1 is REQUIRED for reliable PGO on Windows
# -vp-counters-per-site=8 allocates more counters for large codebases
$env:RUSTFLAGS="-Cprofile-generate=$pgoDataDirUnix -Ccodegen-units=1 -Cllvm-args=-vp-counters-per-site=8"
Write-Host "   RUSTFLAGS: $($env:RUSTFLAGS)"
cargo build --release --target x86_64-pc-windows-msvc

# 4. Generate profile data (Run workloads)
Write-Host "🏃 Running workload to generate profile data..." -ForegroundColor Cyan
$binary = Join-Path $projectRoot "target\x86_64-pc-windows-msvc\release\cytoscnpy-cli.exe"

if (-not (Test-Path $binary)) {
    Write-Error "Instrumented binary not found at: $binary"
    exit 1
}

# Set LLVM_PROFILE_FILE to control where profile data is written
# %p = process ID, %m = unique merge pool ID (avoids conflicts)  
# Use forward slashes for LLVM compatibility on Windows
$env:LLVM_PROFILE_FILE = "$pgoDataDirUnix/cytoscnpy-%p-%m.profraw"
Write-Host "   Profile output: $($env:LLVM_PROFILE_FILE)"

# Run benchmark suite as workload
# We run it on the repo itself to exercise the analyzer
Push-Location $projectRoot

Write-Host "   - Analyzing self (CytoScnPy codebase)..."
& $binary . --quiet
if ($LASTEXITCODE -ne 0) {
    Write-Host "     (Analysis completed with findings - this is expected)"
}

# Run on benchmark folder specifically  
Write-Host "   - Analyzing benchmark/examples..."
& $binary benchmark/examples --quiet --clones --secrets
if ($LASTEXITCODE -ne 0) {
    Write-Host "     (Analysis completed with findings - this is expected)"
}

# Run additional workloads to generate more profile data
Write-Host "   - Running complexity analysis..."
& $binary cc . --json 2>&1 | Out-Null

Write-Host "   - Running maintainability analysis..."
& $binary mi . --json 2>&1 | Out-Null

Write-Host "   - Running Halstead metrics..."
& $binary hal . --json 2>&1 | Out-Null

Write-Host "   - Running raw metrics..."
& $binary raw . --json 2>&1 | Out-Null

Pop-Location

# 5. Merge profile data
Write-Host "DATA Merging profile data..." -ForegroundColor Cyan
# Find llvm-profdata
$rustcSysRoot = rustc --print sysroot
$llvmProfData = Get-ChildItem -Path "$rustcSysRoot\lib\rustlib\x86_64-pc-windows-msvc\bin\llvm-profdata.exe" -ErrorAction SilentlyContinue | Select-Object -First 1

if (-not $llvmProfData) {
    # Fallback search
    $llvmProfData = Get-Command llvm-profdata -ErrorAction SilentlyContinue
}

if (-not $llvmProfData) {
    Write-Error "Could not find llvm-profdata.exe. ensure llvm-tools-preview is installed."
}

# Find all .profraw files generated during profiling
$profRawFiles = Get-ChildItem -Path $pgoDataDir -Filter "*.profraw" -Recurse -ErrorAction SilentlyContinue

if (-not $profRawFiles -or $profRawFiles.Count -eq 0) {
    Write-Error "No .profraw files generated. Check that the instrumented binary ran correctly."
    exit 1
}

Write-Host "   Found $($profRawFiles.Count) profile data files"

$mergedData = Join-Path $pgoDataDir "merged.profdata"
# Pass all .profraw files to llvm-profdata merge
& $llvmProfData merge -o $mergedData $profRawFiles.FullName

if (-not (Test-Path $mergedData)) {
    Write-Error "Failed to create merged profile data."
    exit 1
}

# 6. Build optimized binary
Write-Host "🚀 Building FINAL optimized binary with PGO..." -ForegroundColor Green
$mergedDataUnix = $mergedData -replace '\\', '/'
# Note: LTO must be set in Cargo.toml profile, not RUSTFLAGS (doesn't work for lib crates)
$env:RUSTFLAGS="-Cprofile-use=$mergedDataUnix -Cllvm-args=-pgo-warn-missing-function -Ccodegen-units=1"
Write-Host "   RUSTFLAGS: $($env:RUSTFLAGS)"
cargo build --release --target x86_64-pc-windows-msvc

# 7. Check size
$finalBinary = Get-Item "target\x86_64-pc-windows-msvc\release\cytoscnpy-cli.exe"
$sizeMB = [math]::Round($finalBinary.Length / 1MB, 2)
Write-Host "✅ PGO Build Complete!" -ForegroundColor Green
Write-Host "   Binary: $($finalBinary.FullName)"
Write-Host "   Size:   $sizeMB MB"
