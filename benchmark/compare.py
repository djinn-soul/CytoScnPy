#!/usr/bin/env python3
"""
Compare two versions of CytoScnPy for performance regression testing.
Usage: python compare.py <baseline-binary> <optimized-binary>
"""

import json
import subprocess
import time
import sys
from pathlib import Path
from dataclasses import dataclass
from typing import List

@dataclass
class ComparisonResult:
    dataset: str
    files: int
    baseline_time: float
    optimized_time: float
    improvement_percent: float
    status: str

def run_analysis(binary: Path, dataset: Path) -> tuple[float, int]:
    """Run analysis and return (time, files)."""
    start = time.perf_counter()
    result = subprocess.run(
        [str(binary), "analyze", str(dataset), "--json"],
        capture_output=True,
        text=True,
        timeout=300
    )
    elapsed = time.perf_counter() - start
    
    files = 0
    try:
        if result.stdout:
            data = json.loads(result.stdout)
            files = data.get("analysis_summary", {}).get("total_files", 0)
    except:
        pass
    
    return elapsed, files

def compare_binaries(baseline: Path, optimized: Path, datasets_dir: Path) -> List[ComparisonResult]:
    """Compare two binaries across all datasets."""
    results = []
    
    for dataset in sorted(datasets_dir.iterdir()):
        if not dataset.is_dir():
            continue
        
        print(f"Testing {dataset.name}...")
        
        # Run each 3 times, take median
        baseline_times = []
        optimized_times = []
        files = 0
        
        for i in range(3):
            bt, f = run_analysis(baseline, dataset)
            ot, _ = run_analysis(optimized, dataset)
            baseline_times.append(bt)
            optimized_times.append(ot)
            files = f
        
        baseline_times.sort()
        optimized_times.sort()
        
        baseline_median = baseline_times[1]
        optimized_median = optimized_times[1]
        
        improvement = ((baseline_median - optimized_median) / baseline_median) * 100
        
        status = "🚀" if improvement > 5 else ("⚠️" if improvement < -5 else "➡️")
        
        results.append(ComparisonResult(
            dataset=dataset.name,
            files=files,
            baseline_time=baseline_median,
            optimized_time=optimized_median,
            improvement_percent=improvement,
            status=status
        ))
    
    return results

def print_comparison_table(results: List[ComparisonResult]):
    """Print comparison as markdown table."""
    print("\n## Performance Comparison\n")
    print("| Dataset | Files | Baseline | Optimized | Change | Status |")
    print("|---------|-------|----------|-----------|--------|--------|")
    
    for r in results:
        print(f"| {r.dataset} | {r.files:,} | {r.baseline_time:.3f}s | "
              f"{r.optimized_time:.3f}s | {r.improvement_percent:+.1f}% | {r.status} |")
    
    # Overall
    total_baseline = sum(r.baseline_time for r in results)
    total_optimized = sum(r.optimized_time for r in results)
    overall_improvement = ((total_baseline - total_optimized) / total_baseline) * 100
    
    print(f"\n**Overall:** {total_baseline:.2f}s → {total_optimized:.2f}s "
          f"({overall_improvement:+.1f}%)")

def main():
    if len(sys.argv) != 3:
        print("Usage: python compare.py <baseline-binary> <optimized-binary>")
        print("Example: python compare.py baseline.exe optimized.exe")
        sys.exit(1)
    
    baseline = Path(sys.argv[1])
    optimized = Path(sys.argv[2])
    datasets_dir = Path("benchmark/datasets")
    
    if not baseline.exists():
        print(f"Error: Baseline binary not found: {baseline}")
        sys.exit(1)
    
    if not optimized.exists():
        print(f"Error: Optimized binary not found: {optimized}")
        sys.exit(1)
    
    print("=" * 60)
    print("CytoScnPy Performance Comparison")
    print("=" * 60)
    print(f"Baseline:  {baseline}")
    print(f"Optimized: {optimized}")
    print()
    
    results = compare_binaries(baseline, optimized, datasets_dir)
    print_comparison_table(results)

if __name__ == "__main__":
    main()
