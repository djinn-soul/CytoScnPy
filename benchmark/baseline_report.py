"""Baseline benchmark results: JSON export and the summary table."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def save_results(results: dict[str, Any], output_file: Path):
    """Save benchmark results to JSON file."""
    output_file.write_text(json.dumps(results, indent=2))
    print(f"✓ Results saved to: {output_file}")


def print_summary_table(results: dict[str, Any]):
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
            f"{float(result['avg_time']):>7.2f} ± {float(result['stddev']):<5.2f} "
            f"{float(result['avg_memory_mb']):>10.1f}     "
            f"{float(result['throughput_files_per_sec']):>8.0f} f/s"
        )

    print()
