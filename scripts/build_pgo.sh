#!/bin/bash
# PGO Build Script for CytoScnPy (Linux/macOS)
# Based on: https://doc.rust-lang.org/rustc/profile-guided-optimization.html
#
# Usage: ./scripts/build_pgo.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PGO_DATA_DIR="$PROJECT_ROOT/target/pgo-data"

echo "🚀 Starting PGO Build Process..."
echo "   Project root: $PROJECT_ROOT"
echo "   PGO data dir: $PGO_DATA_DIR"

# 1. Install llvm-tools-preview if missing
echo "Checking for llvm-tools-preview..."
rustup component add llvm-tools-preview

# 2. Cleanup previous PGO data
rm -rf "$PGO_DATA_DIR"
mkdir -p "$PGO_DATA_DIR"
cargo clean

# 3. Build with instrumentation
echo "🏗️  Building with instrumentation..."
# -Ccodegen-units=1 is REQUIRED for reliable PGO
# -vp-counters-per-site=8 allocates more counters for large codebases
export RUSTFLAGS="-Cprofile-generate=$PGO_DATA_DIR -Ccodegen-units=1 -Cllvm-args=-vp-counters-per-site=8"
echo "   RUSTFLAGS: $RUSTFLAGS"
cargo build --release -p cytoscnpy-cli

# 4. Generate profile data (Run workloads)
echo "🏃 Running workload to generate profile data..."

# Detect binary location
if [[ "$OSTYPE" == "darwin"* ]]; then
    BINARY="$PROJECT_ROOT/target/release/cytoscnpy-cli"
else
    BINARY="$PROJECT_ROOT/target/release/cytoscnpy-cli"
fi

if [[ ! -f "$BINARY" ]]; then
    echo "❌ Instrumented binary not found at: $BINARY"
    exit 1
fi

# Set LLVM_PROFILE_FILE to control where profile data is written
# %p = process ID, %m = unique merge pool ID (avoids conflicts)
export LLVM_PROFILE_FILE="$PGO_DATA_DIR/cytoscnpy-%p-%m.profraw"
echo "   Profile output: $LLVM_PROFILE_FILE"

cd "$PROJECT_ROOT"

echo "   - Analyzing self (CytoScnPy codebase)..."
$BINARY . --quiet || echo "     (Analysis completed with findings - this is expected)"

echo "   - Analyzing benchmark/examples..."
$BINARY benchmark/examples --quiet --clones --secrets || echo "     (Analysis completed with findings - this is expected)"

# Run additional workloads to generate more profile data
echo "   - Running complexity analysis..."
$BINARY cc . --json > /dev/null 2>&1 || true

echo "   - Running maintainability analysis..."
$BINARY mi . --json > /dev/null 2>&1 || true

echo "   - Running Halstead metrics..."
$BINARY hal . --json > /dev/null 2>&1 || true

echo "   - Running raw metrics..."
$BINARY raw . --json > /dev/null 2>&1 || true

# 5. Merge profile data
echo "📊 Merging profile data..."

# Find llvm-profdata
RUSTC_SYSROOT=$(rustc --print sysroot)
LLVM_PROFDATA=$(find "$RUSTC_SYSROOT" -name "llvm-profdata" -type f 2>/dev/null | head -1)

if [[ -z "$LLVM_PROFDATA" ]]; then
    # Try system llvm-profdata
    LLVM_PROFDATA=$(which llvm-profdata 2>/dev/null || true)
fi

if [[ -z "$LLVM_PROFDATA" ]]; then
    echo "❌ Could not find llvm-profdata. Ensure llvm-tools-preview is installed."
    exit 1
fi

echo "   Using: $LLVM_PROFDATA"

# Find all .profraw files
PROFRAW_FILES=$(find "$PGO_DATA_DIR" -name "*.profraw" 2>/dev/null)

if [[ -z "$PROFRAW_FILES" ]]; then
    echo "❌ No .profraw files generated. Check that the instrumented binary ran correctly."
    exit 1
fi

PROFRAW_COUNT=$(echo "$PROFRAW_FILES" | wc -l)
echo "   Found $PROFRAW_COUNT profile data files"

MERGED_DATA="$PGO_DATA_DIR/merged.profdata"
$LLVM_PROFDATA merge -o "$MERGED_DATA" $PROFRAW_FILES

if [[ ! -f "$MERGED_DATA" ]]; then
    echo "❌ Failed to create merged profile data."
    exit 1
fi

# 6. Build optimized binary
echo "🚀 Building FINAL optimized binary with PGO..."
export RUSTFLAGS="-Cprofile-use=$MERGED_DATA -Cllvm-args=-pgo-warn-missing-function -Ccodegen-units=1"
echo "   RUSTFLAGS: $RUSTFLAGS"
cargo build --release -p cytoscnpy-cli

# 7. Check size
FINAL_BINARY="$PROJECT_ROOT/target/release/cytoscnpy-cli"
SIZE_BYTES=$(stat -f%z "$FINAL_BINARY" 2>/dev/null || stat --printf="%s" "$FINAL_BINARY" 2>/dev/null)
SIZE_MB=$(echo "scale=2; $SIZE_BYTES / 1048576" | bc)

echo "✅ PGO Build Complete!"
echo "   Binary: $FINAL_BINARY"
echo "   Size:   ${SIZE_MB} MB"
echo ""
echo "📦 To update the committed profile:"
echo "   cp $MERGED_DATA $PROJECT_ROOT/pgo-profiles/"
