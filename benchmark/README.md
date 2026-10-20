# Benchmark Report

---

This benchmark compares CytoScnPy with 8 other dead code detection tools (Vulture at
two confidence levels, so 11 runs in total) on a curated Python test suite with **164
ground truth items** across 5 detection types. It measures detection accuracy
(precision, recall, F1) and performance (execution time, peak memory).

Results below are from a Linux run on 2026-10-20 with CytoScnPy 1.3.0.

### Key Highlights

| Metric | Winner | Value |
| --- | --- | --- |
| 🚀 **Fastest Tool** | CytoScnPy (Rust) | 0.016s |
| 🎯 **Best Overall F1** | CytoScnPy | 0.8466 |
| 🔬 **Highest Precision** | CytoScnPy | 0.7926 |
| 🔎 **Highest Recall** | CytoScnPy | 0.9085 |
| 💾 **Lowest Memory** | CytoScnPy (Rust) | 1.9 MB |
| 🏆 **Best F1 in every category** | CytoScnPy | Class 0.97, Import 0.85, Method 0.84, Function 0.83, Variable 0.81 |

### Detection Capabilities at a Glance

A ✅ means the tool found at least one item of that type in this suite.

| Tool | Classes | Functions | Imports | Methods | Variables |
| --- | :-: | :-: | :-: | :-: | :-: |
| CytoScnPy | ✅ | ✅ | ✅ | ✅ | ✅ |
| Vulture | ✅ | ✅ | ✅ | ✅ | ✅ |
| Skylos | ✅ | ✅ | ✅ | ✅ | ✅ |
| deadcode | ✅ | ✅ | ✅ | ✅ | ✅ |
| uncalled | ❌ | ✅ | ❌ | ✅ | ❌ |
| dead | ❌ | ✅ | ❌ | ❌ | ✅ |
| Ruff | ❌ | ❌ | ✅ | ❌ | ✅ |
| Flake8 | ❌ | ❌ | ✅ | ❌ | ✅ |
| Pylint | ❌ | ❌ | ✅ | ❌ | ✅ |

### Quick Recommendations

- **For CI/CD Pipelines**: **CytoScnPy (Rust)**: fastest run, lowest memory, and the highest F1.
- **For a second opinion on functions**: **Vulture** or **deadcode**: recall close to CytoScnPy's, with more false positives.
- **For Minimal False Positives on methods**: **uncalled**: no false positives on methods, but it only checks functions and methods.
- **For Unused Imports Only**: **Ruff**: fast, mature F401 rule with good recall.

---

## Running the Benchmark

```bash
# Activate environment
.\.venv\Scripts\activate  # Windows
source .venv/bin/activate  # Linux/Mac

# Check tool availability
python benchmark/benchmark_and_verify.py --check

# List the tools
python benchmark/benchmark_and_verify.py --list

# Run benchmark (Standard)
python benchmark/benchmark_and_verify.py

# Run only some tools (substring match)
python benchmark/benchmark_and_verify.py -i cytoscnpy vulture
python benchmark/benchmark_and_verify.py -e pylint

# Run Regression Check (Compare against Baseline)
# Windows:
python benchmark/benchmark_and_verify.py --compare-json benchmark/baseline_win32.json
# Linux/CI:
python benchmark/benchmark_and_verify.py --compare-json benchmark/baseline_linux.json

# Update Baseline (Save current results)
# Windows:
python benchmark/benchmark_and_verify.py --save-json benchmark/baseline_win32.json
# Linux:
python benchmark/benchmark_and_verify.py --save-json benchmark/baseline_linux.json
```

## Implementation

`benchmark_and_verify.py` remains the CLI and import entry point. Its implementation
is organized in `benchmark_support/`: command execution and memory monitoring
(`process.py`), tool configuration (`configuration.py`), the command-line
interface (`benchmark_cli.py`), JSON and text parsers (`json_parsers.py`,
`text_parsers.py`), ground truth loading and matching (`ground_truth.py`,
`verification.py`), and reporting (`reports.py`). Each Python file in this implementation stays below 300 lines.

