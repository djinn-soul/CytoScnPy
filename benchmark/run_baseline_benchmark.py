#!/usr/bin/env python3
"""
Run comprehensive performance benchmarks before optimizations.

Measures:
- Wall-clock time
- Peak memory usage
- Throughput (files/second)
- CPU utilization
- Per-module timing breakdowns
"""

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path
from typing import Dict, Any
import psutil


def get_memory_usage() -> float:
    """Get current process memory usage in MB."""
    process = psutil.Process()
    return process.memory_info().rss / 1024 / 1024


def run_benchmark(
    corpus_path: Path, cytoscnpy_bin: Path, iterations: int = 3, flags: list[str] = None
) -> Dict[str, Any]:
    """Run benchmark and collect metrics."""

    flags = flags or []
    cmd = [str(cytoscnpy_bin), str(corpus_path)] + flags

    print(f"Running: {' '.join(cmd)}")
    print(f"Iterations: {iterations}")
    print()

    times = []
    peak_memory = []

    for i in range(iterations):
        print(f"  Iteration {i + 1}/{iterations}...", end=" ", flush=True)

        # Start memory monitoring
        # Start memory monitoring
        start_time = time.perf_counter()

        # Run cytoscnpy
        try:
            # Use DEVNULL to avoid pipe buffer deadlock on large output
            process = subprocess.Popen(
                cmd,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
                text=True,
                encoding="utf-8",
                errors="replace",
            )

            # Track peak memory
            max_mem = 0.0
            while process.poll() is None:
                try:
                    # Get memory of the process and its children
                    proc_obj = psutil.Process(process.pid)
                    mem = proc_obj.memory_info().rss
                    for child in proc_obj.children(recursive=True):
                        try:
                            mem += child.memory_info().rss
                        except (psutil.NoSuchProcess, psutil.AccessDenied):
                            pass
                    max_mem = max(max_mem, mem / 1024 / 1024)
                except (psutil.NoSuchProcess, psutil.AccessDenied):
                    pass
                time.sleep(0.01)

            _, stderr = process.communicate()

            if process.returncode != 0:
                raise subprocess.CalledProcessError(
                    process.returncode, cmd, output="", stderr=stderr
                )

            end_time = time.perf_counter()
            elapsed = end_time - start_time

            times.append(elapsed)
            peak_memory.append(max_mem)

            print(f"✓ {elapsed:.2f}s (mem: {max_mem:.1f}MB)")

        except subprocess.CalledProcessError as e:
            print(f"✗ Failed: {e}")
            print(f"STDOUT: {e.stdout}")
            print(f"STDERR: {e.stderr}")
            sys.exit(1)

    # Calculate statistics
    avg_time = sum(times) / len(times)
    min_time = min(times)
    max_time = max(times)
    avg_memory = sum(peak_memory) / len(peak_memory)

    # Count files in corpus
    num_files = len(list(corpus_path.rglob("*.py")))
    throughput = num_files / avg_time

    return {
        "times": times,
        "avg_time": avg_time,
        "min_time": min_time,
        "max_time": max_time,
        "stddev": (sum((t - avg_time) ** 2 for t in times) / len(times)) ** 0.5,
        "peak_memory_mb": peak_memory,
        "avg_memory_mb": avg_memory,
        "num_files": num_files,
        "throughput_files_per_sec": throughput,
        "command": " ".join(cmd),
        "flags": flags,
    }


