# Benchmark Report

---

This benchmark evaluates **11 dead code detection tools** against a curated Python test suite containing **131 ground truth items** across 6 categories. The goal is to measure detection accuracy (precision, recall, F1), performance (execution time, memory), and identify the best tools for different use cases.

### Key Highlights

| Metric                     | Winner           | Value           |
| -------------------------- | ---------------- | --------------- |
| 🚀 **Fastest Tool**        | CytoScnPy (Rust) | 0.04s           |
| 🎯 **Best Overall F1**     | deadcode         | 0.6667          |
| 🔬 **Highest Precision**   | uncalled         | 0.7564          |
| 💾 **Lowest Memory**       | CytoScnPy (Rust) | 9.7 MB          |
| ⚖️ **Best Speed/Accuracy** | CytoScnPy        | 0.04s @ F1 0.63 |

### Detection Capabilities at a Glance

| Tool      | Classes | Functions | Imports | Methods | Variables |
| --------- | :-----: | :-------: | :-----: | :-----: | :-------: |
| CytoScnPy |   ✅    |    ✅     |   ✅    |   ✅    |    ✅     |
| Vulture   |   ✅    |    ✅     |   ✅    |   ✅    |    ✅     |
| Skylos    |   ✅    |    ✅     |   ✅    |   ✅    |    ✅     |
| deadcode  |   ✅    |    ✅     |   ✅    |   ✅    |    ✅     |
| uncalled  |   ❌    |    ✅     |   ❌    |   ✅    |    ❌     |
| dead      |   ❌    |    ✅     |   ❌    |   ✅    |    ❌     |
| Ruff      |   ❌    |    ❌     |   ✅    |   ❌    |    ✅     |
| Flake8    |   ❌    |    ❌     |   ✅    |   ❌    |    ❌     |
| Pylint    |   ❌    |    ❌     |   ✅    |   ❌    |    ✅     |

### Quick Recommendations

- **For CI/CD Pipelines**: Use **CytoScnPy (Rust)** – fastest execution with reasonable accuracy
- **For Thorough Analysis**: Use **deadcode** – best overall F1 score across all categories
- **For Minimal False Positives**: Use **uncalled** – highest precision, conservative flagging
- **For Unused Imports Only**: Use **Ruff** – fastest import-focused linter with good accuracy
- **For Production Cleanup**: Combine **deadcode + Ruff** – comprehensive coverage with different strengths

---

## Running the Benchmark

