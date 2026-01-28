#!/bin/bash
# Code Coverage Script for CytoScnPy (Linux/macOS)
# Generates HTML and JSON coverage reports using cargo-llvm-cov

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Parse arguments
OPEN=false
JSON=false
SUMMARY=false
INSTALL=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --open|-o)
            OPEN=true
            shift
            ;;
        --json|-j)
            JSON=true
            shift
            ;;
        --summary|-s)
            SUMMARY=true
            shift
            ;;
        --install|-i)
            INSTALL=true
            shift
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

echo -e "${CYAN}🎯 CytoScnPy Code Coverage Tool${NC}"
echo ""

# Check if cargo-llvm-cov is installed
if ! command -v cargo-llvm-cov &> /dev/null && [ "$INSTALL" = false ]; then
    echo -e "${RED}❌ cargo-llvm-cov is not installed!${NC}"
    echo ""
    echo -e "${YELLOW}To install, run:${NC}"
    echo -e "${YELLOW}  ./scripts/coverage.sh --install${NC}"
    echo ""
    echo -e "${YELLOW}Or manually:${NC}"
    echo -e "${YELLOW}  cargo install cargo-llvm-cov${NC}"
    exit 1
fi

# Install cargo-llvm-cov if requested
if [ "$INSTALL" = true ]; then
    echo -e "${YELLOW}📦 Installing cargo-llvm-cov...${NC}"
    cargo install cargo-llvm-cov
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ cargo-llvm-cov installed successfully!${NC}"
    else
        echo -e "${RED}❌ Failed to install cargo-llvm-cov${NC}"
        exit 1
    fi
    exit 0
fi

# Navigate to cytoscnpy directory
cd cytoscnpy || exit 1

# Clean previous coverage data
echo -e "${YELLOW}🧹 Cleaning previous coverage data...${NC}"
cargo llvm-cov clean

# Generate coverage
echo -e "${YELLOW}🔬 Running tests with coverage instrumentation...${NC}"
echo ""

if [ "$SUMMARY" = true ]; then
    # Just show summary in terminal
    cargo llvm-cov --all-features
elif [ "$JSON" = true ]; then
    # Generate JSON report for CI/CD
    echo -e "${YELLOW}📊 Generating JSON coverage report...${NC}"
    cargo llvm-cov --all-features --json --output-path ../coverage.json
    echo -e "${GREEN}✅ Coverage report saved to: coverage.json${NC}"
else
    # Generate HTML report
    echo -e "${YELLOW}📊 Generating HTML coverage report...${NC}"
    cargo llvm-cov --all-features --html

    if [ $? -eq 0 ]; then
        echo ""
        echo -e "${GREEN}✅ Coverage report generated successfully!${NC}"
        echo ""
        echo -e "${CYAN}📁 Report location: cytoscnpy/target/llvm-cov/html/index.html${NC}"

        if [ "$OPEN" = true ]; then
            echo -e "${YELLOW}🌐 Opening coverage report in browser...${NC}"
            # Try different browsers/open commands
            if command -v xdg-open &> /dev/null; then
                xdg-open target/llvm-cov/html/index.html
            elif command -v open &> /dev/null; then
                open target/llvm-cov/html/index.html
            else
                echo -e "${YELLOW}Please open: cytoscnpy/target/llvm-cov/html/index.html${NC}"
            fi
        else
            echo ""
            echo -e "${YELLOW}To open the report, run:${NC}"
            echo -e "${YELLOW}  ./scripts/coverage.sh --open${NC}"
        fi
    else
        echo ""
        echo -e "${RED}❌ Coverage generation failed!${NC}"
        exit 1
    fi
fi

# Return to root directory
cd ..

echo ""
echo -e "${GREEN}📈 Coverage analysis complete!${NC}"
