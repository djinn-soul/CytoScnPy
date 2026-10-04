from __future__ import annotations

import json
import re
from typing import cast


def _count_cytoscnpy_issues(stdout: str) -> int:
    """Count issues from CytoScnPy output."""
    try:
        data = cast(object, json.loads(stdout))
        if not isinstance(data, dict):
            return 0
        data_dict = cast(dict[str, object], data)
        categories = [
            "unused_functions",
            "unused_methods",
            "unused_imports",
            "unused_classes",
            "unused_variables",
            "unused_parameters",
        ]
        total = 0
        for key in categories:
            items = data_dict.get(key)
            if isinstance(items, list):
                items_list = cast(list[object], items)
                total += len(items_list)
        return total
    except (json.JSONDecodeError, KeyError, TypeError):
        return 0


def _count_ruff_issues(stdout: str, output: str) -> int:
    """Count issues from Ruff output."""
    try:
        data = cast(object, json.loads(stdout))
        if isinstance(data, list):
            data_list = cast(list[object], data)
            return len(data_list)
        if isinstance(data, dict):
            data_dict = cast(dict[str, object], data)
            issues = data_dict.get("issues")
            if isinstance(issues, list):
                issues_list = cast(list[object], issues)
                return len(issues_list)
    except (json.JSONDecodeError, KeyError, TypeError):
        # Preserve the existing text count when the tool does not emit JSON.
        pass
    return len(output.strip().splitlines())


def _count_pylint_issues(stdout: str, output: str) -> int:
    """Count issues from Pylint output."""
    try:
        data = cast(object, json.loads(stdout))
        if isinstance(data, list):
            data_list = cast(list[object], data)
            return len(data_list)
    except (json.JSONDecodeError, KeyError, TypeError):
        # Preserve the existing text count when the tool does not emit JSON.
        pass
    return len([line for line in output.splitlines() if ": " in line])


def _count_dead_issues(output: str) -> int:
    """Count issues from dead output."""
    return len(
        [
            line
            for line in output.splitlines()
            if "is never" in line.lower() or "never read" in line.lower()
        ]
    )


def _count_deadcode_issues(output: str) -> int:
    """Count issues from deadcode output."""
    return len([line for line in output.splitlines() if re.search(r": DC\d+", line)])


def _count_skylos_issues(stdout: str) -> int:
    """Count issues from Skylos output."""
    try:
        data = cast(object, json.loads(stdout))
        if not isinstance(data, dict):
            return 0
        data_dict = cast(dict[str, object], data)
        total = 0
        for key in [
            "unused_functions",
            "unused_imports",
            "unused_classes",
            "unused_variables",
        ]:
            items = data_dict.get(key)
            if isinstance(items, list):
                items_list = cast(list[object], items)
                total += len(items_list)
        return total
    except (json.JSONDecodeError, KeyError, TypeError):
        return 0


def _count_issues(name: str, stdout: str, _stderr: str, output: str) -> int:
    """Helper to count issues for each tool."""
    if name in ["CytoScnPy (Rust)", "CytoScnPy (Python)"]:
        return _count_cytoscnpy_issues(stdout)
    if name == "Ruff":
        return _count_ruff_issues(stdout, output)
    if name == "Flake8":
        return len(output.strip().splitlines())
    if name == "Pylint":
        return _count_pylint_issues(stdout, output)
    if "Vulture" in name:
        return len(output.strip().splitlines())
    if name == "uncalled":
        return len([line for line in output.splitlines() if "unused" in line.lower()])
    if name == "dead":
        return _count_dead_issues(output)
    if name == "deadcode":
        return _count_deadcode_issues(output)
    if name == "Skylos":
        return _count_skylos_issues(stdout)
    return 0
