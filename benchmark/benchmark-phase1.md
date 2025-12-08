# CytoScnPy Benchmark Results - After Phase 1

**Date**: 2025-12-07  
**Phase**: Phase 1 (Cargo.toml + FxHashMap)

## Changes Applied

- [x] Cargo.toml release profile (LTO, codegen-units=1, strip=true)
- [x] rustc-hash v2.1.1
- [x] FxHashMap in analyzer.rs and visitor.rs

---

## Combined Benchmark (All Projects)

| Stage        | Mean    | Std Dev   | Min     | Max     | Improvement      |
| ------------ | ------- | --------- | ------- | ------- | ---------------- |
| **Baseline** | 5.223 s | ± 0.708 s | 4.712 s | 6.032 s | -                |
| **Phase 1**  | 4.044 s | ± 0.236 s | 3.843 s | 4.305 s | **22.6% faster** |

---

## Next: Phase 2 (Memory Optimizations)

Expected additional improvement: 15-25%
