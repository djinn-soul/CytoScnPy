# Load PGO profile and set RUSTFLAGS (PowerShell for Windows)
# Usage: . scripts/load-pgo-profile.ps1 [-Platform windows]

param(
    [string]$Platform = "windows"
)

# Get the repository root (script is in scripts/ folder)
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot = Split-Path -Parent $ScriptDir

# Determine profile path
$PGO_PROFILE = "$RepoRoot/pgo-profiles/$Platform/merged.profdata"

# Fallback to Windows profile if platform-specific doesn't exist
if (-not (Test-Path $PGO_PROFILE)) {
    $PGO_PROFILE = "$RepoRoot/pgo-profiles/windows/merged.profdata"
}

# Set RUSTFLAGS
$env:RUSTFLAGS = "-Cprofile-use=$PGO_PROFILE -Ccodegen-units=1"

Write-Host "PGO Profile: $PGO_PROFILE"
Write-Host "RUSTFLAGS: $env:RUSTFLAGS"
