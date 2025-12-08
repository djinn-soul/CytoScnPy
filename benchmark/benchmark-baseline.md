# CytoScnPy Baseline Benchmark Results

**Date**: 2025-12-07  
**Platform**: Windows  
**Tool**: hyperfine v1.20.0  
**Binary**: `target/release/cytoscnpy-bin.exe` (no optimizations yet)

---

## Combined Benchmark (All Projects)

| Command                      | Mean        | Std Dev   | Min     | Max     |
| ---------------------------- | ----------- | --------- | ------- | ------- |
| `analyze benchmark-projects` | **5.223 s** | ± 0.708 s | 4.712 s | 6.032 s |

---

## Individual Project Benchmarks

| Project      |              Mean |       Min |       Max | Relative |
| :----------- | ----------------: | --------: | --------: | -------: |
| **Requests** |    148.0 ms ± 6.8 |  140.8 ms |  154.3 ms |    1.00× |
| **Flask**    |    165.5 ms ± 5.2 |  159.9 ms |  170.1 ms |    1.12× |
| **Rich**     |   317.9 ms ± 19.3 |  305.9 ms |  340.2 ms |    2.15× |
| **FastAPI**  |   744.2 ms ± 20.6 |  724.2 ms |  765.4 ms |    5.03× |
| **Django**   | 3634.5 ms ± 121.7 | 3558.2 ms | 3774.8 ms |   24.56× |

---

## Re-run Command

```powershell
# Combined (all projects at once)
hyperfine --warmup 1 --runs 3 "target\release\cytoscnpy-bin.exe analyze benchmark-projects"

# Individual projects comparison
hyperfine --warmup 1 --runs 3 `
  "target\release\cytoscnpy-bin.exe analyze benchmark-projects\requests" `
  "target\release\cytoscnpy-bin.exe analyze benchmark-projects\flask" `
  "target\release\cytoscnpy-bin.exe analyze benchmark-projects\rich" `
  "target\release\cytoscnpy-bin.exe analyze benchmark-projects\fastapi" `
  "target\release\cytoscnpy-bin.exe analyze benchmark-projects\django"
```

---

## Expected After Optimizations

| Phase                    | Combined Target | Improvement |
| ------------------------ | --------------- | ----------- |
| Phase 1 (Cargo + FxHash) | ~4.4 s          | 15-20%      |
| Phase 2 (Memory opts)    | ~3.4 s          | 35-50%      |
| Phase 3 (Cloning)        | ~3.1 s          | 40-60%      |
