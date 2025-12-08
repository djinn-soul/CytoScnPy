# Bugs Found During Benchmarking

## 2. False Positives (Tool Errors)

The tool flagged several classes as "unused" that are actually **USED**. These are genuine bugs in the tool's reference counting logic:

| Class          | File                         | Usage Pattern                       | Root Cause                              |
| :------------- | :--------------------------- | :---------------------------------- | :-------------------------------------- |
| `Meta`         | `dynamic/metaprogramming.py` | `class MyClass(metaclass=Meta):`    | Tool misses usage in `metaclass` kwarg. |
| `AsyncContext` | `modern/async_code.py`       | `async with AsyncContext() as ctx:` | Tool misses usage in `async with`.      |
| `AsyncIter`    | `modern/async_code.py`       | `async for item in AsyncIter():`    | Tool misses usage in `async for`.       |

These findings were correctly excluded from Ground Truth (because they are used), so the tool reporting them is a **Quality Issue** (False Positive).

**Root Cause:** The `ClassDef` handler in `visitor.rs` does not visit the `node.keywords` field where `metaclass=` is specified.

**Fix:** See Bug #5 below.

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

## 5. Bug: Metaclass Keyword Argument Not Visited

**Severity:** High  
**Impact:**

- Classes used as metaclasses are incorrectly flagged as unused
- False positive for `Meta` in `dynamic/metaprogramming.py`
- Breaks detection for any class used via `class X(metaclass=Y)` pattern

**Description:**
The `ClassDef` handler in `visitor.rs` visits decorators and base classes but does NOT visit the `keywords` field. This field contains keyword arguments like `metaclass=SomeClass`, which means the metaclass is never tracked as \"used\".

**Location:** `cytoscnpy/src/visitor.rs`, lines 367-443 (ClassDef handler)

**Fix Instructions:**

1.  **Modify `visitor.rs`** in the `Stmt::ClassDef(node)` handler:
    - After line 424 (after visiting base classes), add:
      ```rust
      // Visit keyword arguments (e.g., metaclass=SomeClass)
      for keyword in &amp;node.keywords {
          self.visit_expr(&amp;keyword.value);
      }
      ```  

2.  **Verify:**
    - Test with: `class MyClass(metaclass=Meta): pass`
    - Ensure `Meta` is not flagged as unused
    - Run benchmark test: `benchmark-projects/dynamic/metaprogramming.py`


## 6. Minor TODOs in Commands Module

**Severity:** Low  

**Location:** `cytoscnpy/src/commands.rs`

1. **Line 180**: `no_assert` parameter not passed to `analyze_complexity`
   - CLI accepts `--no-assert` flag but doesn't use it
   - Comment says: \"TODO: Pass no_assert to analyze_complexity if implemented\"

2. **Line 463**: `multi` flag for MI comment counting not implemented
   - CLI accepts `-m/--multi` flag but doesn't use it
   - Comment says: \"TODO: Use 'multi' flag to adjust comment counting if needed\"

**Impact:** CLI flags are accepted but ignored - no functional impact, but misleading to users.

**Fix:** Either implement the functionality or remove the unused CLI arguments.

