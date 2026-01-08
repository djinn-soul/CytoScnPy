#!/usr/bin/env python3
"""CytoScnPy Benchmark Suite using Hyperfine.

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
    """Stores benchmark metrics for a dataset."""

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
        capture_output=True,
        text=True,
        timeout=300,
    )
    files, lines = 0, 0
    try:
        if result.stdout:
            data = json.loads(result.stdout)
            summary = data.get("analysis_summary", {})
            files = summary.get("total_files", 0)
            lines = summary.get("total_lines_analyzed", 0)
    except Exception:
        pass
    return files, lines


def run_hyperfine_benchmark(
    binary: Path, dataset: Path, runs: int = 10, warmup: int = 3
) -> Optional[dict]:
    """Run hyperfine and return results."""
    name = dataset.name
    json_output = Path(f"benchmark/results_{name}.json")

    cmd = [
        "hyperfine",
        "--warmup",
        str(warmup),
        "--runs",
        str(runs),
        "--export-json",
        str(json_output),
        f"{binary} analyze {dataset} --json",
    ]

    print(f"\n=== {name} ===")
    result = subprocess.run(cmd, capture_output=False)

    if result.returncode == 0 and json_output.exists():
        with open(json_output) as f:
            return json.load(f)
    return None


def run_benchmark_suite(
    datasets_dir: Path, runs: int = 10, warmup: int = 3
) -> List[BenchmarkResult]:
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
            results.append(
                BenchmarkResult(
                    dataset=dataset.name,
                    files=files,
                    lines=lines,
                    mean_seconds=r.get("mean", 0),
                    stddev_seconds=r.get("stddev", 0),
                    min_seconds=r.get("min", 0),
                    max_seconds=r.get("max", 0),
                    runs=runs,
                )
            )

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

    cmd.extend(
        [
            "--export-markdown",
            "benchmark/comparison.md",
            "--export-json",
            "benchmark/comparison.json",
        ]
    )

    subprocess.run(cmd)


def print_results_table(results: List[BenchmarkResult]):
    """Print results as markdown table."""
    print("\n## Benchmark Results (Hyperfine)\n")
    print("| Dataset | Files | Lines | Mean (s) | Stddev | Min | Max |")
    print("|---------|-------|-------|----------|--------|-----|-----|")

    for r in results:
        print(
            f"| {r.dataset} | {r.files:,} | {r.lines:,} | "
            f"{r.mean_seconds:.3f} | {r.stddev_seconds:.3f} | "
            f"{r.min_seconds:.3f} | {r.max_seconds:.3f} |"
        )

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
                "runs": r.runs,
            }
            for r in results
        ],
    }
    with open(output_file, "w") as f:
        json.dump(data, f, indent=2)
    print(f"\nResults saved to: {output_file}")


def check_regression(
    current: List[BenchmarkResult], baseline_file: Path, threshold: float = 0.10
) -> bool:
    """Compare current results against baseline. Returns True if regression detected."""
    if not baseline_file.exists():
        print(f"\n[!] No baseline file found at {baseline_file}")
        print("    Run once without --regression-check to create baseline.")
        return False

    with open(baseline_file) as f:
        baseline = json.load(f)

    baseline_results = {r["dataset"]: r for r in baseline.get("results", [])}

    regressions = []
    for r in current:
        if r.dataset in baseline_results:
            base = baseline_results[r.dataset]
            base_mean = base.get("mean_seconds", 0)

            if base_mean > 0:
                ratio = (r.mean_seconds - base_mean) / base_mean
                if ratio > threshold:
                    regressions.append(
                        f"  - {r.dataset}: {base_mean:.3f}s -> {r.mean_seconds:.3f}s (+{ratio * 100:.1f}%)"
                    )

    # Check overall throughput
    total_lines = sum(r.lines for r in current)
    total_time = sum(r.mean_seconds for r in current)
    base_total_time = sum(r.get("mean_seconds", 0) for r in baseline.get("results", []))
    base_total_lines = sum(r.get("lines", 0) for r in baseline.get("results", []))

    if base_total_time > 0 and base_total_lines > 0:
        current_throughput = total_lines / total_time if total_time > 0 else 0
        baseline_throughput = base_total_lines / base_total_time
        throughput_change = (
            current_throughput - baseline_throughput
        ) / baseline_throughput

        print(f"\n[=] Regression Check (threshold: {threshold * 100:.0f}%)")
        print(f"    Baseline throughput: {baseline_throughput:,.0f} lines/s")
        print(f"    Current throughput:  {current_throughput:,.0f} lines/s")
        print(f"    Change: {throughput_change * 100:+.1f}%")

    if regressions:
        print("\n[!] PERFORMANCE REGRESSIONS DETECTED:")
        for r in regressions:
            print(r)
        return True
    else:
        print("\n[OK] No regressions detected.")
        return False


def main():
    """Main entry point."""
    import argparse

    parser = argparse.ArgumentParser(description="CytoScnPy Benchmark Suite")
    parser.add_argument(
        "--skip-regression",
        action="store_true",
        help="Skip regression check against baseline",
    )
    parser.add_argument(
        "--threshold",
        type=float,
        default=0.10,
        help="Regression threshold (default: 0.10 = 10%% slower)",
    )
    args = parser.parse_args()

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

    # Run regression check by default if baseline exists
    if not args.skip_regression and output_file.exists():
        regression_found = check_regression(results, output_file, args.threshold)
        if regression_found:
            print("\n[!] Benchmark FAILED due to regression!")
            print("    Use --skip-regression to skip this check")
            sys.exit(1)

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
