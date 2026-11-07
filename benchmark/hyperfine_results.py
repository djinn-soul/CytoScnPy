"""Hyperfine benchmark results: table output, JSON export, and regression checks."""

from __future__ import annotations

import json
import platform
from dataclasses import dataclass
from pathlib import Path


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


def print_results_table(results: list[BenchmarkResult]):
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


def save_results(results: list[BenchmarkResult], output_file: Path):
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
    with output_file.open("w") as f:
        json.dump(data, f, indent=2)
    print(f"\nResults saved to: {output_file}")


def check_regression(
    current: list[BenchmarkResult], baseline_file: Path, threshold: float = 0.10
) -> bool:
    """
    Compare current benchmark results against a baseline to detect performance regressions.

    Args:
        current: List of current benchmark results.
        baseline_file: Path to the baseline JSON file.
        threshold: Percentage threshold (e.g., 0.10 for 10%) for declaring a regression.

    Returns:
        True if any regression exceeds the threshold, False otherwise.
    """
    if not baseline_file.exists():
        print(f"\n[!] No baseline file found at {baseline_file}")
        print("    Run once without --regression-check to create baseline.")
        return False

    with baseline_file.open() as f:
        baseline = json.load(f)

    baseline_results = {r["dataset"]: r for r in baseline.get("results", [])}

    regressions: list[str] = []
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
            print(r)  # type: ignore[arg-type]
        return True
    print("\n[OK] No regressions detected.")
    return False