```bash
# Activate environment
.\.venv\Scripts\activate  # Windows
source .venv/bin/activate  # Linux/Mac

# Check tool availability
python benchmark/benchmark_and_verify.py --check

# Run benchmark (Standard)
python benchmark/benchmark_and_verify.py

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

## Continuous Integration

The benchmark runs automatically on every push/PR to `main` via GitHub Actions (`.github/workflows/benchmark.yml`).

### How It Works

1. **First Run**: If no `baseline_linux.json` exists, it generates one and uploads as artifact
2. **Subsequent Runs**: Compares current results against `baseline_linux.json`
3. **Regression Detection**: Fails the build if:
   - Time increases by >10% AND >1s absolute
   - Memory increases by >10% AND >5MB absolute
   - F1 Score decreases (any amount)

### Platform-Specific Baselines

| Platform | Baseline File                   |
| -------- | ------------------------------- |
| Windows  | `benchmark/baseline_win32.json` |
| Linux/CI | `benchmark/baseline_linux.json` |

> **Note**: Performance varies significantly between platforms. Linux is generally faster. Always compare against the matching platform baseline.

## Results (Target: `benchmark/examples`)

### Ground Truth Summary

| Type      | Count   |
| --------- | ------- |
| Functions | 51      |
| Classes   | 14      |
| Methods   | 27      |
| Imports   | 19      |
| Variables | 20      |
| **Total** | **131** |

---

## Overall Performance

| Tool                 | Time (s) | Mem (MB) | TP     | FP     | FN     | Precision  | Recall     | F1 Score   |
| -------------------- | -------- | -------- | ------ | ------ | ------ | ---------- | ---------- | ---------- |
| **CytoScnPy (Rust)** | **0.04** | **9.7**  | **77** | **38** | **54** | **0.6696** | **0.5878** | **0.6260** |
| CytoScnPy (Python)   | 0.22     | 27.5     | 77     | 38     | 54     | 0.6696     | 0.5878     | 0.6260     |
| Skylos               | 1.31     | 64.7     | 64     | 29     | 67     | 0.6882     | 0.4885     | 0.5714     |
| Vulture (0%)         | 0.26     | 20.1     | 88     | 53     | 43     | 0.6241     | 0.6718     | 0.6471     |
| Vulture (60%)        | 0.22     | 20.1     | 88     | 53     | 43     | 0.6241     | 0.6718     | 0.6471     |
| Flake8               | 4.30     | 272.7    | 15     | 17     | 116    | 0.4688     | 0.1145     | 0.1840     |
| Pylint               | 12.78    | 400.9    | 17     | 18     | 114    | 0.4857     | 0.1298     | 0.2048     |
| Ruff                 | 0.31     | 38.0     | 24     | 20     | 107    | 0.5455     | 0.1832     | 0.2743     |
| uncalled             | 0.26     | 18.5     | 59     | 19     | 72     | 0.7564     | 0.4504     | 0.5646     |
| dead                 | 0.44     | 38.3     | 41     | 51     | 90     | 0.4457     | 0.3130     | 0.3677     |
| **deadcode**         | 0.35     | 29.1     | **90** | 49     | 41     | 0.6475     | **0.6870** | **0.6667** |

---

## Performance by Detection Type

### Class Detection (11 ground truth items)

| Tool             | TP  | FP  | FN  | Precision | Recall | F1 Score |
| ---------------- | --- | --- | --- | --------- | ------ | -------- |
| CytoScnPy (Rust) | 11  | 5   | 3   | 0.6875    | 0.7857 | 0.7333   |
| Vulture          | 11  | 8   | 3   | 0.5789    | 0.7857 | 0.6667   |
| Skylos           | 11  | 8   | 3   | 0.5789    | 0.7857 | 0.6667   |
| deadcode         | 11  | 8   | 3   | 0.5789    | 0.7857 | 0.6667   |
| Flake8           | 0   | 0   | 14  | 0.0000    | 0.0000 | 0.0000   |
| Pylint           | 0   | 0   | 14  | 0.0000    | 0.0000 | 0.0000   |
| Ruff             | 0   | 0   | 14  | 0.0000    | 0.0000 | 0.0000   |
| uncalled         | 0   | 0   | 14  | 0.0000    | 0.0000 | 0.0000   |
| dead             | 0   | 0   | 14  | 0.0000    | 0.0000 | 0.0000   |

#### Analysis

| Tool             | Explanation                                                                                                                                                       |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **CytoScnPy** 🥇 | Rust-based analyzer with best class detection precision. Finds all 11 unused classes with only 5 FP.                                                              |
| **Vulture**      | Specialized unused code finder. Achieves good recall on classes. 8 FP from classes it considers unused but are actually used via inheritance or dynamic patterns. |
| **Skylos**       | Purpose-built dead code detector with full class tracking. Same performance as Vulture.                                                                           |
| **deadcode**     | Enhanced Vulture alternative. Same class detection performance as Vulture and Skylos.                                                                             |
| **Flake8**       | Style linter only. Has no rules for unused class detection - only checks code style and unused imports (F401).                                                    |
| **Pylint**       | General linter. No `unused-class` rule exists. Only has `unused-import` (W0611), `unused-variable` (W0612), `unused-argument` (W0613).                            |
| **Ruff**         | Fast Flake8-compatible linter. Implements F401 (unused imports) and F841 (unused variables), but no class detection.                                              |
| **uncalled**     | Function-only detector. Specifically designed to find uncalled functions, not classes.                                                                            |
| **dead**         | Function-focused tool. Analyzes function call graphs only, no class instantiation tracking.                                                                       |

---

### Function Detection (50 ground truth items)

| Tool             | TP  | FP  | FN  | Precision | Recall | F1 Score |
| ---------------- | --- | --- | --- | --------- | ------ | -------- |
| Vulture          | 47  | 21  | 4   | 0.6912    | 0.9216 | 0.7899   |
| deadcode         | 47  | 21  | 4   | 0.6912    | 0.9216 | 0.7899   |
| uncalled         | 40  | 19  | 11  | 0.6780    | 0.7843 | 0.7273   |
| CytoScnPy (Rust) | 37  | 17  | 14  | 0.6852    | 0.7255 | 0.7048   |
| Skylos           | 29  | 6   | 22  | 0.8286    | 0.5686 | 0.6744   |
| dead             | 30  | 51  | 21  | 0.3704    | 0.5882 | 0.4545   |
| Flake8           | 0   | 0   | 51  | 0.0000    | 0.0000 | 0.0000   |
| Pylint           | 0   | 0   | 51  | 0.0000    | 0.0000 | 0.0000   |
| Ruff             | 0   | 0   | 51  | 0.0000    | 0.0000 | 0.0000   |

#### Analysis

| Tool                    | Explanation                                                                                                                                             |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Vulture/deadcode** 🥇 | Best balance of precision/recall. Finds 47/51 functions with acceptable FP rate. Uses AST analysis to track all function definitions and calls.         |
| **uncalled**            | Strong performer. Specifically designed for finding uncalled functions. Lower recall (78%) suggests it may respect some dynamic patterns or decorators. |
| **Skylos**              | Highest precision (83%) but lower recall. Conservative approach - prefers not flagging uncertain cases. Good for avoiding false alarms.                 |
| **CytoScnPy**           | Improved F1 (0.70) with good precision (69%) and recall (73%). Tracks return statements, **all** exports, and TYPE_CHECKING imports.                    |
| **dead**                | High FP (51). Uses AST walking but lacks context about dynamic usage, decorators, or framework patterns. Reports many live functions as dead.           |
| **Flake8**              | No function detection. Only implements style/import rules.                                                                                              |
| **Pylint**              | No `unused-function` rule in standard Pylint. Would need custom checker plugin.                                                                         |
| **Ruff**                | Implements Flake8 rules. No dead function detection in its rule set.                                                                                    |

---

### Import Detection (19 ground truth items)

| Tool             | TP  | FP  | FN  | Precision | Recall | F1 Score |
| ---------------- | --- | --- | --- | --------- | ------ | -------- |
| Ruff             | 16  | 16  | 3   | 0.5000    | 0.8421 | 0.6275   |
| Flake8           | 15  | 17  | 4   | 0.4688    | 0.7895 | 0.5882   |
| deadcode         | 8   | 5   | 11  | 0.6154    | 0.4211 | 0.5000   |
| Pylint           | 10  | 14  | 9   | 0.4167    | 0.5263 | 0.4651   |
| CytoScnPy (Rust) | 7   | 6   | 12  | 0.5385    | 0.3684 | 0.4375   |
| Vulture          | 6   | 5   | 13  | 0.5455    | 0.3158 | 0.4000   |
| Skylos           | 5   | 7   | 14  | 0.4167    | 0.2632 | 0.3226   |
| uncalled         | 0   | 0   | 19  | 0.0000    | 0.0000 | 0.0000   |
| dead             | 0   | 0   | 19  | 0.0000    | 0.0000 | 0.0000   |

#### Analysis

| Tool          | Explanation                                                                                                                                                    |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Ruff** 🥇   | Best import detector. Implements F401 (`imported but unused`). High recall (84%) catches most unused imports. FP from imports used in type hints or `__all__`. |
| **Flake8**    | Standard F401 implementation. Slightly lower recall than Ruff. Similar FP patterns - struggles with `TYPE_CHECKING` blocks and re-exports.                     |
| **deadcode**  | Good precision (62%) with moderate recall. Balances accuracy with coverage.                                                                                    |
| **Pylint**    | W0611 (`unused-import`). More conservative than Ruff/Flake8. Lower recall due to better handling of some edge cases, but misses more genuine unused imports.   |
| **CytoScnPy** | Cross-file import tracking. Lower recall suggests focus on obvious cases. Good precision - avoids flagging re-exported imports.                                |
| **Vulture**   | Import detection is secondary focus. Higher precision but lower recall - only flags clearly unused imports.                                                    |
| **Skylos**    | Similar to Vulture. Import detection not its primary strength. Conservative approach leads to many missed unused imports.                                      |
| **uncalled**  | Function-only tool. Does not analyze import statements at all.                                                                                                 |
| **dead**      | Function-focused. No import usage tracking implemented.                                                                                                        |

---

### Method Detection (27 ground truth items)

| Tool             | TP  | FP  | FN  | Precision | Recall | F1 Score |
| ---------------- | --- | --- | --- | --------- | ------ | -------- |
| uncalled         | 19  | 0   | 8   | 1.0000    | 0.7037 | 0.8261   |
| CytoScnPy (Rust) | 19  | 4   | 8   | 0.8261    | 0.7037 | 0.7600   |
| Vulture          | 19  | 5   | 8   | 0.7917    | 0.7037 | 0.7451   |
| deadcode         | 19  | 5   | 8   | 0.7917    | 0.7037 | 0.7451   |
| Skylos           | 16  | 4   | 11  | 0.8000    | 0.5926 | 0.6809   |
| dead             | 11  | 0   | 16  | 1.0000    | 0.4074 | 0.5789   |
| Flake8           | 0   | 0   | 27  | 0.0000    | 0.0000 | 0.0000   |
| Pylint           | 0   | 0   | 27  | 0.0000    | 0.0000 | 0.0000   |
| Ruff             | 0   | 0   | 27  | 0.0000    | 0.0000 | 0.0000   |

#### Analysis

| Tool            | Explanation                                                                                                                                                                                              |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **uncalled** 🥇 | Perfect precision! Every method it flags is genuinely unused. Reports methods as functions, correctly matched via type aliasing. Misses 8 methods (likely in complex inheritance or dynamically called). |
| **CytoScnPy**   | Strong performer with 19 detections. Good balance of precision and recall for method detection.                                                                                                          |
| **Vulture**     | Same performance as CytoScnPy. Reports "unused function" for methods. 5 FP likely from methods used via `super()` calls or overridden in subclasses.                                                     |
| **deadcode**    | Same method detection as Vulture and CytoScnPy. Good at finding unused methods.                                                                                                                          |
| **Skylos**      | Good detection with 4 FP. Similar to Vulture in approach. FP from methods it can't trace through inheritance chains.                                                                                     |
| **dead**        | Perfect precision but lowest recall (41%). Very conservative - only flags methods it's absolutely certain are unused.                                                                                    |
| **Flake8**      | No method detection. Style linter only.                                                                                                                                                                  |
| **Pylint**      | No `unused-method` rule exists. Would need custom implementation to track method calls.                                                                                                                  |
| **Ruff**        | No method detection rules implemented.                                                                                                                                                                   |

> **Note:** Method detection is challenging because methods can be called via `self`, `super()`, inheritance, or dynamically via `getattr()`. Tools with 100% precision prioritize avoiding false positives.

---

### Variable Detection (19 ground truth items)

| Tool             | TP  | FP  | FN  | Precision | Recall | F1 Score |
| ---------------- | --- | --- | --- | --------- | ------ | -------- |
| Ruff             | 8   | 4   | 12  | 0.6667    | 0.4000 | 0.5000   |
| Pylint           | 7   | 4   | 13  | 0.6364    | 0.3500 | 0.4516   |
| deadcode         | 5   | 10  | 15  | 0.3333    | 0.2500 | 0.2857   |
| Vulture          | 5   | 14  | 15  | 0.2632    | 0.2500 | 0.2564   |
| Skylos           | 3   | 4   | 17  | 0.4286    | 0.1500 | 0.2222   |
| CytoScnPy (Rust) | 3   | 6   | 17  | 0.3333    | 0.1500 | 0.2069   |
| Flake8           | 0   | 0   | 20  | 0.0000    | 0.0000 | 0.0000   |
| uncalled         | 0   | 0   | 20  | 0.0000    | 0.0000 | 0.0000   |
| dead             | 0   | 0   | 20  | 0.0000    | 0.0000 | 0.0000   |

#### Analysis

| Tool          | Explanation                                                                                                                                             |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Ruff** 🥇   | Best variable detector via F841 (`Local variable assigned but never used`). Good precision (67%). Misses global variables and pattern-matched bindings. |
| **Pylint**    | W0612 (`unused-variable`). Similar to Ruff. Slightly lower recall. Good at local scope but misses complex scoping patterns.                             |
| **deadcode**  | Detects 5 unused variables. Higher FP rate suggests aggressive flagging of potential dead code.                                                         |
| **Vulture**   | Same detection as deadcode. Flags more variables but with less accuracy. Struggles with variables used in comprehensions or as iteration targets.       |
| **Skylos**    | Lower variable detection priority. Conservative approach - only flags obvious cases.                                                                    |
| **CytoScnPy** | Variable detection is developing. Higher FP suggests aggressive flagging. Needs improvement in scope tracking.                                          |
| **Flake8**    | No built-in unused variable rule. Would need `flake8-unused-arguments` plugin.                                                                          |
| **uncalled**  | Function-only tool. No variable tracking implemented.                                                                                                   |
| **dead**      | Function-focused. Does not track variable assignments or usage.                                                                                         |

> **Note:** Variable detection is complex due to: pattern matching bindings, walrus operators (`:=`), comprehension variables, closure captures, and `nonlocal`/`global` declarations.

---

## Test Suite Overview

| Category             | Description                                          |
| -------------------- | ---------------------------------------------------- |
| `01_basic`           | Unused functions, classes, methods, nested functions |
| `02_imports`         | Unused imports, cross-module usage, package imports  |
| `03_dynamic`         | getattr/globals() dynamic access patterns            |
| `04_metaprogramming` | Decorator patterns                                   |
| `05_frameworks`      | Flask and FastAPI entry points                       |
| `06_advanced`        | Pattern matching, type hints, complex scoping        |

---

## Key Findings

### Best Overall

- **deadcode** leads with F1: 0.67 - excellent balance across all detection types
- **uncalled** highest precision (0.76) - best for minimizing false alarms
- **CytoScnPy** fastest (0.09s) with strong F1: 0.60 - best for CI/CD integration

### Best by Category

| Category     | Best Tool        | F1 Score | Why                              |
| ------------ | ---------------- | -------- | -------------------------------- |
| **Class**    | CytoScnPy        | 0.73     | Best precision with good recall  |
| **Function** | Vulture/deadcode | 0.79     | Best precision/recall balance    |
| **Import**   | Ruff             | 0.63     | Fast, mature F401 implementation |
| **Method**   | uncalled         | 0.83     | Perfect precision, good recall   |
| **Variable** | Ruff             | 0.50     | F841 rule with good precision    |

### Tool Categories

| Category                | Tools                                | Strengths                                              |
| ----------------------- | ------------------------------------ | ------------------------------------------------------ |
| **Dead Code Analyzers** | Vulture, Skylos, CytoScnPy, deadcode | Full dead code detection (classes, functions, methods) |
| **Function Detectors**  | uncalled, dead                       | Specialized for uncalled functions/methods             |
| **Import Linters**      | Ruff, Flake8, Pylint                 | Unused import detection with style checking            |

### Limitations

- **No tool achieves >82% F1** on any category - dead code detection remains challenging
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

The ground truth contains **126 manually verified items** across 6 test categories:

1. **Manual Analysis**: Each test file was manually reviewed to identify genuinely unused code
2. **Cross-Validation**: Multiple reviewers verified the classifications
3. **Category Balance**: Intentional distribution across different code patterns:

   - 50 functions (40%)
   - 27 methods (21%)
   - 19 imports (15%)
   - 19 variables (15%)
   - 11 classes (9%)

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

CytoScnPy uses a **Rust-based parser** instead of Python's AST module:

| Factor              | CytoScnPy (Rust)       | Python-based Tools         |
| ------------------- | ---------------------- | -------------------------- |
| **Parser**          | tree-sitter (compiled) | Python AST (interpreted)   |
| **Memory model**    | Zero-copy parsing      | Object allocation per node |
| **Parallelization** | Native multi-threading | GIL limitations            |

Result: **20x faster** than Skylos, **4x faster** than Vulture for the same test suite.

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
| **Speed (CI/CD)**         | CytoScnPy           | 0.07s execution, minimal memory              |
| **Accuracy**              | Vulture             | Highest F1 score (0.68)                      |
| **Avoid false positives** | Skylos              | Highest precision (0.73)                     |
| **Unused imports only**   | Ruff                | Best import detection, blazing fast          |
| **Comprehensive check**   | Vulture + Ruff      | Different strengths complement each other    |
| **Framework code**        | Skylos              | Better at respecting decorators/entry points |

</details>

<details>
<summary><strong>Q: Can I trust these results for production code?</strong></summary>

**With caveats:**

✅ **Trust for**:

- Relative performance comparisons
- Understanding tool capabilities
- General accuracy expectations

⚠️ **Be cautious because**:

- Benchmark uses a controlled test suite (126 items)
- Real codebases have different patterns
- Dynamic code (Django, SQLAlchemy) may have different results
- Your mileage may vary based on coding style

**Recommendation**: Run multiple tools on your actual codebase and manually verify suggestions before deleting code.

</details>

<details>
<summary><strong>Q: Why doesn't any tool achieve >82% F1 Score?</strong></summary>

Dead code detection is a **fundamentally hard problem** due to:

1. **Dynamic Language Features**

   ```python
   getattr(obj, func_name)()  # Which function is called?
   globals()[var_name]        # Which variable is accessed?
   ```

2. **Framework Magic**

   ```python
   @app.route("/")           # Flask uses this, but AST can't know
   def home(): pass
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
   def helper(): pass  # Used in file2.py

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

_Last updated: 2025-12-12 (131 total ground truth items, 11 tools benchmarked)_