## Continuous Integration

The benchmark runs automatically on every pull request to `main` via GitHub Actions (`.github/workflows/benchmark.yml`).

### How It Works

1. **First Run**: If no `baseline_linux.json` exists, it generates one and uploads as artifact
2. **Subsequent Runs**: Compares current results against `baseline_linux.json`
3. **Regression Detection**: Fails the build if:
   - Time increases by >10% AND >1s absolute
   - Memory increases by >10% AND >5MB absolute
   - F1 Score decreases (any amount)

### Platform-Specific Baselines

| Platform | Baseline File |
| --- | --- |
| Windows | `benchmark/baseline_win32.json` |
| Linux/CI | `benchmark/baseline_linux.json` |

> **Note**: Performance varies significantly between platforms. Linux is generally faster. Always compare against the matching platform baseline.

## Results (Target: `benchmark/examples`)

### Ground Truth Summary

| Type | Count |
| --- | --- |
| Functions | 55 |
| Imports | 33 |
| Variables | 30 |
| Methods | 28 |
| Classes | 18 |
| **Total** | **164** |

---

## Overall Performance

| Tool | Time (s) | Mem (MB) | TP | FP | FN | Precision | Recall | F1 Score |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **CytoScnPy (Rust)** | **0.016** | **1.9** | **149** | 39 | **15** | **0.7926** | **0.9085** | **0.8466** |
| CytoScnPy (Python) | 0.030 | 34.0 | 149 | 39 | 15 | 0.7926 | 0.9085 | 0.8466 |
| Vulture (0%) | 0.038 | 17.0 | 113 | 55 | 51 | 0.6726 | 0.6890 | 0.6807 |
| Vulture (60%) | 0.040 | 16.9 | 113 | 55 | 51 | 0.6726 | 0.6890 | 0.6807 |
| deadcode | 0.063 | 21.3 | 109 | 50 | 55 | 0.6855 | 0.6646 | 0.6749 |
| Skylos | 3.229 | 905.7 | 101 | **36** | 63 | 0.7372 | 0.6159 | 0.6711 |
| uncalled | 0.026 | 14.7 | 63 | 24 | 101 | 0.7241 | 0.3841 | 0.5020 |
| Ruff | 0.026 | 11.4 | 37 | 16 | 127 | 0.6981 | 0.2256 | 0.3410 |
| Flake8 | 0.081 | 531.5 | 34 | 15 | 130 | 0.6939 | 0.2073 | 0.3192 |
| dead | 0.039 | 17.9 | 41 | 70 | 123 | 0.3694 | 0.2500 | 0.2982 |
| Pylint | 1.000 | 381.8 | 26 | 24 | 138 | 0.5200 | 0.1585 | 0.2430 |

The suite is small, so times mostly reflect process startup. CytoScnPy (Python)
runs the same Rust engine through the Python package and reports identical findings.

---

## Performance by Detection Type

### Class Detection (18 ground truth items)

| Tool | TP | FP | FN | Precision | Recall | F1 Score |
| --- | --- | --- | --- | --- | --- | --- |
| **CytoScnPy (Rust)** | **18** | **1** | **0** | **0.9474** | **1.0000** | **0.9730** |
| Vulture | 18 | 5 | 0 | 0.7826 | 1.0000 | 0.8780 |
| deadcode | 18 | 5 | 0 | 0.7826 | 1.0000 | 0.8780 |
| Skylos | 14 | 2 | 4 | 0.8750 | 0.7778 | 0.8235 |
| Flake8 | 0 | 0 | 18 | 0.0000 | 0.0000 | 0.0000 |
| Pylint | 0 | 0 | 18 | 0.0000 | 0.0000 | 0.0000 |
| Ruff | 0 | 0 | 18 | 0.0000 | 0.0000 | 0.0000 |
| uncalled | 0 | 0 | 18 | 0.0000 | 0.0000 | 0.0000 |
| dead | 0 | 0 | 18 | 0.0000 | 0.0000 | 0.0000 |

