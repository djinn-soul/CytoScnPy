#!/bin/bash
set -e

# Configuration
REPO="djinn09/CytoScnPy"
BINARY_NAME="cytoscnpy"
INSTALL_DIR="/usr/local/bin"

# Detect OS and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        ASSET_NAME="${BINARY_NAME}-linux-x64"
        ;;
    Darwin)
        if [ "$ARCH" == "arm64" ]; then
            ASSET_NAME="${BINARY_NAME}-macos-arm64"
        else
            ASSET_NAME="${BINARY_NAME}-macos-x64"
        fi
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

echo "Detected platform: $OS $ARCH"
echo "Downloading $ASSET_NAME..."

# Get the latest release URL
LATEST_URL=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep "browser_download_url.*$ASSET_NAME" | cut -d : -f 2,3 | tr -d \")

if [ -z "$LATEST_URL" ]; then
    echo "Error: Could not find release asset for your platform."
    exit 1
fi

# Download and Install
curl -L -o "$BINARY_NAME" "$LATEST_URL"
chmod +x "$BINARY_NAME"

echo "Installing to $INSTALL_DIR (requires sudo)..."
sudo mv "$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"

echo ""
echo "Success! CytoScnPy CLI installed."
echo ""
echo "Usage:"
echo "  cytoscnpy .                    # Analyze current directory"
echo "  cytoscnpy mcp-server           # Start MCP server for AI assistants"
echo ""
echo "For MCP configuration (Claude, Cursor, Copilot), see:"
echo "  https://github.com/djinn09/CytoScnPy/blob/main/cytoscnpy-mcp/README.md"
