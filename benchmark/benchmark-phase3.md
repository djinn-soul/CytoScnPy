# CytoScnPy Benchmark Results - After Phase 3

**Date**: 2025-12-07  
**Phase**: Phase 3 (LineIndex Optimization)

## Changes Applied

- [x] Optimized `LineIndex::new` to use `bytes().enumerate()` instead of `char_indices()`
- [x] Analyzer refactored into 4 modular files

---

## Benchmark Results (10 runs, 3 warmup)

| Metric   | Value                 |
| -------- | --------------------- |
| **Mean** | **2.357 s** ± 0.471 s |
| **Min**  | 1.669 s               |
| **Max**  | 3.027 s               |

---

## Full Progress Summary

| Stage                      | Time        | Improvement |
| -------------------------- | ----------- | ----------- |
| Baseline                   | 5.223 s     | -           |
| Phase 1 (LTO + FxHashMap)  | 4.044 s     | 22.6%       |
| Phase 2 (Reference counts) | 3.059 s     | 41.4%       |
| **Phase 3 (LineIndex)**    | **2.357 s** | **54.9%**   |

---

## Implementation Summary

- **Phase 1**: `[profile.release]` optimizations + `rustc-hash` FxHashMap
- **Phase 2**: Changed `references` from `Vec<(String, PathBuf)>` to `FxHashMap<String, usize>`
- **Phase 3**: `LineIndex::new` uses byte iteration (faster than Unicode iteration)
- **Refactor**: Split `analyzer.rs` (1100+ lines) into 4 modular files
