# Benchmarking Setup for CytoScnPy Optimizations

## Philosophy: Measure First, Optimize Second

> "Premature optimization is the root of all evil" - Donald Knuth

Before implementing ANY optimizations from `OPTIMIZATION_RECOMMENDATIONS.md`, we need:

1. ✅ **Baseline measurements** - Know current performance
2. ✅ **Test datasets** - Realistic large projects
3. ✅ **Automated benchmarks** - Repeatable, reliable measurements
4. ✅ **Regression detection** - Catch performance losses

---

## Current Baseline Results (Hyperfine)

Measured on: 2025-12-12 | Tool: hyperfine 1.20.0 | Platform: Windows 11

| Dataset            | Python Files | Lines     | Mean Time  | Stddev | Min   | Max   |
| ------------------ | ------------ | --------- | ---------- | ------ | ----- | ----- |
| tiny_flask         | 83           | 18,240    | **0.214s** | 0.169  | 0.100 | 0.644 |
| small_requests     | 36           | 11,248    | **0.184s** | 0.087  | 0.120 | 0.410 |
| medium_fastapi     | 1,279        | 114,154   | **0.402s** | 0.058  | 0.352 | 0.549 |
| large_django       | 2,886        | 506,403   | **1.606s** | 0.415  | 0.984 | 2.370 |
| massive_tensorflow | 3,147        | 1,216,986 | **4.050s** | 0.632  | 2.832 | 5.216 |

**Total:** 7,431 files, 1.87M lines  
**Average Throughput:** ~289,000 lines/second

---

## Test Datasets

### Downloaded Projects

| Tier    | Dataset            | Files | Lines | Source                           |
| ------- | ------------------ | ----- | ----- | -------------------------------- |
| Tiny    | tiny_flask         | 83    | 18K   | github.com/pallets/flask         |
| Small   | small_requests     | 36    | 11K   | github.com/psf/requests          |
| Medium  | medium_fastapi     | 1,279 | 114K  | github.com/tiangolo/fastapi      |
| Large   | large_django       | 2,886 | 506K  | github.com/django/django         |
| Massive | massive_tensorflow | 3,147 | 1.2M  | github.com/tensorflow/tensorflow |

### Download Commands

```bash
cd benchmark/datasets
git clone --depth 1 https://github.com/pallets/flask.git tiny_flask
git clone --depth 1 https://github.com/psf/requests.git small_requests
git clone --depth 1 https://github.com/tiangolo/fastapi.git medium_fastapi
git clone --depth 1 https://github.com/django/django.git large_django
git clone --depth 1 https://github.com/tensorflow/tensorflow.git massive_tensorflow
```

---

## Benchmark Tools

### Primary: Hyperfine (Recommended)

**Why hyperfine:**

- Statistical analysis (mean, stddev, min, max)
- Warmup runs to prime caches
- Outlier detection
- JSON/Markdown export
- Cross-platform

**Install:**

```bash
# Windows (scoop)
scoop install hyperfine

# macOS
brew install hyperfine

# Linux / Cargo
cargo install hyperfine
```

**Run benchmarks:**

```bash
# Run full suite with Python wrapper
python benchmark/run_benchmarks.py

# Or run hyperfine directly
hyperfine --warmup 3 --runs 10 \
  'target/release/cytoscnpy-bin analyze benchmark/datasets/large_django --json'
```

### Secondary: Criterion (Rust micro-benchmarks)

For internal function benchmarks (resolve_name, get_qualified_name, etc.):

```toml
# cytoscnpy/Cargo.toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
```

```bash
cd cytoscnpy
cargo bench
```

---

## Benchmark Scripts

### `benchmark/run_benchmarks.py`

Main benchmark runner using hyperfine:

- Runs 10 iterations per dataset with 3 warmup
- Exports JSON results with statistics
- Generates comparison tables

```bash
python benchmark/run_benchmarks.py
```

**Output files:**

- `benchmark/baseline_results.json` - Main results
- `benchmark/comparison.md` - Markdown table
- `benchmark/comparison.json` - JSON comparison
- `benchmark/results_*.json` - Per-dataset results

### `benchmark/compare.py`

Compare two binary versions:

```bash
# Build baseline
git stash
cargo build --release
cp target/release/cytoscnpy-bin.exe baseline.exe

# Build optimized
git stash pop
cargo build --release

# Compare
python benchmark/compare.py baseline.exe target/release/cytoscnpy-bin.exe
```

---

## Quick Start

```bash
# 1. Build release binary
cargo build --release

# 2. Run benchmarks (requires hyperfine)
python benchmark/run_benchmarks.py

# 3. View results
cat benchmark/baseline_results.json
cat benchmark/comparison.md
```

---

## Optimization Workflow

### Before Optimizing

```bash
# 1. Ensure clean git state
git status

# 2. Run baseline benchmarks
python benchmark/run_benchmarks.py

# 3. Save baseline binary
cp target/release/cytoscnpy-bin.exe benchmark/baseline.exe
```

### After Optimizing

```bash
# 1. Rebuild
cargo build --release

# 2. Compare against baseline
python benchmark/compare.py benchmark/baseline.exe target/release/cytoscnpy-bin.exe

# 3. Validate all tests pass
cargo test
```

---

## Success Criteria

An optimization is successful if:

- ✅ **Performance:** ≥5% improvement on ≥3 datasets
- ✅ **Memory:** No increase (preferably reduction)
- ✅ **Accuracy:** All tests pass
- ✅ **Scalability:** No regression on massive_tensorflow

---

## Key Files

| File                              | Purpose                      |
| --------------------------------- | ---------------------------- |
| `benchmark/run_benchmarks.py`     | Main hyperfine runner        |
| `benchmark/compare.py`            | A/B comparison tool          |
| `benchmark/baseline_results.json` | Saved baseline data          |
| `benchmark/datasets/`             | Test project repos           |
| `OPTIMIZATION_RECOMMENDATIONS.md` | Optimization guide           |
| `RAYON_PARALLELIZATION.md`        | Parallel processing analysis |
