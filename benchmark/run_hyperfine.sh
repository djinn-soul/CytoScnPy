#!/bin/bash
# CytoScnPy Benchmark Suite using Hyperfine (Linux/macOS)

BINARY="target/release/cytoscnpy-bin"
DATASETS=(
    "benchmark/datasets/tiny_flask"
    "benchmark/datasets/small_requests"
    "benchmark/datasets/medium_fastapi"
    "benchmark/datasets/large_django"
    "benchmark/datasets/massive_tensorflow"
)

echo "============================================================"
echo "CytoScnPy Benchmark Suite (Hyperfine)"
echo "============================================================"
echo ""

# Check binary exists
if [ ! -f "$BINARY" ]; then
    echo "Binary not found: $BINARY"
    echo "Run 'cargo build --release' first"
    exit 1
fi

# Run individual benchmarks
echo "Running individual benchmarks..."
echo ""

for dataset in "${DATASETS[@]}"; do
    if [ -d "$dataset" ]; then
        name=$(basename "$dataset")
        echo "=== $name ==="
        hyperfine --warmup 3 --runs 10 --export-json "benchmark/results_$name.json" \
            "$BINARY analyze $dataset --json"
        echo ""
    fi
done

# Run comparison of all datasets
echo "============================================================"
echo "Comparative Benchmark (All Datasets)"
echo "============================================================"

cmds=""
for dataset in "${DATASETS[@]}"; do
    if [ -d "$dataset" ]; then
        name=$(basename "$dataset")
        cmds="$cmds --command-name $name '$BINARY analyze $dataset --json'"
    fi
done

eval "hyperfine --warmup 2 --runs 5 $cmds --export-markdown benchmark/comparison.md --export-json benchmark/comparison.json"

echo ""
echo "Results saved to:"
echo "  - benchmark/comparison.md (Markdown table)"
echo "  - benchmark/comparison.json (JSON data)"
echo "  - benchmark/results_*.json (Individual results)"
