from __future__ import annotations

import json
import sys
import time
from collections.abc import Mapping
from pathlib import Path
from typing import cast

from .models import (
    Args,
    BenchmarkResult,
    FinalReport,
    FinalReportEntry,
    VerificationResult,
)
from .values import _as_float, _as_str

TYPES_TO_PRINT = ["overall", "class", "function", "import", "method", "variable"]


def report_results(
    args: Args,
    results: list[BenchmarkResult],
    verification_results: list[VerificationResult],
) -> None:
    """Print, save, and compare results with the established regression thresholds."""
    _print_benchmark_table(results)
    _print_verification_tables(verification_results)
    final_report = _build_final_report(results, verification_results)
    if args.save_json:
        _save_report(final_report, args.save_json)
    if args.compare_json:
        _compare_with_baseline(args, final_report)


def _print_benchmark_table(results: list[BenchmarkResult]) -> None:
    print("\n[=] Benchmark Results")
    print(f"{'Tool':<20} | {'Time (s)':<10} | {'Mem (MB)':<10} | {'Issues (Est)':<12}")
    print("-" * 60)
    for res in results:
        print(
            f"{res['name']:<20} | {res['time']:<10.3f} | {res['memory_mb']:<10.2f} | {res['issues']:<12}"
        )
    print("-" * 60)


def _print_verification_tables(verification_results: list[VerificationResult]) -> None:
    print("\n[=] Verification Results (Ground Truth Comparison)")
    for type_key in TYPES_TO_PRINT:
        print(f"\n--- {type_key.capitalize()} Detection ---")
        print(
            f"{'Tool':<20} | {'TP':<5} | {'FP':<5} | {'FN':<5} | {'Precision':<10} | {'Recall':<10} | {'F1 Score':<10}"
        )
        print("-" * 80)
        for v in verification_results:
            stats_value = v.get(type_key)
            if isinstance(stats_value, dict):
                print(
                    f"{_as_str(v.get('Tool')):<20} | {stats_value['TP']:<5} | {stats_value['FP']:<5} | {stats_value['FN']:<5} | {stats_value['Precision']:<10.4f} | {stats_value['Recall']:<10.4f} | {stats_value['F1']:<10.4f}"
                )
        print("-" * 80)


def _build_final_report(
    results: list[BenchmarkResult],
    verification_results: list[VerificationResult],
) -> FinalReport:
    final_report: FinalReport = {
        "timestamp": time.time(),
        "platform": sys.platform,
        "results": [],
    }
    for res in results:
        v_res = next(
            (v for v in verification_results if _as_str(v.get("Tool")) == res["name"]),
            None,
        )
        f1_score = precision = recall = 0.0
        stats_payload: Mapping[str, object] = {}
        if v_res:
            stats_payload = v_res
            overall_value = v_res.get("overall")
            if isinstance(overall_value, dict):
                f1_score = overall_value["F1"]
                precision = overall_value["Precision"]
                recall = overall_value["Recall"]
        entry: FinalReportEntry = {
            "name": res["name"],
            "time": res["time"],
            "memory_mb": res["memory_mb"],
            "issues": res["issues"],
            "f1_score": f1_score,  # Use overall F1
            "precision": precision,
            "recall": recall,
            "stats": stats_payload,
        }
        final_report["results"].append(entry)
    return final_report


def _save_report(final_report: FinalReport, path: str) -> None:
    try:
        with Path(path).open("w") as f:
            json.dump(final_report, f, indent=2)
        print(f"\n[+] Results saved to {path}")
    except OSError as e:
        print(f"[-] Failed to save JSON results: {e}")


def _baseline_entries(
    baseline_dict: dict[str, object] | None,
) -> list[dict[str, object]]:
    if baseline_dict is None:
        return []
    base_results = baseline_dict.get("results")
    if not isinstance(base_results, list):
        return []
    return [
        cast(dict[str, object], item)
        for item in cast(list[object], base_results)
        if isinstance(item, dict)
    ]


