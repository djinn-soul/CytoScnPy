"""Analyze False Positives and False Negatives for CytoScnPy."""

import subprocess
import json
from pathlib import Path


def normalize_path(p):
    """Normalize path separator."""
    return str(Path(p).as_posix()).strip("/").lower()


def load_ground_truth(base_dir):
    """Load all ground truth files."""
    truth = {}
    covered_files = set()

    for gt_path in Path(base_dir).rglob("ground_truth.json"):
        with open(gt_path) as f:
            data = json.load(f)

        base = gt_path.parent
        for file_name, content in data.get("files", {}).items():
            full_path = (base / file_name).resolve()
            norm_path = normalize_path(str(full_path))
            covered_files.add(norm_path)

            for item in content.get("dead_items", []):
                if item.get("suppressed"):
                    continue
                key = (norm_path, item["type"], item["name"], item.get("line_start"))
                truth[key] = item

    return truth, covered_files


def load_cytoscnpy_output(target_dir):
    """Run CytoScnPy and parse output."""
    result = subprocess.run(
        [r"E:\Github\CytoScnPy\target\release\cytoscnpy-bin.exe", target_dir, "--json"],
        capture_output=True,
        text=True,
    )

    if not result.stdout:
        print(f"Error: No output from tool. Stderr: {result.stderr}")
        return {}

    data = json.loads(result.stdout)

    findings = {}
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
            line = item.get("line")
            actual_type = item.get("def_type", def_type)
            if actual_type == "parameter":
                actual_type = "variable"

            fkey = (norm_path, actual_type, name, line)
            findings[fkey] = item

    return findings


def match_items(finding_key, truth_keys):
    """Check if a finding matches any truth item."""
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
        is_match = False
        if f_simple == t_simple:
            # Check line if available
            if f_line is not None and t_line is not None:
                if abs(f_line - t_line) <= 2:
                    is_match = True
            else:
                 is_match = True

        if not is_match:
            continue

        return t_key

    return None


def main():
    """Main entry point."""
    base_dir = r"E:\Github\CytoScnPy\benchmark\examples"

    print("Loading ground truth...")
    truth, covered_files = load_ground_truth(base_dir)
    print(f"Loaded {len(truth)} ground truth items from {len(covered_files)} files")

    print("\nRunning CytoScnPy...")
    findings = load_cytoscnpy_output(base_dir)
    print(f"CytoScnPy reported {len(findings)} items")

    # Filter findings to covered files only
    filtered_findings = {}
    for key, item in findings.items():
        f_path = key[0]
        is_covered = False
        for cv in covered_files:
            if f_path.endswith(cv) or cv.endswith(f_path):
                is_covered = True
                break
        if is_covered:
            filtered_findings[key] = item

    print(f"After filtering to covered files: {len(filtered_findings)} items")

    # Match findings
    matched_truth = set()
    matched_findings = set()

    for f_key in filtered_findings:
        match = match_items(f_key, truth.keys())

        if match:
            matched_truth.add(match)
            matched_findings.add(f_key)

    # Calculate metrics
    tp = len(matched_findings)
    fp = len(filtered_findings) - tp
    fn = len(truth) - len(matched_truth)

    print("\n=== Overall Metrics ===")
    print(f"TP: {tp}, FP: {fp}, FN: {fn}")
    precision = tp / (tp + fp) if (tp + fp) > 0 else 0
    recall = tp / (tp + fn) if (tp + fn) > 0 else 0
    f1 = (
        2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0
    )
    print(f"Precision: {precision:.4f}, Recall: {recall:.4f}, F1: {f1:.4f}")

    # Analyze FP
    print(f"\n=== FALSE POSITIVES ({fp} items) ===")
    fp_by_type = {}
    for f_key, item in filtered_findings.items():
        if f_key not in matched_findings:
            f_path, f_type, f_name, f_line = f_key
            fp_by_type.setdefault(f_type, []).append((f_path, f_name, f_line))

    for ftype, items in sorted(fp_by_type.items()):
        print(f"\n{ftype.upper()} ({len(items)}):")
        for path, name, line in items[:10]:
            # Just show filename
            fname = Path(path).name
            print(f"  - {name} @ {fname}:{line}")
        if len(items) > 10:
            print(f"  ... and {len(items) - 10} more")

    # Analyze FN
    print(f"\n=== FALSE NEGATIVES ({fn} items) ===")
    fn_by_type = {}
    for t_key, item in truth.items():
        if t_key not in matched_truth:
            t_path, t_type, t_name, t_line = t_key
            fn_by_type.setdefault(t_type, []).append((t_path, t_name, t_line))

    for ftype, items in sorted(fn_by_type.items()):
        print(f"\n{ftype.upper()} ({len(items)}):")
        for path, name, line in items[:10]:
            # Just show filename
            fname = Path(path).name
            print(f"  - {name} @ {fname}:{line}")
        if len(items) > 10:
            print(f"  ... and {len(items) - 10} more")


if __name__ == "__main__":
    main()
