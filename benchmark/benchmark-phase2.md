# CytoScnPy Benchmark Results - After Phase 2

**Date**: 2025-12-07  
**Phase**: Phase 2 (Reference Counting Optimization)

## Changes Applied

- [x] Changed `references` from `Vec<(String, PathBuf)>` to `FxHashMap<String, usize>`
- [x] Eliminated wasteful PathBuf cloning per reference (was ~100K+ clones on Django)
- [x] Merged counts directly during aggregation

---

## Benchmark Comparison

| Stage        | Mean        | Std Dev   | Improvement      |
| ------------ | ----------- | --------- | ---------------- |
| **Baseline** | 5.223 s     | ± 0.708 s | -                |
| **Phase 1**  | 4.044 s     | ± 0.236 s | 22.6% faster     |
| **Phase 2**  | **3.059 s** | ± 0.148 s | **41.4% faster** |

---

## Memory Impact

- Before: Every `add_ref()` call cloned a PathBuf (~50+ bytes each)
- After: Just increment a counter (8 bytes per unique name)
- Estimated memory reduction: **60-80%** for references data structure

## Next Steps

1. Refactor analyzer.rs into logical modules
2. Continue Phase 3 optimizations (rule instantiation, Config ref)