def _find_regressions(
    current: FinalReportEntry, base: dict[str, object], threshold: float
) -> list[str]:
    """Return regression messages in time, memory, F1, precision, recall order."""
    name = current["name"]
    base_time = _as_float(base.get("time"))
    base_memory = _as_float(base.get("memory_mb"))
    base_f1 = _as_float(base.get("f1_score"))
    if base_time is None or base_memory is None or base_f1 is None:
        raise ValueError(f"Baseline entry missing numeric fields for {name}")

    regressions: list[str] = []
    time_diff = current["time"] - base_time
    time_ratio = time_diff / base_time if base_time > 0 else 0
    # Ignore small time increases (< 1.0s) to avoid noise
    if time_ratio > threshold and time_diff > 1.0:
        regressions.append(
            f"{name} Time: {base_time:.3f}s -> {current['time']:.3f}s (+{time_ratio * 100:.1f}%)"
        )

    mem_diff = current["memory_mb"] - base_memory
    mem_ratio = mem_diff / base_memory if base_memory > 0 else 0
    # Ignore small memory increases (< 15MB) to avoid CI noise
    # from Python startup and RSS sampling on hosted runners.
    if mem_ratio > threshold and mem_diff > 15.0:
        regressions.append(
            f"{name} Memory: {base_memory:.1f}MB -> {current['memory_mb']:.1f}MB (+{mem_ratio * 100:.1f}%)"
        )

    # F1 regresses if strictly lower, with a tolerance for float comparison.
    f1_diff = base_f1 - current["f1_score"]
    if f1_diff > 0.001:
        regressions.append(
            f"{name} F1 Score: {base_f1:.4f} -> {current['f1_score']:.4f} (-{f1_diff:.4f})"
        )

    # Precision and recall regress if they drop by more than 0.01.
    for label, key in (("Precision", "precision"), ("Recall", "recall")):
        base_value = _as_float(base.get(key))
        if base_value is None:
            continue
        current_value = (
            current["precision"] if key == "precision" else current["recall"]
        )
        diff = base_value - current_value
        if diff > 0.01:
            regressions.append(
                f"{name} {label}: {base_value:.4f} -> {current_value:.4f} (-{diff:.4f})"
            )
    return regressions


def _compare_with_baseline(args: Args, final_report: FinalReport) -> None:
    print(f"\n[+] Comparing against baseline: {args.compare_json}")
    try:
        with Path(args.compare_json).open() as f:
            baseline = cast(object, json.load(f))
        baseline_dict = (
            cast(dict[str, object], baseline) if isinstance(baseline, dict) else None
        )
        if baseline_dict is not None and baseline_dict.get("platform") != sys.platform:
            print(
                f"[!] WARNING: Baseline platform ({baseline_dict.get('platform')}) does not match current system ({sys.platform}). Performance comparison may be inaccurate."
            )

        cytoscnpy_regressions: list[str] = []
        other_regressions: list[str] = []
        base_candidates = _baseline_entries(baseline_dict)
        for current in final_report["results"]:
            base = next(
                (
                    b
                    for b in base_candidates
                    if _as_str(b.get("name")) == current["name"]
                ),
                None,
            )
            if not base:
                print(f"    [?] New tool found (no baseline): {current['name']}")
                continue
            found = _find_regressions(current, base, args.threshold)
            if "CytoScnPy" in current["name"]:
                cytoscnpy_regressions.extend(found)
            else:
                other_regressions.extend(found)
        _report_regressions(cytoscnpy_regressions, other_regressions)
    except FileNotFoundError:
        print(f"[-] Baseline file not found: {args.compare_json}")
        sys.exit(1)
    except (OSError, json.JSONDecodeError, ValueError) as e:
        print(f"[-] Error comparing baseline: {e}")
        sys.exit(1)


def _report_regressions(
    cytoscnpy_regressions: list[str], other_regressions: list[str]
) -> None:
    # Report comparison tool regressions as warnings (informational, non-blocking)
    if other_regressions:
        print(
            "\n[!] WARNING: Comparison tool regressions detected (informational only):"
        )
        for r in other_regressions:
            print(f"    - {r}")

    # Only fail CI/CD if CytoScnPy itself regressed
    if cytoscnpy_regressions:
        print("\n[!] CYTOSCNPY PERFORMANCE REGRESSIONS DETECTED:")
        for r in cytoscnpy_regressions:
            print(f"    - {r}")
        sys.exit(1)
    print("\n[OK] No CytoScnPy regressions detected.")
