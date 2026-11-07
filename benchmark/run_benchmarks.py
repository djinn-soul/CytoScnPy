#!/usr/bin/env python3
"""CytoScnPy Benchmark Suite using Hyperfine.

Provides accurate measurements with warmup, statistics, and outlier detection.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

from hyperfine_results import (
    BenchmarkResult,
    check_regression,
    print_results_table,
    save_results,
)


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
    files, lines = 0, 0
    try:
        result = subprocess.run(
            [str(binary), "analyze", str(dataset), "--json"],
            capture_output=True,
            text=True,
            timeout=300,
        )
        if result.stdout:
            data = json.loads(result.stdout)
            summary = data.get("analysis_summary", {})
            files = summary.get("total_files", 0)
            lines = summary.get("total_lines_analyzed", 0)
    except (json.JSONDecodeError, subprocess.SubprocessError, OSError) as e:
        print(f"[-] Could not collect file stats for {dataset}: {e}", file=sys.stderr)

    return files, lines


def run_hyperfine_benchmark(
    binary: Path, dataset: Path, runs: int = 10, warmup: int = 3
) -> dict[str, Any] | None:
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
        with json_output.open() as f:
            return json.load(f)
    return None


def run_benchmark_suite(
    datasets_dir: Path, runs: int = 10, warmup: int = 3
) -> list[BenchmarkResult]:
    """Run benchmarks on all datasets using hyperfine."""
    binary = get_cytoscnpy_binary()
    print(f"Using binary: {binary}")
    print(f"Runs per dataset: {runs}, Warmup: {warmup}")

    results: list[BenchmarkResult] = []
    datasets = sorted([d for d in datasets_dir.iterdir() if d.is_dir()])

    for dataset in datasets:
        # Get file/line counts
        files, lines = get_file_stats(binary, dataset)

        # Run hyperfine
        hf_result: dict[str, Any] | None = run_hyperfine_benchmark(
            binary, dataset, runs, warmup
        )

        if hf_result and "results" in hf_result:
            r: dict[str, Any] = hf_result["results"][0]
            results.append(
                BenchmarkResult(
                    dataset=dataset.name,
                    files=files,
                    lines=lines,
                    mean_seconds=float(r.get("mean", 0)),
                    stddev_seconds=float(r.get("stddev", 0)),
                    min_seconds=float(r.get("min", 0)),
                    max_seconds=float(r.get("max", 0)),
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
        # Fallback to examples if datasets not found
        datasets_dir = Path("benchmark/examples")

    if not datasets_dir.exists():
        print("Error: benchmark/datasets and benchmark/examples directories not found")
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
