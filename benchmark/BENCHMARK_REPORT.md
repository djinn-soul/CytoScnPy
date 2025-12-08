# Benchmark Report

**Date:** 2025-12-06 11:53:43

## Binary Information

- **Path:** `target/release/cytoscnpy-bin.exe`
- **Size:** 7.20 MB (7,546,880 bytes)
- **Platform:** Windows

## Scan Performance

- **Scan Target:** `benchmark/examples` (126 ground truth items)
- **Time Taken:** ~0.11 seconds (typical)
- **Max Memory Usage (RSS):** ~14 MB (typical)

## Ground Truth Summary

| Type      | Count   |
| --------- | ------- |
| Functions | 50      |
| Classes   | 11      |
| Methods   | 27      |
| Imports   | 19      |
| Variables | 19      |
| **Total** | **126** |

## Tools Benchmarked

| Tool               | Time (s) | Memory (MB) | F1 Score |
| ------------------ | -------- | ----------- | -------- |
| CytoScnPy (Rust)   | 0.11     | 14.0        | 0.5878   |
| CytoScnPy (Python) | 2.41     | 24.5        | 0.5878   |
| Skylos             | 1.56     | 69.5        | 0.5981   |
| Vulture            | 0.30     | 25.1        | 0.6848   |
| Flake8             | 4.08     | 277.0       | 0.1899   |
| Pylint             | 9.58     | 422.4       | 0.2112   |
| Ruff               | 0.44     | 42.5        | 0.2824   |
| uncalled           | 0.37     | 23.7        | 0.5771   |
| dead               | 0.60     | 37.4        | 0.3280   |

## How to Run

```bash
# Activate environment
.\.venv\Scripts\activate  # Windows
source .venv/bin/activate  # Linux/Mac

# Check tool availability
python benchmark/benchmark_and_verify.py --check

# Run full benchmark
python benchmark/benchmark_and_verify.py
```
