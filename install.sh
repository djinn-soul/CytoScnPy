#!/bin/bash
set -euo pipefail

# Configuration
REPO="djinn-soul/CytoScnPy"
BINARY_NAME="cytoscnpy"
INSTALL_DIR="/usr/local/bin"

# Detect OS and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        if [ "$ARCH" != "x86_64" ]; then
            echo "Unsupported Linux architecture: $ARCH"
            exit 1
        fi
        ASSET_NAME="${BINARY_NAME}-linux-x64"
        ;;
    Darwin)
        if [ "$ARCH" == "arm64" ]; then
            ASSET_NAME="${BINARY_NAME}-macos-arm64"
        elif [ "$ARCH" == "x86_64" ]; then
            ASSET_NAME="${BINARY_NAME}-macos-x64"
        else
            echo "Unsupported macOS architecture: $ARCH"
            exit 1
        fi
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

echo "Detected platform: $OS $ARCH"
echo "Downloading $ASSET_NAME..."

# Download to a private temporary directory so failed verification cannot
# replace an existing installation.
TEMP_DIR=$(mktemp -d)
DOWNLOAD_PATH="$TEMP_DIR/$ASSET_NAME"
CHECKSUM_PATH="$TEMP_DIR/SHA256SUMS.txt"
RELEASE_BASE="https://github.com/$REPO/releases/latest/download"
cleanup() {
    rm -f -- "$DOWNLOAD_PATH" "$CHECKSUM_PATH"
    rmdir -- "$TEMP_DIR" 2>/dev/null || true
}
trap cleanup EXIT

# WARNING: Never install the downloaded executable before this release checksum
# verification succeeds.
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
    --output "$DOWNLOAD_PATH" "$RELEASE_BASE/$ASSET_NAME"
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
    --output "$CHECKSUM_PATH" "$RELEASE_BASE/SHA256SUMS.txt"

EXPECTED_HASH=$(awk -v asset="$ASSET_NAME" '$2 == asset { print $1 }' "$CHECKSUM_PATH")
if [[ ! "$EXPECTED_HASH" =~ ^[0-9a-fA-F]{64}$ ]]; then
    echo "Error: Release checksum is missing or malformed for $ASSET_NAME."
    exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL_HASH=$(sha256sum "$DOWNLOAD_PATH" | awk '{ print $1 }')
elif command -v shasum >/dev/null 2>&1; then
    ACTUAL_HASH=$(shasum -a 256 "$DOWNLOAD_PATH" | awk '{ print $1 }')
else
    echo "Error: No SHA-256 utility is available."
    exit 1
fi
if [ "${ACTUAL_HASH,,}" != "${EXPECTED_HASH,,}" ]; then
    echo "Error: SHA-256 verification failed for $ASSET_NAME."
    exit 1
fi

echo "Installing to $INSTALL_DIR (requires sudo)..."
sudo install -m 0755 "$DOWNLOAD_PATH" "$INSTALL_DIR/$BINARY_NAME"

echo ""
echo "Success! CytoScnPy CLI installed."
echo ""
echo "Usage:"
echo "  cytoscnpy .                    # Analyze current directory"
echo "  cytoscnpy mcp-server           # Start MCP server for AI assistants"
echo ""
echo "For MCP configuration (Claude, Cursor, Copilot), see:"
echo "  https://github.com/djinn-soul/CytoScnPy/blob/main/cytoscnpy-mcp/README.md"
