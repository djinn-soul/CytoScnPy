#!/bin/bash
# Quick test with coverage display (Linux/macOS)
# Run this instead of 'cargo test' to see coverage

# Colors
CYAN='\033[0;36m'
NC='\033[0m'

TEST_NAME=""

# Parse arguments
if [ $# -gt 0 ]; then
    TEST_NAME="$1"
fi

echo -e "${CYAN}🧪 Running tests with coverage...${NC}"

# Navigate to cytoscnpy
cd cytoscnpy || exit 1

if [ -n "$TEST_NAME" ]; then
    # Run specific test with coverage
    cargo llvm-cov --test "$TEST_NAME"
else
    # Run all tests with coverage summary
    cargo llvm-cov --all-features
fi

# Return to root
cd ..