#### Analysis

| Tool | Explanation |
| --- | --- |
| **CytoScnPy** 🥇 | Finds all 18 unused classes with a single false positive. |
| **Vulture / deadcode** | Also find every unused class, but flag 5 live classes, typically ones used through inheritance or dynamic patterns. |
| **Skylos** | Fewer false positives than Vulture but misses 4 classes. |
| **Flake8 / Pylint / Ruff** | No unused-class rule; they only check imports and variables. |
| **uncalled / dead** | Function-focused tools with no class tracking. |

---

### Function Detection (55 ground truth items)

| Tool | TP | FP | FN | Precision | Recall | F1 Score |
| --- | --- | --- | --- | --- | --- | --- |
| **CytoScnPy (Rust)** | 50 | **16** | 5 | 0.7576 | 0.9091 | **0.8264** |
| Skylos | 48 | 20 | 7 | 0.7059 | 0.8727 | 0.7805 |
| Vulture | **51** | 26 | **4** | 0.6623 | **0.9273** | 0.7727 |
| deadcode | 51 | 26 | 4 | 0.6623 | 0.9273 | 0.7727 |
| dead | 32 | 2 | 23 | **0.9412** | 0.5818 | 0.7191 |
| uncalled | 43 | 24 | 12 | 0.6418 | 0.7818 | 0.7049 |
| Flake8 | 0 | 0 | 55 | 0.0000 | 0.0000 | 0.0000 |
| Pylint | 0 | 0 | 55 | 0.0000 | 0.0000 | 0.0000 |
| Ruff | 0 | 0 | 55 | 0.0000 | 0.0000 | 0.0000 |

#### Analysis

| Tool | Explanation |
| --- | --- |
| **CytoScnPy** 🥇 | Best balance: one fewer function found than Vulture, with 10 fewer false positives. |
| **Vulture / deadcode** | Highest recall (51/55), at the cost of 26 false positives. |
| **Skylos** | Close to CytoScnPy on recall, with more false positives. |
| **dead** | Very precise (2 false positives) but misses 23 functions. |
| **uncalled** | Designed for uncalled functions; moderate precision and recall. |
| **Flake8 / Pylint / Ruff** | No unused-function rule. |

---

### Import Detection (33 ground truth items)

| Tool | TP | FP | FN | Precision | Recall | F1 Score |
| --- | --- | --- | --- | --- | --- | --- |
| **CytoScnPy (Rust)** | **29** | 6 | **4** | 0.8286 | **0.8788** | **0.8529** |
| Skylos | 28 | 6 | 5 | 0.8235 | 0.8485 | 0.8358 |
| Ruff | 28 | 8 | 5 | 0.7778 | 0.8485 | 0.8116 |
| Flake8 | 27 | 9 | 6 | 0.7500 | 0.8182 | 0.7826 |
| deadcode | 12 | **0** | 21 | **1.0000** | 0.3636 | 0.5333 |
| Pylint | 16 | 12 | 17 | 0.5714 | 0.4848 | 0.5246 |
| Vulture | 11 | 1 | 22 | 0.9167 | 0.3333 | 0.4889 |
| uncalled | 0 | 0 | 33 | 0.0000 | 0.0000 | 0.0000 |
| dead | 0 | 0 | 33 | 0.0000 | 0.0000 | 0.0000 |

#### Analysis

| Tool | Explanation |
| --- | --- |
| **CytoScnPy** 🥇 | Highest recall and F1, with cross-file tracking of re-exports, `__all__`, and `TYPE_CHECKING` imports. |
| **Skylos / Ruff / Flake8** | Close behind; Ruff and Flake8 use the F401 rule. Their false positives are mostly imports used in type hints or re-exported. |
| **deadcode / Vulture** | Very precise but conservative: they miss most unused imports. |
| **Pylint** | W0611 (`unused-import`), with lower precision and recall here. |
| **uncalled / dead** | Do not analyze imports. |

