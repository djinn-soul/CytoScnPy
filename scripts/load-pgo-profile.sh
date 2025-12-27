#!/usr/bin/env bash
# Load PGO profile and set RUSTFLAGS
# Usage: source scripts/load-pgo-profile.sh [platform]
# Platform: windows, linux, macos, or auto (default)

set -e

PLATFORM="${1:-auto}"

# Get the repository root (script is in scripts/ folder)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

# Auto-detect platform if not specified
if [ "$PLATFORM" == "auto" ]; then
    case "$(uname -s)" in
        MINGW*|MSYS*|CYGWIN*) PLATFORM="windows" ;;
        Linux*)               PLATFORM="linux" ;;
        Darwin*)              PLATFORM="macos" ;;
        *)                    PLATFORM="windows" ;;  # Fallback
    esac
fi

# Determine profile path
PGO_PROFILE="${REPO_ROOT}/pgo-profiles/${PLATFORM}/merged.profdata"

# Fallback to Windows profile if platform-specific doesn't exist
if [ ! -f "$PGO_PROFILE" ]; then
    PGO_PROFILE="${REPO_ROOT}/pgo-profiles/windows/merged.profdata"
fi

# Convert Windows path to Unix-style for LLVM on Windows
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    PGO_PROFILE=$(echo "$PGO_PROFILE" | sed 's|\\|/|g')
fi

# Export RUSTFLAGS with PGO profile
export RUSTFLAGS="-Cprofile-use=${PGO_PROFILE} -Ccodegen-units=1"

echo "PGO Profile: $PGO_PROFILE"
echo "RUSTFLAGS: $RUSTFLAGS"
