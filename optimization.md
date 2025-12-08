# CytoScnPy Performance Optimization Report

**Status**: ✅ Implementation Complete - 54.9% Performance Improvement  
**Date**: 2025-12-07

---

## 📊 Implementation Status

### ✅ Completed Optimizations

| #   | Optimization                                  | Impact          | Files                                        |
| --- | --------------------------------------------- | --------------- | -------------------------------------------- |
| 1   | Cargo.toml release profile (LTO, opt-level 3) | ~15%            | `Cargo.toml`                                 |
| 2   | FxHashMap/FxHashSet (faster hashing)          | ~10%            | `visitor.rs`, `analyzer/`                    |
| 3   | Reference counting (Vec→HashMap)              | ~20%            | `visitor.rs`, `processing.rs`                |
| 4   | LineIndex byte iteration                      | ~5%             | `utils.rs`                                   |
| 5   | Analyzer module refactor                      | Maintainability | `analyzer/`                                  |
| 6   | lazy_static → OnceLock                        | Cleaner code    | `constants.rs`, `framework.rs`, `secrets.rs` |

### ⏸️ Deferred (Requires Significant Refactor)

| #   | Optimization                  | Reason                                        |
| --- | ----------------------------- | --------------------------------------------- |
| 7   | Rule instantiation with Arc   | Requires `Rule` trait redesign                |
| 8   | Config by reference           | Requires lifetime propagation through Context |
| 9   | Arc<str>/Cow<str> for strings | Invasive changes across visitor               |


---

## 📈 Benchmark Results

| Stage                              | Time        | Improvement |
| ---------------------------------- | ----------- | ----------- |
| Baseline                           | 5.223 s     | -           |
| Phase 1 (LTO + FxHashMap)          | 4.044 s     | 22.6%       |
| Phase 2 (Reference counts)         | 3.059 s     | 41.4%       |
| **Phase 3 (LineIndex + OnceLock)** | **2.357 s** | **54.9%**   |

---

## 🔧 Technical Details

### 1. Cargo.toml Release Profile ✅

```toml
[profile.release]
lto = "thin"           # Link-time optimization
codegen-units = 1      # Better optimization
opt-level = 3          # Maximum optimization
strip = true           # Strip debug symbols
```

### 2. FxHashMap ✅

Replace `std::collections::HashMap` with faster non-cryptographic hashing:

```rust
use rustc_hash::{FxHashMap, FxHashSet};
```

### 3. Reference Counting ✅

**Before:** `Vec<(String, PathBuf)>` - PathBuf never used!  
**After:** `FxHashMap<String, usize>` - Direct counting

### 4. LineIndex Byte Iteration ✅

**Before:**

```rust
for (i, ch) in source.char_indices() { ... }
```

**After:**

```rust
for (i, byte) in source.as_bytes().iter().enumerate() { ... }
```

### 5. Analyzer Refactor ✅

Split 1100+ line file into modular structure:

- `analyzer/mod.rs` - Struct + builders (168 lines)
- `analyzer/types.rs` - Result types (68 lines)
- `analyzer/heuristics.rs` - Confidence adjustments (109 lines)
- `analyzer/processing.rs` - Core processing (860 lines)

### 6. OnceLock Migration ✅

**Before:**

```rust
lazy_static::lazy_static! {
    static ref PATTERNS: Vec<Pattern> = ...;
}
```

**After:**

```rust
fn get_patterns() -> &'static Vec<Pattern> {
    static PATTERNS: OnceLock<Vec<Pattern>> = OnceLock::new();
    PATTERNS.get_or_init(|| vec![...])
}
```

---

## 📝 Deferred Optimizations (Future Work)

### Rule Instantiation with Arc

Currently rules are instantiated per-file. Could share using Arc:

```rust
let danger_rules = Arc::new(get_danger_rules());
// In par_iter: rules.extend(danger_rules.iter().map(|r| r.clone_box()));
```

**Blocked:** Requires `Rule` trait to implement `CloneBox`.

### Config by Reference

Currently Config is cloned per-file:

```rust
LinterVisitor::new(rules, filename, line_index, self.config.clone());
```

**Blocked:** Requires lifetime parameters through Context struct.

### Arc<str>/Cow<str> for Strings

Reduce string cloning in hot paths by using:

- `Arc<str>` for shared strings (file paths, module names)
- `Cow<str>` for sometimes-borrowed strings

**Blocked:** Requires changes to Definition struct, visitor, and all callers (~20-30 files).
