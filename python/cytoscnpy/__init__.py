"""CytoScnPy: High-performance static analysis for Python."""

from __future__ import annotations

import json
from collections.abc import Sequence
from pathlib import Path
from typing import TypedDict, cast

from .cytoscnpy import run, scan_code_json, scan_json

__all__ = [
    "ScanResult",
    "run",
    "scan",
    "scan_code",
    "scan_code_json",
    "scan_json",
]


class ScanResult(TypedDict, total=False):
    """Structured analysis results from CytoScnPy."""

    unused_functions: list[dict[str, object]]
    unused_methods: list[dict[str, object]]
    unused_imports: list[dict[str, object]]
    unused_classes: list[dict[str, object]]
    unused_variables: list[dict[str, object]]
    unused_parameters: list[dict[str, object]]
    unused_dependencies: list[dict[str, object]]
    missing_dependencies: list[str]
    missing_dependency_details: list[dict[str, object]]
    transitive_dependencies: list[dict[str, object]]
    dev_dependencies_in_production: list[dict[str, object]]
    stdlib_dependencies: list[dict[str, object]]
    secrets: list[dict[str, object]]
    danger: list[dict[str, object]]
    quality: list[dict[str, object]]
    taint_findings: list[dict[str, object]]
    parse_errors: list[dict[str, object]]
    clones: list[dict[str, object]]
    file_metrics: list[dict[str, object]]
    analysis_summary: dict[str, object]


def scan(
    paths: Sequence[str | Path] | str | Path = ".",
    *,
    confidence: int | None = None,
    secrets: bool | None = None,
    danger: bool | None = None,
    quality: bool | None = None,
    include_tests: bool | None = None,
    exclude_folders: Sequence[str] | None = None,
    include_folders: Sequence[str] | None = None,
    include_ipynb: bool | None = None,
    clones: bool | None = None,
    clone_similarity: float | None = None,
) -> ScanResult:
    """Scan Python files and return structured analysis findings.

    Args:
        paths: A path, sequence of paths (files or directories) to analyze. Defaults to ".".
        confidence: Confidence threshold (0-100). If omitted, loaded from project config.
        secrets: Scan for hard-coded secrets/tokens. If omitted, loaded from config.
        danger: Scan for dangerous patterns/security issues. If omitted, loaded from config.
        quality: Scan for code quality/complexity issues. If omitted, loaded from config.
        include_tests: Include test files in analysis. If omitted, loaded from config.
        exclude_folders: Folders to exclude.
        include_folders: Folders to force-include.
        include_ipynb: Include Jupyter notebooks. If omitted, loaded from config.
        clones: Enable clone detection. If omitted, loaded from config.
        clone_similarity: Minimum similarity (0.0 - 1.0) for clone detection.

    Returns:
        A dictionary containing analysis results (unused code, security findings,
        quality findings, metrics, and summary).
    """
    if isinstance(paths, str | Path):
        resolved_paths = [str(paths)]
    else:
        resolved_paths = [str(p) for p in paths]

    raw_json = scan_json(
        paths=resolved_paths,
        confidence=confidence,
        secrets=secrets,
        danger=danger,
        quality=quality,
        include_tests=include_tests,
        exclude_folders=list(exclude_folders) if exclude_folders else [],
        include_folders=list(include_folders) if include_folders else [],
        include_ipynb=include_ipynb,
        clones=clones,
        clone_similarity=clone_similarity,
    )
    parsed = cast(object, json.loads(raw_json))
    if isinstance(parsed, dict):
        return cast(ScanResult, parsed)
    return {}


def scan_code(
    code: str,
    filename: str = "<stdin>",
    *,
    confidence: int = 60,
    secrets: bool = True,
    danger: bool = True,
    quality: bool = True,
) -> ScanResult:
    """Scan a Python code string directly and return structured findings.

    Args:
        code: Python source code snippet to analyze.
        filename: Optional virtual filename (useful for error messages / rules).
        confidence: Minimum confidence threshold (0-100).
        secrets: Scan for hard-coded secrets.
        danger: Scan for dangerous patterns / security vulnerabilities.
        quality: Scan for code quality / complexity issues.

    Returns:
        A dictionary containing analysis results.
    """
    raw_json = scan_code_json(
        code=code,
        filename=filename,
        confidence=confidence,
        secrets=secrets,
        danger=danger,
        quality=quality,
    )
    parsed = cast(object, json.loads(raw_json))
    if isinstance(parsed, dict):
        return cast(ScanResult, parsed)
    return {}
