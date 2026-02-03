from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Callable


def find_ground_truth_files(root_dir: Path) -> list[Path]:
    """Find all ground_truth.json files in the directory."""
    return list(root_dir.rglob("ground_truth.json"))


def _check_function(name: str, line_content: str) -> str | None:
    """Verify that a function/method definition exists in the line."""
    search_name = name.split(".")[-1]
    if (
        f"def {search_name}" not in line_content
        and f"async def {search_name}" not in line_content
    ):
        return f"TYPE MISMATCH: Expected function/method '{name}' (looking for 'def {search_name}')"
    return None


def _check_class(name: str, line_content: str) -> str | None:
    """Verify that a class definition exists in the line."""
    if f"class {name}" not in line_content:
        return f"TYPE MISMATCH: Expected class '{name}'"
    return None


def _check_import(name: str, line_content: str) -> str | None:
    """Verify that an import statement exists in the line."""
    if name not in line_content and "import" not in line_content:
        return f"TYPE MISMATCH: Expected import '{name}'"
    return None


def _check_variable(name: str, line_content: str) -> str | None:
    """Verify that a variable usage exists in the line."""
    if name not in line_content:
        simple_name = name.split(".")[-1]
        if simple_name not in line_content:
            return f"NAME NOT FOUND: Expected variable '{name}'"
    return None


def verify_item(item: dict[str, Any], lines: list[str]) -> str | None:
    """Verify a single truth item against the source file content."""
    line_start = item.get("line_start")
    name: str | None = item.get("name")
    item_type: str | None = item.get("type")

    if not isinstance(line_start, int) or not name:
        return f"MISSING FIELDS: {item}"

    if line_start > len(lines):
        return f"LINE OUT OF BOUNDS: line {line_start} > {len(lines)}"

    line_content = lines[line_start - 1].strip()
    checkers: dict[str, Callable[[str, str], str | None]] = {
        "function": _check_function,
        "method": _check_function,
        "class": _check_class,
        "import": _check_import,
        "variable": _check_variable,
    }

    if item_type and (checker := checkers.get(item_type)):
        if issue := checker(name, line_content):
            return f"{issue} at line {line_start}, found: {line_content}"

    return None


def verify_ground_truth(gt_path: Path) -> list[str]:
    """Verify entire ground truth file against source files."""
    issues: list[str] = []
    base_dir = gt_path.parent

    try:
        with gt_path.open(encoding="utf-8") as f:
            data: dict[str, Any] = json.load(f)
    except (json.JSONDecodeError, OSError) as e:
        return [f"JSON ERROR: {e!s}"]

    files: dict[str, dict[str, Any]] = data.get("files", {})
    for filename, file_data in files.items():
        py_path = base_dir / filename
        if not py_path.exists():
            issues.append(f"FILE MISSING: {filename} not found in {base_dir}")
            continue

        with py_path.open(encoding="utf-8") as f:
            lines = f.read().splitlines()

        dead_items: list[dict[str, Any]] = file_data.get("dead_items", [])
        for item in dead_items:
            if item.get("suppressed"):
                continue
            if issue := verify_item(item, lines):
                issues.append(f"{filename}: {issue}")

    return issues


def main() -> None:
    """Main verification logic."""
    root_dir = Path(r"e:\Github\CytoScnPy\benchmark\examples")
    gt_files = find_ground_truth_files(root_dir)

    total_issues = 0
    for gt in gt_files:
        issues = verify_ground_truth(gt)
        if issues:
            print(f"Issues in {gt}:")
            for i in issues:
                print(f"  - {i}")
            print("-" * 40)
            total_issues += len(issues)

    print(f"\nTotal Issues Found: {total_issues}")


if __name__ == "__main__":
    main()