def run_comprehensive_benchmarks(
    corpus_path: Path, cytoscnpy_bin: Path
) -> Dict[str, Any]:
    """Run benchmarks with different configurations."""

    print("=" * 80)
    print("CytoScnPy Baseline Performance Benchmark")
    print("=" * 80)
    print()

    configs = [
        {
            "name": "Basic (dead code only)",
            "flags": [],
        },
        {
            "name": "With secrets scanning",
            "flags": ["--secrets"],
        },
        {
            "name": "With danger scanning",
            "flags": ["--danger"],
        },
        {
            "name": "With quality metrics",
            "flags": ["--quality"],
        },
        {
            "name": "Full scan (all features)",
            "flags": ["--secrets", "--danger", "--quality"],
        },
        {
            "name": "Clone detection",
            "flags": ["--clones"],
        },
    ]

    results = {}

    for config in configs:
        print(f"\nBenchmark: {config['name']}")
        print("-" * 80)

        result = run_benchmark(
            corpus_path, cytoscnpy_bin, iterations=3, flags=config["flags"]
        )

        results[config["name"]] = result

        print("\nResults:")
        print(
            f"  Average time:     {result['avg_time']:.2f}s ± {result['stddev']:.2f}s"
        )
        print(f"  Best time:        {result['min_time']:.2f}s")
        print(f"  Average memory:   {result['avg_memory_mb']:.1f}MB")
        print(f"  Throughput:       {result['throughput_files_per_sec']:.0f} files/sec")
        print()

    return results


def save_results(results: Dict[str, Any], output_file: Path):
    """Save benchmark results to JSON file."""
    output_file.write_text(json.dumps(results, indent=2))
    print(f"✓ Results saved to: {output_file}")


def print_summary_table(results: Dict[str, Any]):
    """Print a summary comparison table."""
    print("\n" + "=" * 80)
    print("SUMMARY COMPARISON")
    print("=" * 80)
    print()
    print(
        f"{'Configuration':<35} {'Time (s)':<15} {'Memory (MB)':<15} {'Throughput':<15}"
    )
    print("-" * 80)

    for config_name, result in results.items():
        print(
            f"{config_name:<35} "
            f"{result['avg_time']:>7.2f} ± {result['stddev']:<5.2f} "
            f"{result['avg_memory_mb']:>10.1f}     "
            f"{result['throughput_files_per_sec']:>8.0f} f/s"
        )

    print()


def main():
    parser = argparse.ArgumentParser(description="Run baseline performance benchmarks")
    parser.add_argument(
        "--corpus",
        type=Path,
        default=Path("benchmark_corpus"),
        help="Path to test corpus (default: benchmark_corpus)",
    )
    parser.add_argument(
        "--binary",
        type=Path,
        default=Path("target/release/cytoscnpy"),
        help="Path to cytoscnpy binary (default: target/release/cytoscnpy)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("benchmark/baseline_results.json"),
        help="Output file for results (default: benchmark/baseline_results.json)",
    )
    parser.add_argument(
        "--quick",
        action="store_true",
        help="Run quick benchmark (basic mode only, 1 iteration)",
    )

    args = parser.parse_args()

    # Validate inputs
    if not args.corpus.exists():
        print(f"Error: Corpus directory not found: {args.corpus}")
        print("Run: python benchmark/generate_test_corpus.py")
        sys.exit(1)

    if not args.binary.exists():
        print(f"Error: Binary not found: {args.binary}")
        print("Run: cargo build --release")
        sys.exit(1)

    # System info
    print("System Information:")
    print(
        f"  CPU cores:     {psutil.cpu_count(logical=False)} physical, {psutil.cpu_count(logical=True)} logical"
    )
    print(f"  Total RAM:     {psutil.virtual_memory().total / 1024**3:.1f} GB")
    print(f"  Python:        {sys.version.split()[0]}")
    print()

    # Count files
    num_files = len(list(args.corpus.rglob("*.py")))
    corpus_size_mb = (
        sum(f.stat().st_size for f in args.corpus.rglob("*.py")) / 1024 / 1024
    )
    print("Corpus Statistics:")
    print(f"  Files:         {num_files:,}")
    print(f"  Total size:    {corpus_size_mb:.1f} MB")
    print()

    # Run benchmarks
    if args.quick:
        print("Running quick benchmark (basic mode only)...")
        result = run_benchmark(args.corpus, args.binary, iterations=1, flags=[])
        results = {"Basic (dead code only)": result}
    else:
        results = run_comprehensive_benchmarks(args.corpus, args.binary)

    # Save and display results
    save_results(results, args.output)
    print_summary_table(results)

    print("\n✓ Benchmark complete!")
    print("\nNext steps:")
    print(f"  1. Review results in {args.output}")
    print("  2. Implement optimizations")
    print("  3. Re-run benchmark to measure improvements")


if __name__ == "__main__":
    main()