---

### Method Detection (28 ground truth items)

| Tool | TP | FP | FN | Precision | Recall | F1 Score |
| --- | --- | --- | --- | --- | --- | --- |
| **CytoScnPy (Rust)** | **24** | 5 | **4** | 0.8276 | **0.8571** | **0.8421** |
| uncalled | 20 | **0** | 8 | **1.0000** | 0.7143 | 0.8333 |
| Vulture | 20 | 8 | 8 | 0.7143 | 0.7143 | 0.7143 |
| deadcode | 20 | 8 | 8 | 0.7143 | 0.7143 | 0.7143 |
| Skylos | 8 | 2 | 20 | 0.8000 | 0.2857 | 0.4211 |
| Flake8 | 0 | 0 | 28 | 0.0000 | 0.0000 | 0.0000 |
| Pylint | 0 | 0 | 28 | 0.0000 | 0.0000 | 0.0000 |
| Ruff | 0 | 0 | 28 | 0.0000 | 0.0000 | 0.0000 |
| dead | 0 | 0 | 28 | 0.0000 | 0.0000 | 0.0000 |

#### Analysis

| Tool | Explanation |
| --- | --- |
| **CytoScnPy** 🥇 | Highest recall and F1 for methods. |
| **uncalled** 🥈 | No false positives at all, but misses 8 methods. |
| **Vulture / deadcode** | Same results; false positives come from methods reached via `super()` or overrides. |
| **Skylos** | Precise but misses most unused methods in this suite. |
| **Flake8 / Pylint / Ruff / dead** | No method detection. |

> **Note:** Method detection is challenging because methods can be called via `self`, `super()`, inheritance, or dynamically via `getattr()`.

---

### Variable Detection (30 ground truth items)

| Tool | TP | FP | FN | Precision | Recall | F1 Score |
| --- | --- | --- | --- | --- | --- | --- |
| **CytoScnPy (Rust)** | **28** | 11 | **2** | **0.7179** | **0.9333** | **0.8116** |
| Vulture | 13 | 15 | 17 | 0.4643 | 0.4333 | 0.4483 |
| Pylint | 10 | 12 | 20 | 0.4545 | 0.3333 | 0.3846 |
| Ruff | 9 | 8 | 21 | 0.5294 | 0.3000 | 0.3830 |
| deadcode | 8 | 11 | 22 | 0.4211 | 0.2667 | 0.3265 |
| Flake8 | 7 | **6** | 23 | 0.5385 | 0.2333 | 0.3256 |
| dead | 9 | 68 | 21 | 0.1169 | 0.3000 | 0.1682 |
| Skylos | 3 | 6 | 27 | 0.3333 | 0.1000 | 0.1538 |
| uncalled | 0 | 0 | 30 | 0.0000 | 0.0000 | 0.0000 |

#### Analysis

| Tool | Explanation |
| --- | --- |
| **CytoScnPy** 🥇 | Finds 28 of 30 unused variables, including module-level and pattern-matched bindings. |
| **Vulture** | Flags more variables than the linters, with lower accuracy. |
| **Pylint / Ruff / Flake8** | W0612 / F841 cover local variables only, so global and pattern-matching cases are missed. |
| **deadcode / Skylos** | Variable detection is a low priority for both. |
| **dead** | Flags many live variables (68 false positives). |
| **uncalled** | No variable tracking. |

> **Note:** Variable detection is complex due to: pattern matching bindings, walrus operators (`:=`), comprehension variables, closure captures, and `nonlocal`/`global` declarations.

---

## Test Suite Overview

