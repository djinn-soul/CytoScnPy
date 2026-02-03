"""Analyze False Positives and False Negatives for CytoScnPy."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path
from typing import Any


def normalize_path(p: str) -> str:
    """Normalize path separator."""
    return str(Path(p).as_posix()).strip("/").lower()


def load_ground_truth(
    base_dir: str,
) -> tuple[dict[tuple[str, str, str, int | None], dict[str, Any]], set[str]]:
    """
    Load all ground truth files from the specified directory.

    Args:
        base_dir: The root directory to search for ground_truth.json files.

    Returns:
        A tuple containing:
        - A dictionary of truth items keyed by (path, type, name, line).
        - A set of normalized covered file paths.
    """
    truth: dict[tuple[str, str, str, int | None], dict[str, Any]] = {}
    covered_files: set[str] = set()

    for gt_path in Path(base_dir).rglob("ground_truth.json"):
        with gt_path.open() as f:
            data = json.load(f)

        base = gt_path.parent
        for file_name, content in data.get("files", {}).items():
            full_path = (base / file_name).resolve()
            norm_path = normalize_path(str(full_path))
            covered_files.add(norm_path)

            for item in content.get("dead_items", []):
                if item.get("suppressed"):
                    continue
                # Cast item to dict to satisfy typing
                name: str = item["name"]
                type_: str = item["type"]
                line: int | None = item.get("line_start")
                key = (norm_path, type_, name, line)
                truth[key] = item

    return truth, covered_files


def load_cytoscnpy_output(
    target_dir: str,
) -> dict[tuple[str, str, str, int | None], dict[str, Any]]:
    """
    Run CytoScnPy and parse its JSON output into a standardized finding format.

    Args:
        target_dir: The directory to analyze.

    Returns:
        A dictionary of findings keyed by (path, type, name, line).
    """
    # Use the absolute path provided in the original code
    tool_bin = r"E:\Github\CytoScnPy\target\release\cytoscnpy-bin.exe"
    result = subprocess.run(
        [tool_bin, target_dir, "--json"],
        capture_output=True,
        text=True,
    )

    if not result.stdout:
        print(f"Error: No output from tool. Stderr: {result.stderr}")
        return {}

    data = json.loads(result.stdout)
    findings: dict[tuple[str, str, str, int | None], dict[str, Any]] = {}
    type_map = {
        "unused_functions": "function",
        "unused_methods": "method",
        "unused_imports": "import",
        "unused_classes": "class",
        "unused_variables": "variable",
        "unused_parameters": "variable",
    }

    for key, def_type in type_map.items():
        for item in data.get(key, []):
            norm_path = normalize_path(item.get("file", ""))
            name = item.get("simple_name") or item.get("name", "").split(".")[-1]
            line: int | None = item.get("line")
            actual_type: str = item.get("def_type", def_type)
            if actual_type == "parameter":
                actual_type = "variable"

            fkey = (norm_path, actual_type, name, line)
            findings[fkey] = item

    return findings


def match_items(
    finding_key: tuple[str, str, str, int | None], truth_keys: Any
) -> tuple[str, str, str, int | None] | None:
    """
    Check if a finding matches any truth item.

    Args:
        finding_key: Tuple of (path, type, name, line) for the finding.
        truth_keys: Iterable of truth item keys.

    Returns:
        The matching truth key if found, otherwise None.
    """
    f_path, f_type, f_name, f_line = finding_key

    for t_key in truth_keys:
        t_path, t_type, t_name, t_line = t_key

        # Path match (endswith)
        if not (
            f_path.endswith(t_path)
            or t_path.endswith(f_path)
            or Path(f_path).name == Path(t_path).name
        ):
            continue

        # Type match (method<->function equivalence)
        if not (
            f_type == t_type
            or (f_type == "method" and t_type == "function")
            or (f_type == "function" and t_type == "method")
        ):
            continue

        # Name match
        f_simple = f_name.split(".")[-1]
        t_simple = t_name.split(".")[-1]

        # Determine if we have a match
        if f_simple == t_simple:
            # Check line if available
            if f_line is not None and t_line is not None:
                if abs(f_line - t_line) <= 2:
                    return t_key
            else:
                return t_key

    return None


def filter_findings(
    findings: dict[tuple[str, str, str, int | None], dict[str, Any]],
    covered_files: set[str],
) -> dict[tuple[str, str, str, int | None], dict[str, Any]]:
    """Filter findings to covered files only."""
    filtered: dict[tuple[str, str, str, int | None], dict[str, Any]] = {}
    for key, item in findings.items():
        f_path = key[0]
        if any(f_path.endswith(cv) or cv.endswith(f_path) for cv in covered_files):
            filtered[key] = item
    return filtered


def get_matches(
    filtered_findings: dict[tuple[str, str, str, int | None], dict[str, Any]],
    truth: dict[tuple[str, str, str, int | None], dict[str, Any]],
) -> tuple[
    set[tuple[str, str, str, int | None]], set[tuple[str, str, str, int | None]]
]:
    """Match findings against ground truth."""
    matched_truth: set[tuple[str, str, str, int | None]] = set()
    matched_findings: set[tuple[str, str, str, int | None]] = set()

    for f_key in filtered_findings:
        match = match_items(f_key, truth.keys())
        if match:
            matched_truth.add(match)
            matched_findings.add(f_key)

    return matched_findings, matched_truth


def print_metrics(tp: int, fp: int, fn: int) -> None:
    """Calculate and print metrics."""
    print("\n=== Overall Metrics ===")
    print(f"TP: {tp}, FP: {fp}, FN: {fn}")
    precision = tp / (tp + fp) if (tp + fp) > 0 else 0
    recall = tp / (tp + fn) if (tp + fn) > 0 else 0
    f1 = (
        2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0
    )
    print(f"Precision: {precision:.4f}, Recall: {recall:.4f}, F1: {f1:.4f}")


def print_breakdown(
    title: str, count: int, items_by_type: dict[str, list[tuple[str, str, int | None]]]
) -> None:
    """Print breakdown of findings by type."""
    print(f"\n=== {title} ({count} items) ===")
    for ftype, items in sorted(items_by_type.items()):
        print(f"\n{ftype.upper()} ({len(items)}):")
        for path, name, line in items[:10]:
            fname = Path(path).name
            print(f"  - {name} @ {fname}:{line}")
        if len(items) > 10:
            print(f"  ... and {len(items) - 10} more")


def analyze_unmatched(
    items_dict: dict[tuple[str, str, str, int | None], Any],
    matched_keys: set[tuple[str, str, str, int | None]],
) -> dict[str, list[tuple[str, str, int | None]]]:
    """Group unmatched items by type."""
    by_type: dict[str, list[tuple[str, str, int | None]]] = {}
    for key in items_dict:
        if key not in matched_keys:
            path, itype, name, line = key
            by_type.setdefault(itype, []).append((path, name, line))
    return by_type


def main():
    """Main entry point."""
    base_dir = r"E:\Github\CytoScnPy\benchmark\examples"

    print("Loading ground truth...")
    truth, covered_files = load_ground_truth(base_dir)
    print(f"Loaded {len(truth)} ground truth items from {len(covered_files)} files")

    print("\nRunning CytoScnPy...")
    findings = load_cytoscnpy_output(base_dir)
    print(f"CytoScnPy reported {len(findings)} items")

    filtered_findings = filter_findings(findings, covered_files)
    print(f"After filtering to covered files: {len(filtered_findings)} items")

    matched_findings, matched_truth = get_matches(filtered_findings, truth)

    tp = len(matched_findings)
    fp = len(filtered_findings) - tp
    fn = len(truth) - len(matched_truth)

    print_metrics(tp, fp, fn)

    fps_by_type = analyze_unmatched(filtered_findings, matched_findings)
    print_breakdown("FALSE POSITIVES", fp, fps_by_type)

    fns_by_type = analyze_unmatched(truth, matched_truth)
    print_breakdown("FALSE NEGATIVES", fn, fns_by_type)


if __name__ == "__main__":
    main()
