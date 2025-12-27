# CytoScnPy Development Scripts

This directory contains helper scripts for development and testing on both **Windows** and **Linux/macOS**.

## Scripts

### PGO Profile Loading

**`load-pgo-profile.ps1`** / **`load-pgo-profile.sh`** - Load PGO profiles for release builds

```bash
# Linux/macOS
source scripts/load-pgo-profile.sh auto

# Windows PowerShell
. scripts/load-pgo-profile.ps1 -Platform windows
```

### PGO Build (Full Process)

**`build_pgo.ps1`** / **`build_pgo.sh`** - Complete Profile-Guided Optimization build

```bash
# Linux/macOS
./scripts/build_pgo.sh

# Windows PowerShell
.\scripts\build_pgo.ps1
```

These scripts perform the full PGO workflow:

1. Install `llvm-tools-preview` component
2. Build instrumented binary
3. Run workloads to generate profile data (analyzes the codebase itself)
4. Merge `.profraw` files into `.profdata`
5. Build final optimized binary with PGO

> **Note:** This is a time-consuming process (~5-10 min). Use `load-pgo-profile` for quick builds with pre-generated profiles.

### Coverage Reports

#### Windows (PowerShell)

**`coverage.ps1`** - Generate comprehensive code coverage reports

```powershell
# Install cargo-llvm-cov
.\scripts\coverage.ps1 -Install

# Generate HTML report
.\scripts\coverage.ps1

# Open report in browser
.\scripts\coverage.ps1 -Open

# Show summary in terminal
.\scripts\coverage.ps1 -Summary

# Generate JSON for CI/CD
.\scripts\coverage.ps1 -Json
```

#### Linux/macOS (Bash)

**`coverage.sh`** - Same functionality for Unix systems

```bash
# Make executable (first time only)
chmod +x scripts/coverage.sh

# Install cargo-llvm-cov
./scripts/coverage.sh --install

# Generate HTML report
./scripts/coverage.sh

# Open report in browser
./scripts/coverage.sh --open

# Show summary in terminal
./scripts/coverage.sh --summary

# Generate JSON for CI/CD
./scripts/coverage.sh --json
```

### Quick Test with Coverage

#### Windows (PowerShell)

**`test-coverage.ps1`** - Quick test run with coverage summary

```powershell
# Run all tests with coverage
.\scripts\test-coverage.ps1

# Run specific test with coverage
.\scripts\test-coverage.ps1 -TestName "analyzer_test"
```

#### Linux/macOS (Bash)

**`test-coverage.sh`** - Same functionality for Unix systems

```bash
# Make executable (first time only)
chmod +x scripts/test-coverage.sh

# Run all tests with coverage
./scripts/test-coverage.sh

# Run specific test with coverage
./scripts/test-coverage.sh analyzer_test
```

**Tip:** Use this instead of `cargo test` during development - shows coverage every time!

## Setting Up Coverage

### Windows

```powershell
# 1. Install cargo-llvm-cov
.\scripts\coverage.ps1 -Install

# 2. Run tests with coverage
.\scripts\test-coverage.ps1

# 3. Generate full HTML report
.\scripts\coverage.ps1 -Open
```

### Linux/macOS

```bash
# 1. Make scripts executable
chmod +x scripts/*.sh

# 2. Install cargo-llvm-cov
./scripts/coverage.sh --install

# 3. Run tests with coverage
./scripts/test-coverage.sh

# 4. Generate full HTML report
./scripts/coverage.sh --open
```

## CI/CD Integration

Coverage is automatically generated on every push/PR via GitHub Actions.
See `.github/workflows/coverage.yml` for configuration.

## Platform Support

- ✅ **Windows**: PowerShell scripts (`.ps1`)
- ✅ **Linux**: Bash scripts (`.sh`)
- ✅ **macOS**: Bash scripts (`.sh`)
- ✅ **WSL**: Both PowerShell and Bash work
