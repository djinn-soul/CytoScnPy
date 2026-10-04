"""Normalize scanner findings and discover Python files for pytest items."""

from collections.abc import Iterable, Mapping
from pathlib import Path
from typing import cast

from .pytest_dependencies import dependency_findings

JsonObject = Mapping[str, object]

# Directories that the analyzer itself skips. Mirrors the default exclusion
# set so pytest does not synthesize items for files cytoscnpy never analyzed
# (e.g. `.venv`, vendored deps, build artefacts).
_SKIP_DIRS: frozenset[str] = frozenset(
    {
        ".git",
        ".hg",
        ".svn",
        ".venv",
        "venv",
        "env",
        ".env",
        "__pycache__",
        ".mypy_cache",
        ".pytest_cache",
        ".ruff_cache",
        ".tox",
        ".nox",
        "build",
        "dist",
        "node_modules",
        "site-packages",
        ".eggs",
    }
)


def iter_python_files(scan_path: Path) -> list[Path]:
    """Collect Python paths eligible for synthetic pytest items."""
    if scan_path.is_file():
        return [scan_path] if scan_path.suffix == ".py" else []
    if not scan_path.exists():
        return []
    return sorted(
        path
        for path in scan_path.rglob("*.py")
        if path.is_file() and not any(part in _SKIP_DIRS for part in path.parts)
    )


def _iter_objects(value: object) -> Iterable[JsonObject]:
    if not isinstance(value, list):
        return
    items = cast(list[object], value)
    for item in items:
        if isinstance(item, dict):
            yield cast(JsonObject, item)


def _string_field(item: JsonObject, key: str, default: str = "?") -> str:
    value = item.get(key, default)
    return value if isinstance(value, str) else str(value)


def group_by_file(data: JsonObject, anchor: Path | None = None) -> dict[str, list[str]]:
    """Normalize all finding types into {file_path_str: [message, ...]}."""
    anchor = Path() if anchor is None else anchor
    by_file = dependency_findings(data, anchor)

    dead_keys = [
        ("unused_functions", "unused function"),
        ("unused_methods", "unused method"),
        ("unused_classes", "unused class"),
        ("unused_imports", "unused import"),
        ("unused_variables", "unused variable"),
        ("unused_parameters", "unused parameter"),
    ]
    for key, label in dead_keys:
        for item in _iter_objects(data.get(key, [])):
            file = _string_field(item, "file", "")
            name = _string_field(item, "name")
            line = _string_field(item, "line")
            by_file.setdefault(file, []).append(f"  {line}: {label}: {name}")

    for key in ("danger", "quality", "clones"):
        for item in _iter_objects(data.get(key, [])):
            file = _string_field(item, "file", "")
            msg = _string_field(item, "message")
            rule = _string_field(item, "rule_id", key)
            line = _string_field(item, "line")
            by_file.setdefault(file, []).append(f"  {line}: {rule}: {msg}")

    for item in _iter_objects(data.get("secrets", [])):
        file = _string_field(item, "file", "")
        msg = _string_field(item, "message")
        line = _string_field(item, "line")
        by_file.setdefault(file, []).append(f"  {line}: secret: {msg}")

    for item in _iter_objects(data.get("taint_findings", [])):
        file = _string_field(item, "file", "")
        source = _string_field(item, "source")
        line = _string_field(item, "source_line")
        by_file.setdefault(file, []).append(f"  {line}: taint: {source}")

    for item in _iter_objects(data.get("parse_errors", [])):
        file = _string_field(item, "file", "")
        error = _string_field(item, "error", "parse error")
        by_file.setdefault(file, []).append(f"  parse error: {error}")

    _add_scan_errors(data, anchor, by_file)

    return by_file


def _add_scan_errors(
    data: JsonObject, anchor: Path, by_file: dict[str, list[str]]
) -> None:
    """Keep traversal/read errors visible even when no source item exists."""
    errors = data.get("scan_errors", [])
    for item in _iter_objects(errors):
        file = _string_field(item, "file", _string_field(item, "path", str(anchor)))
        error = _string_field(
            item, "error", _string_field(item, "reason", "scan error")
        )
        by_file.setdefault(file, []).append(f"  scan error: {error}")
    if isinstance(errors, list):
        for error in cast(list[object], errors):
            if isinstance(error, str):
                by_file.setdefault(str(anchor), []).append(f"  scan error: {error}")


def resolve_file(path: Path) -> Path | None:
    """Resolve a finding path, allowing invalid paths to remain unmatched."""
    try:
        return path.resolve()
    except (OSError, ValueError):
        return None


def detect_non_py_findings(by_file: Mapping[str, list[str]]) -> list[str]:
    """Extract findings for files that are not Python sources."""
    non_py_findings: list[str] = []
    for file_str, msgs in by_file.items():
        if not file_str.endswith(".py"):
            non_py_findings.extend(f"{file_str}:{msg}" for msg in msgs)
    return non_py_findings
