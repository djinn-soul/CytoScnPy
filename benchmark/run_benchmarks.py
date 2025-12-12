#!/usr/bin/env python3
"""
CytoScnPy Benchmark Suite using Hyperfine
Provides accurate measurements with warmup, statistics, and outlier detection.
"""

import json
import subprocess
import sys
import os
from pathlib import Path
from dataclasses import dataclass
from typing import List, Optional
import platform

@dataclass
class BenchmarkResult:
    dataset: str
    files: int
    lines: int
    mean_seconds: float
    stddev_seconds: float
    min_seconds: float
    max_seconds: float
    runs: int

def get_cytoscnpy_binary() -> Path:
    """Find the cytoscnpy binary."""
    possible_paths = [
        Path("target/release/cytoscnpy-bin.exe"),
        Path("target/release/cytoscnpy-bin"),
        Path("target/debug/cytoscnpy-bin.exe"),
        Path("target/debug/cytoscnpy-bin"),
    ]
    for p in possible_paths:
        if p.exists():
            return p
    return Path("cytoscnpy")

def get_file_stats(binary: Path, dataset: Path) -> tuple[int, int]:
    """Get file count and line count from a quick run."""
    result = subprocess.run(
        [str(binary), "analyze", str(dataset), "--json"],
        capture_output=True, text=True, timeout=300
    )
    files, lines = 0, 0
    try:
        if result.stdout:
            data = json.loads(result.stdout)
            summary = data.get("analysis_summary", {})
            files = summary.get("total_files", 0)
            lines = summary.get("total_lines_analyzed", 0)
    except:
        pass
    return files, lines

def run_hyperfine_benchmark(binary: Path, dataset: Path, runs: int = 10, warmup: int = 3) -> Optional[dict]:
    """Run hyperfine and return results."""
    name = dataset.name
    json_output = Path(f"benchmark/results_{name}.json")
    
    cmd = [
        "hyperfine",
        "--warmup", str(warmup),
        "--runs", str(runs),
        "--export-json", str(json_output),
        f"{binary} analyze {dataset} --json"
    ]
    
    print(f"\n=== {name} ===")
    result = subprocess.run(cmd, capture_output=False)
    
    if result.returncode == 0 and json_output.exists():
        with open(json_output) as f:
            return json.load(f)
    return None

def run_benchmark_suite(datasets_dir: Path, runs: int = 10, warmup: int = 3) -> List[BenchmarkResult]:
    """Run benchmarks on all datasets using hyperfine."""
    binary = get_cytoscnpy_binary()
    print(f"Using binary: {binary}")
    print(f"Runs per dataset: {runs}, Warmup: {warmup}")
    
    results = []
    datasets = sorted([d for d in datasets_dir.iterdir() if d.is_dir()])
    
    for dataset in datasets:
        # Get file/line counts
        files, lines = get_file_stats(binary, dataset)
        
        # Run hyperfine
        hf_result = run_hyperfine_benchmark(binary, dataset, runs, warmup)
        
        if hf_result and "results" in hf_result:
            r = hf_result["results"][0]
            results.append(BenchmarkResult(
                dataset=dataset.name,
                files=files,
                lines=lines,
                mean_seconds=r.get("mean", 0),
                stddev_seconds=r.get("stddev", 0),
                min_seconds=r.get("min", 0),
                max_seconds=r.get("max", 0),
                runs=runs
            ))
    
    return results

def run_comparison(datasets_dir: Path, binary: Path):
    """Run comparison of all datasets at once."""
    print("\n" + "=" * 60)
    print("Comparative Benchmark (All Datasets)")
    print("=" * 60)
    
    cmd = ["hyperfine", "--warmup", "2", "--runs", "5"]
    
    for dataset in sorted(datasets_dir.iterdir()):
        if dataset.is_dir():
            cmd.extend(["--command-name", dataset.name])
            cmd.append(f"{binary} analyze {dataset} --json")
    
    cmd.extend([
        "--export-markdown", "benchmark/comparison.md",
        "--export-json", "benchmark/comparison.json"
    ])
    
    subprocess.run(cmd)

def print_results_table(results: List[BenchmarkResult]):
    """Print results as markdown table."""
    print("\n## Benchmark Results (Hyperfine)\n")
    print("| Dataset | Files | Lines | Mean (s) | Stddev | Min | Max |")
    print("|---------|-------|-------|----------|--------|-----|-----|")
    
    for r in results:
        print(f"| {r.dataset} | {r.files:,} | {r.lines:,} | "
              f"{r.mean_seconds:.3f} | {r.stddev_seconds:.3f} | "
              f"{r.min_seconds:.3f} | {r.max_seconds:.3f} |")
    
    total_files = sum(r.files for r in results)
    total_lines = sum(r.lines for r in results)
    total_mean = sum(r.mean_seconds for r in results)
    
    print(f"\n**Total:** {total_files:,} files, {total_lines:,} lines")
    print(f"**Combined mean time:** {total_mean:.2f}s")
    if total_mean > 0:
        print(f"**Throughput:** {total_lines / total_mean:,.0f} lines/second")

def save_results(results: List[BenchmarkResult], output_file: Path):
    """Save results to JSON."""
    import time
    data = {
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "platform": platform.platform(),
        "tool": "hyperfine",
        "results": [
            {
                "dataset": r.dataset,
                "files": r.files,
                "lines": r.lines,
                "mean_seconds": r.mean_seconds,
                "stddev_seconds": r.stddev_seconds,
                "min_seconds": r.min_seconds,
                "max_seconds": r.max_seconds,
                "runs": r.runs
            }
            for r in results
        ]
    }
    with open(output_file, "w") as f:
        json.dump(data, f, indent=2)
    print(f"\nResults saved to: {output_file}")

def main():
    script_dir = Path(__file__).parent
    os.chdir(script_dir.parent)
    
    datasets_dir = Path("benchmark/datasets")
    
    if not datasets_dir.exists():
        print("Error: benchmark/datasets directory not found")
        sys.exit(1)
    
    # Check hyperfine is available
    try:
        subprocess.run(["hyperfine", "--version"], capture_output=True, check=True)
    except (subprocess.CalledProcessError, FileNotFoundError):
        print("Error: hyperfine not found. Install with:")
        print("  Windows: scoop install hyperfine")
        print("  macOS:   brew install hyperfine")
        print("  Linux:   cargo install hyperfine")
        sys.exit(1)
    
    print("=" * 60)
    print("CytoScnPy Benchmark Suite (Hyperfine)")
    print("=" * 60)
    
    results = run_benchmark_suite(datasets_dir, runs=20, warmup=3)
    print_results_table(results)
    
    output_file = Path("benchmark/baseline_results.json")
    save_results(results, output_file)
    
    # Run comparison
    binary = get_cytoscnpy_binary()
    run_comparison(datasets_dir, binary)
    
    print("\n" + "=" * 60)
    print("Done! Files generated:")
    print("  - benchmark/baseline_results.json")
    print("  - benchmark/comparison.md")
    print("  - benchmark/comparison.json")
    print("  - benchmark/results_*.json (per dataset)")

if __name__ == "__main__":
    main()