Each directory under `benchmark/examples/` has a `ground_truth.json`. Ground truth
is loaded recursively, so nested suites count too.

| Directory | What it covers |
| --- | --- |
| `cases/01_basic` … `cases/06_advanced` | Basic dead code, imports, dynamic access, decorators, Flask/FastAPI/Azure Functions entry points, pattern matching and complex scoping |
| `cases/issue_*` | Regression cases from reported issues |
| `complex/` | Complex logic, Halstead-heavy code, and a multi-module `sample_repo` |
| `dynamic/` | `getattr`/`globals()` access, metaclasses, metaprogramming |
| `frameworks/` | Framework entry points |
| `modern/` | ABCs, async, enums, protocols, type aliases |
| `quality/` | Basic dead code mixed with quality issues |
| `security/` | Dead code mixed with security-sensitive code |
| `data_science/`, `misc/` | Data-science style scripts and pragma comments |

---

## Key Findings

### Best Overall

- **CytoScnPy** leads with F1 0.85, the highest precision (0.79) and recall (0.91), and the lowest time and memory.
- **Vulture** and **deadcode** follow at F1 0.68 and 0.67.
- **Skylos** reaches F1 0.67 but is the slowest tool and uses the most memory on this suite.

### Best by Category

| Category | Best Tool | F1 Score | Runner-up |
| --- | --- | --- | --- |
| **Class** | CytoScnPy | 0.97 | Vulture / deadcode (0.88) |
| **Function** | CytoScnPy | 0.83 | Skylos (0.78) |
| **Import** | CytoScnPy | 0.85 | Skylos (0.84) |
| **Method** | CytoScnPy | 0.84 | uncalled (0.83) |
| **Variable** | CytoScnPy | 0.81 | Vulture (0.45) |

### Tool Categories

| Category | Tools | Strengths |
| --- | --- | --- |
| **Dead Code Analyzers** | CytoScnPy, Vulture, Skylos, deadcode | Full dead code detection (classes, functions, methods) |
| **Function Detectors** | uncalled, dead | Specialized for uncalled functions/methods |
| **Linters** | Ruff, Flake8, Pylint | Unused imports and local variables alongside style checks |

### Limitations

- **Small, curated suite**: 164 items cannot represent every codebase; results on your code will differ.
- **Method detection** requires tracking inheritance, `super()`, and dynamic dispatch
- **Variable detection** is limited by scoping complexity and pattern matching
- **Dynamic patterns** (`getattr`, `globals()`, `eval`) defeat all static analyzers

---

## 🔄 Alternative Tools (Not Yet Benchmarked)

The following tools could be considered for future benchmark additions. They offer different approaches or specialized capabilities for dead code detection.

### Static Analyzers

