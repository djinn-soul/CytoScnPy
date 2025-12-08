# Bugs Found During Benchmarking

## 2. False Positives (Tool Errors)

The tool flagged several classes as "unused" that are actually **USED**. These are genuine bugs in the tool's reference counting logic:

| Class          | File                         | Usage Pattern                       | Root Cause                              |
| :------------- | :--------------------------- | :---------------------------------- | :-------------------------------------- |
| `Meta`         | `dynamic/metaprogramming.py` | `class MyClass(metaclass=Meta):`    | Tool misses usage in `metaclass` kwarg. |
| `AsyncContext` | `modern/async_code.py`       | `async with AsyncContext() as ctx:` | Tool misses usage in `async with`.      |
| `AsyncIter`    | `modern/async_code.py`       | `async for item in AsyncIter():`    | Tool misses usage in `async for`.       |

These findings were correctly excluded from Ground Truth (because they are used), so the tool reporting them is a **Quality Issue** (False Positive).

## 3. False Negatives (Tool Misses)

The 9 False Negatives are genuine misses by the tool.

- **`Order` class**: Failed to be detected as unused. Likely due to global name collisions or inheritance handling issues.
- **Basic Test Cases**: Multiple `UnusedClass` definitions in `examples/cases` were missed.

## 4. Bug: Methods Reported as Functions

**Severity:** High
**Impact:**

- Inaccurate metrics for Function Detection (inflated FP/TP).
- 0 True Positives for Method Detection.
- Confusion in report categorization.

**Description:**
The `CytoScnPy` analyzer currently groups both `function` and `method` definition types into the `unused_functions` vector. This causes methods to be reported as "Unused Functions" in the JSON output and CLI report. Consequently, the "Methods" category in the report is either missing or empty.

**Fix Instructions:**

1.  **Modify `analyzer.rs`:**
    - Update `AnalysisResult` struct to include `pub unused_methods: Vec<Definition>`.
    - In `CytoScnPy::analyze` and `CytoScnPy::analyze_code`:
      - Initialize `unused_methods` vector.
      - Update the aggregation loop to separate `function` to `unused_functions` and `method` to `unused_methods`.
      - Include `unused_methods` in the returned `AnalysisResult`.
2.  **Modify `output.rs`:**
    - Update `print_summary_pills` to include a pill for "Methods".
    - Update `print_report` to call `print_unused_items` for `unused_methods`.
3.  **Verify:**
    - Run `cargo build` to ensure all struct usages are updated.
    - Run `benchmark/verify_functions.py` and a new `verify_methods.py` to confirm correct categorization.
