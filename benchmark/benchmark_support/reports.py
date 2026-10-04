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


def report_results(  # noqa: C901
    args: Args,
    results: list[BenchmarkResult],
    verification_results: list[VerificationResult],
) -> None:
    """Print, save, and compare results with the established regression thresholds."""
    # Print Benchmark Results
    print("\n[=] Benchmark Results")
    print(f"{'Tool':<20} | {'Time (s)':<10} | {'Mem (MB)':<10} | {'Issues (Est)':<12}")
    print("-" * 60)

    for res in results:
        print(
            f"{res['name']:<20} | {res['time']:<10.3f} | {res['memory_mb']:<10.2f} | {res['issues']:<12}"
        )

    print("-" * 60)

    # Print Verification Results
    print("\n[=] Verification Results (Ground Truth Comparison)")

    # Define types to print
    types_to_print = ["overall", "class", "function", "import", "method", "variable"]

    for type_key in types_to_print:
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

    # Compile Final JSON Report
    final_report: FinalReport = {
        "timestamp": time.time(),
        "platform": sys.platform,
        "results": [],
    }

    for res in results:
        # Find corresponding verification result
        v_res = next(
            (v for v in verification_results if _as_str(v.get("Tool")) == res["name"]),
            None,
        )

        f1_score = 0.0
        precision = 0.0
        recall = 0.0
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

    # Save JSON if requested
    if args.save_json:
        try:
            with Path(args.save_json).open("w") as f:
                json.dump(final_report, f, indent=2)
            print(f"\n[+] Results saved to {args.save_json}")
        except OSError as e:
            print(f"[-] Failed to save JSON results: {e}")

    # Compare against baseline if requested
    if args.compare_json:
        print(f"\n[+] Comparing against baseline: {args.compare_json}")
        try:
            with Path(args.compare_json).open() as f:
                baseline = cast(object, json.load(f))

            baseline_dict = (
                cast(dict[str, object], baseline)
                if isinstance(baseline, dict)
                else None
            )
            if (
                baseline_dict is not None
                and baseline_dict.get("platform") != sys.platform
            ):
                print(
                    f"[!] WARNING: Baseline platform ({baseline_dict.get('platform')}) does not match current system ({sys.platform}). Performance comparison may be inaccurate."
                )

            cytoscnpy_regressions: list[str] = []
            other_regressions: list[str] = []
            results_list = final_report["results"]
            for current in results_list:
                base_candidates: list[dict[str, object]] = []
                if baseline_dict is not None:
                    base_results = baseline_dict.get("results")
                    if isinstance(base_results, list):
                        base_results_list = cast(list[object], base_results)
                        for item in base_results_list:
                            if isinstance(item, dict):
                                base_candidates.append(cast(dict[str, object], item))  # noqa: PERF401
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

                base_time = _as_float(base.get("time"))
                base_memory = _as_float(base.get("memory_mb"))
                base_f1 = _as_float(base.get("f1_score"))
                if base_time is None or base_memory is None or base_f1 is None:
                    raise ValueError(
                        f"Baseline entry missing numeric fields for {current['name']}"
                    )

                # Determine if this is CytoScnPy or a comparison tool
                is_cytoscnpy = "CytoScnPy" in current["name"]
                # Check Time
                time_diff = current["time"] - base_time
                time_ratio = time_diff / base_time if base_time > 0 else 0
                if time_ratio > args.threshold:
                    # Ignore small time increases (< 1.0s) to avoid noise
                    if time_diff > 1.0:
                        regression_msg = f"{current['name']} Time: {base_time:.3f}s -> {current['time']:.3f}s (+{time_ratio * 100:.1f}%)"
                        if is_cytoscnpy:
                            cytoscnpy_regressions.append(regression_msg)
                        else:
                            other_regressions.append(regression_msg)

                # Check Memory
                mem_diff = current["memory_mb"] - base_memory
                mem_ratio = mem_diff / base_memory if base_memory > 0 else 0
                if mem_ratio > args.threshold:
                    # Ignore small memory increases (< 15MB) to avoid CI noise
                    # from Python startup and RSS sampling on hosted runners.
                    if mem_diff > 15.0:
                        regression_msg = f"{current['name']} Memory: {base_memory:.1f}MB -> {current['memory_mb']:.1f}MB (+{mem_ratio * 100:.1f}%)"
                        if is_cytoscnpy:
                            cytoscnpy_regressions.append(regression_msg)
                        else:
                            other_regressions.append(regression_msg)

                # Check F1 Score (Regression if strictly lower, handling float precision)
                f1_diff = base_f1 - current["f1_score"]
                if f1_diff > 0.001:  # Tolerance for float comparison
                    regression_msg = f"{current['name']} F1 Score: {base_f1:.4f} -> {current['f1_score']:.4f} (-{f1_diff:.4f})"
                    if is_cytoscnpy:
                        cytoscnpy_regressions.append(regression_msg)
                    else:
                        other_regressions.append(regression_msg)

                # Check Precision (Regression if drops more than 0.01)
                base_precision = _as_float(base.get("precision"))
                if base_precision is not None:
                    prec_diff = base_precision - current["precision"]
                    if prec_diff > 0.01:
                        regression_msg = f"{current['name']} Precision: {base_precision:.4f} -> {current['precision']:.4f} (-{prec_diff:.4f})"
                        if is_cytoscnpy:
                            cytoscnpy_regressions.append(regression_msg)
                        else:
                            other_regressions.append(regression_msg)

                # Check Recall (Regression if drops more than 0.01)
                base_recall = _as_float(base.get("recall"))
                if base_recall is not None:
                    recall_diff = base_recall - current["recall"]
                    if recall_diff > 0.01:
                        regression_msg = f"{current['name']} Recall: {base_recall:.4f} -> {current['recall']:.4f} (-{recall_diff:.4f})"
                        if is_cytoscnpy:
                            cytoscnpy_regressions.append(regression_msg)
                        else:
                            other_regressions.append(regression_msg)

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
            else:
                print("\n[OK] No CytoScnPy regressions detected.")

        except FileNotFoundError:
            print(f"[-] Baseline file not found: {args.compare_json}")
            sys.exit(1)
        except (OSError, json.JSONDecodeError, ValueError) as e:
            print(f"[-] Error comparing baseline: {e}")
            sys.exit(1)