| Tool                                                           | Type               | Description                                                | Why Consider                                       |
| -------------------------------------------------------------- | ------------------ | ---------------------------------------------------------- | -------------------------------------------------- |
| **[Pyflakes](https://github.com/PyCQA/pyflakes)**              | Lightweight Linter | Fast, minimal static analysis focusing on errors not style | Faster than Flake8, focuses only on logical errors |
| **[Prospector](https://github.com/prospector-dev/prospector)** | Meta-Linter        | Aggregates Pylint, Pyflakes, Vulture in one tool           | All-in-one solution, highly customizable           |
| **[Fixit](https://github.com/Instagram/Fixit)**                | Auto-fixer         | Facebook's lint framework with auto-fix capabilities       | Can automatically remove detected dead code        |
| **[Semgrep](https://github.com/semgrep/semgrep)**              | Pattern Matcher    | Customizable pattern-based code analysis                   | User-defined dead code patterns, multi-language    |
| **[Bandit](https://github.com/PyCQA/bandit)**                  | Security Linter    | Security-focused analysis (includes some dead code)        | Catches security-related unused code patterns      |

### Dynamic Analyzers

| Tool                                                        | Type             | Description                             | Why Consider                     |
| ----------------------------------------------------------- | ---------------- | --------------------------------------- | -------------------------------- |
| **[Coverage.py](https://github.com/coveragepy/coveragepy)** | Runtime Coverage | Measures code execution during tests    | 100% accuracy for executed paths |
| **[Figleaf](https://github.com/ctb/figleaf)**               | Trace Analyzer   | Monitors code execution at runtime      | Fine-grained execution tracking  |
| **[py-spy](https://github.com/benfred/py-spy)**             | Profiler         | Sampling profiler showing executed code | Low overhead, production-safe    |

### Specialized Tools

| Tool                                                                     | Type            | Description                                      | Why Consider                                 |
| ------------------------------------------------------------------------ | --------------- | ------------------------------------------------ | -------------------------------------------- |
| **[autoflake](https://github.com/PyCQA/autoflake)**                      | Import Cleaner  | Removes unused imports & variables automatically | Auto-fix focused, integrates with pre-commit |
| **[unimport](https://github.com/hakancelikdev/unimport)**                | Import Analyzer | Specialized unused import detector               | More import patterns than Ruff/Flake8        |
| **[pycln](https://github.com/hadialqattan/pycln)**                       | Import Cleaner  | Formatter for import cleanup                     | Respects `__all__`, type-checking imports    |
| **[absolufy-imports](https://github.com/MarcoGorelli/absolufy-imports)** | Import Analyzer | Converts relative to absolute imports            | Helps with cross-module analysis             |

### IDE/Editor Extensions

| Tool                                                                                        | Platform    | Description                                                 |
| ------------------------------------------------------------------------------------------- | ----------- | ----------------------------------------------------------- |
| **[Pylance](https://marketplace.visualstudio.com/items?itemName=ms-python.vscode-pylance)** | VS Code     | Microsoft's Python language server with unused code graying |
| **[PyCharm Inspector](https://www.jetbrains.com/pycharm/)**                                 | PyCharm IDE | Built-in dead code detection with quick-fixes               |
| **[Sourcery](https://sourcery.ai/)**                                                        | Multiple    | AI-powered refactoring with dead code detection             |

### Why These Aren't Included Yet

| Reason                                       | Tools Affected                 |
| -------------------------------------------- | ------------------------------ |
| **Dynamic analysis** (requires running code) | Coverage.py, Figleaf, py-spy   |
| **Meta-tools** (wrap existing tools)         | Prospector, Deadcode Detective |
| **Different focus** (security, formatting)   | Bandit, autoflake, pycln       |
| **IDE-only** (not standalone CLI)            | Pylance, PyCharm               |
| **Commercial/SaaS**                          | Sourcery, DeepSource           |

> **💡 Tip**: For the most comprehensive dead code detection, combine a static analyzer (Vulture/CytoScnPy) with a dynamic analyzer (Coverage.py) and an import linter (Ruff/autoflake).

---

## ❓ Frequently Asked Questions (FAQ)

### About Benchmark Design

<details>
<summary><strong>Q: Why were these specific tools selected for the benchmark?</strong></summary>

The tools were selected to represent the full spectrum of dead code detection approaches:

| Category                              | Tools                      | Selection Rationale                                                               |
| ------------------------------------- | -------------------------- | --------------------------------------------------------------------------------- |
| **Dedicated Dead Code Analyzers**     | CytoScnPy, Vulture, Skylos | Purpose-built for comprehensive dead code detection (classes, functions, methods) |
| **Function-Specific Detectors**       | uncalled, dead             | Specialized tools that focus solely on uncalled function detection                |
| **General Linters with Import Rules** | Ruff, Flake8, Pylint       | Popular linters that include unused import detection as part of broader rule sets |

**Why these tools specifically?**

- **CytoScnPy**: The tool being benchmarked – Rust-based for speed
- **Vulture**: Most popular dedicated dead code finder in Python ecosystem
- **Skylos**: Modern alternative with AST-based analysis
- **Ruff**: Fastest linter, gaining rapid adoption
- **Flake8**: Industry standard for linting
- **Pylint**: Most comprehensive linter, long history
- **uncalled/dead**: Niche tools for specific use cases

</details>

<details>
<summary><strong>Q: Why use F1 Score as the primary metric?</strong></summary>

**F1 Score balances precision and recall**, which is critical for dead code detection:

```

F1 = 2 × (Precision × Recall) / (Precision + Recall)
```

| If you optimize only for...              | Problem                                               |
| ---------------------------------------- | ----------------------------------------------------- |
| **Precision** (minimize false positives) | You'll miss lots of actual dead code (low recall)     |
| **Recall** (find all dead code)          | You'll flag lots of live code as dead (low precision) |

**F1 forces tools to balance both**, making it the fairest single metric for comparison.

</details>

<details>
<summary><strong>Q: How was the ground truth dataset created?</strong></summary>

The ground truth contains **164 manually verified items** across the suites in `benchmark/examples/`:

1. **Manual Analysis**: Each test file was manually reviewed to identify genuinely unused code
2. **Cross-Validation**: Multiple reviewers verified the classifications
3. **Category Balance**: Intentional distribution across different code patterns:

   - 55 functions (34%)
   - 33 imports (20%)
   - 30 variables (18%)
   - 28 methods (17%)
   - 18 classes (11%)

4. **Edge Case Coverage**: Test suite includes challenging patterns:
   - Dynamic attribute access (`getattr`, `globals()`)
   - Metaprogramming (decorators, metaclasses)
   - Framework patterns (Flask/FastAPI routes)
   - Complex scoping (closures, nested functions)

</details>

<details>
<summary><strong>Q: Why separate baselines for Windows and Linux?</strong></summary>

Performance characteristics differ significantly between platforms:

| Factor                | Windows                            | Linux                   |
| --------------------- | ---------------------------------- | ----------------------- |
| **File system**       | NTFS (slower for many small files) | ext4 (generally faster) |
| **Process spawning**  | Slower subprocess creation         | Faster fork()           |
| **Typical execution** | ~40-60% slower                     | Baseline reference      |

Comparing Windows results to Linux baselines would cause false regression failures. Each platform has its own baseline for accurate comparison.

</details>

---

### About Tool Performance

<details>
<summary><strong>Q: Why is CytoScnPy so much faster than other tools?</strong></summary>

CytoScnPy is a compiled Rust program instead of a Python program:

| Factor | CytoScnPy (Rust) | Python-based Tools |
| --- | --- | --- |
| **Parser** | `ruff_python_parser` (compiled Rust) | Python `ast` module |
| **Startup** | Native binary, no interpreter | Python interpreter and imports |
| **Parallelization** | Native multi-threading (rayon) | Limited by the GIL |

On this suite it is about **200x faster** than Skylos and **2.4x faster** than Vulture. The suite is small, so much of the gap is startup time; the difference grows with codebase size.

</details>

<details>
<summary><strong>Q: Why do some tools have 0% detection for certain categories?</strong></summary>

Different tools have different design goals:

| Tool         | Why it misses categories                                                                               |
| ------------ | ------------------------------------------------------------------------------------------------------ |
| **Flake8**   | Style linter only – no dead code rules except F401 (imports)                                           |
| **Pylint**   | General linter – has `unused-import` and `unused-variable`, but no `unused-function` or `unused-class` |
| **Ruff**     | Implements Flake8 rules – same limitations                                                             |
| **uncalled** | Specifically designed for functions only – ignores everything else                                     |
| **dead**     | Function-focused call graph analyzer – no class/import tracking                                        |

This is by design, not a bug. Use tools appropriate for your detection needs.

</details>

<details>
<summary><strong>Q: Why does Vulture show the same results at 0% and 60% confidence?</strong></summary>

The benchmark test suite contains **genuinely unused code** with no ambiguity. Vulture's confidence levels filter out uncertain detections:

- At **0%**: Reports everything, including low-confidence items
- At **60%**: Only reports items with ≥60% confidence

In this benchmark, all unused code is clearly unused (not partially used or dynamically accessed), so confidence filtering doesn't change the results.

</details>

---

### About Using These Results

<details>
<summary><strong>Q: Which tool should I use for my project?</strong></summary>

| Your Priority             | Recommended Tool(s) | Why                                          |
| ------------------------- | ------------------- | -------------------------------------------- |
| **Speed (CI/CD)**         | CytoScnPy           | 0.016s execution, 1.9 MB peak memory         |
| **Accuracy**              | CytoScnPy           | Highest F1 score (0.85)                      |
| **Avoid false positives** | CytoScnPy, uncalled | Highest overall precision (0.79); uncalled has no false positives on methods |
| **Unused imports only**   | Ruff                | Mature F401 rule, very fast                  |
| **Second opinion**        | Vulture + Ruff      | Different strengths complement each other    |
| **Framework code**        | CytoScnPy           | Understands Flask, Django, FastAPI, Pydantic, and Azure Functions entry points |

</details>

<details>
<summary><strong>Q: Can I trust these results for production code?</strong></summary>

**With caveats:**

✅ **Trust for**:

- Relative performance comparisons
- Understanding tool capabilities
- General accuracy expectations

⚠️ **Be cautious because**:

- Benchmark uses a controlled test suite (164 items)
- Real codebases have different patterns
- Dynamic code (Django, SQLAlchemy) may have different results
- Your mileage may vary based on coding style

**Recommendation**: Run multiple tools on your actual codebase and manually verify suggestions before deleting code.

</details>

<details>
<summary><strong>Q: Why doesn't any tool reach 100% F1 Score?</strong></summary>

Dead code detection is a **fundamentally hard problem** due to:

1. **Dynamic Language Features**

   ```python
   getattr(obj, func_name)()  # Which function is called?
   globals()[var_name]  # Which variable is accessed?
```

2. **Framework Magic**

   ```python
   @app.route("/")  # Flask uses this, but AST can't know
   def home():
       pass
   ```

3. **Metaprogramming**

   ```python
   class Meta(type):
       def __new__(cls, name, bases, attrs):
           # Dynamically adds methods
   ```

4. **Cross-Module Analysis**

   ```python
   # file1.py
   def helper():
       pass  # Used in file2.py


   # file2.py
   from file1 import helper  # Static analyzers may miss this
   ```

No static analyzer can perfectly solve these problems without running the code.

</details>

---

### About Metrics

<details>
<summary><strong>Q: What do TP, FP, FN mean?</strong></summary>

| Metric                  | Meaning                        | Example                                                   |
| ----------------------- | ------------------------------ | --------------------------------------------------------- |
| **TP (True Positive)**  | Correctly identified dead code | Tool flags `unused_func`, and it really is unused ✅      |
| **FP (False Positive)** | Incorrectly flagged as dead    | Tool flags `used_func`, but it's actually used ❌         |
| **FN (False Negative)** | Missed dead code               | Tool didn't flag `dead_func`, but it's actually unused ❌ |

**Precision** = TP / (TP + FP) → "Of what I flagged, how much was correct?"
**Recall** = TP / (TP + FN) → "Of all dead code, how much did I find?"

</details>

<details>
<summary><strong>Q: How is memory usage measured?</strong></summary>

Memory is measured as **Peak Resident Set Size (RSS)** during tool execution:

- Captured using `psutil.Process().memory_info().rss`
- Measured at peak during analysis, not just at start/end
- Includes Python interpreter overhead for Python-based tools
- Rust-based tools (CytoScnPy) show lower memory due to efficient allocation

</details>

---

_Last updated: 2026-10-20 (164 total ground truth items, 9 tools in 11 configurations)_
