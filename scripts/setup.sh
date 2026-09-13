#!/usr/bin/env bash

set -euo pipefail

echo "========================================"
echo " Rust / Clang / Mold Setup for Fedora"
echo "========================================"

# -------------------------------------------------------
# 1. Check Fedora
# -------------------------------------------------------
if ! command -v dnf >/dev/null 2>&1; then
    echo "ERROR: dnf not found."
    echo "This script is intended for Fedora/RHEL-based systems."
    exit 1
fi

# -------------------------------------------------------
# 2. Update package metadata
# -------------------------------------------------------
echo
echo "[1/6] Updating package metadata..."
sudo dnf makecache

# -------------------------------------------------------
# 3. Install compiler + linker + common Rust build tools
# -------------------------------------------------------
echo
echo "[2/6] Installing build dependencies..."

sudo dnf install -y \
    gcc \
    gcc-c++ \
    clang \
    llvm \
    lld \
    mold \
    make \
    cmake \
    pkgconf-pkg-config \
    openssl-devel

# -------------------------------------------------------
# 4. Install Rust if missing
# -------------------------------------------------------
echo
echo "[3/6] Checking Rust..."

if ! command -v rustc >/dev/null 2>&1; then
    echo "Rust not found. Installing through rustup..."

    curl --proto '=https' --tlsv1.2 -sSf \
        https://sh.rustup.rs | sh -s -- -y

    source "$HOME/.cargo/env"
else
    echo "Rust already installed:"
    rustc --version
fi

# -------------------------------------------------------
# 5. Update Rust toolchain
# -------------------------------------------------------
echo
echo "[4/6] Updating Rust toolchain..."

if command -v rustup >/dev/null 2>&1; then
    rustup update stable
    rustup default stable
fi

# -------------------------------------------------------
# 6. Verify required tools
# -------------------------------------------------------
echo
echo "[5/6] Verifying installation..."

echo
echo "Rust:"
rustc --version
cargo --version

echo
echo "Clang:"
clang --version | head -n 1

echo
echo "Mold:"
mold --version

echo
echo "LLVM linker:"
ld.lld --version | head -n 1 || true

echo
echo "GCC:"
gcc --version | head -n 1

# -------------------------------------------------------
# Test clang + mold directly
# -------------------------------------------------------
echo
echo "[6/6] Testing clang with mold..."

TMP_DIR="$(mktemp -d)"

cat > "$TMP_DIR/test.c" <<'EOF'
#include <stdio.h>

int main(void) {
    printf("clang + mold working\n");
    return 0;
}
EOF

clang "$TMP_DIR/test.c" \
    -fuse-ld=mold \
    -o "$TMP_DIR/test"

"$TMP_DIR/test"

rm -rf "$TMP_DIR"

echo
echo "========================================"
echo " Setup completed successfully."
echo "========================================"
echo
echo "You can now run:"
echo
echo "    cargo clean"
echo "    cargo build"
echo
