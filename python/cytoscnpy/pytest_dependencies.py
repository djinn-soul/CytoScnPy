"""Normalize dependency findings that belong to manifests rather than source lines."""

from collections.abc import Mapping
from pathlib import Path
from typing import cast


def dependency_findings(
    data: Mapping[str, object], anchor: Path
) -> dict[str, list[str]]:
    """Retain manifest findings, including scans with no Python source items."""
    result: dict[str, list[str]] = {}
    directory = anchor.parent if anchor.is_file() else anchor
    manifest = directory / "pyproject.toml"
    for parent in (directory, *directory.parents):
        if (parent / "pyproject.toml").is_file():
            manifest = parent / "pyproject.toml"
            break
    for key, label in (
        ("missing_dependencies", "missing dependency"),
        ("unused_dependencies", "unused dependency"),
        ("transitive_dependencies", "undeclared transitive dependency"),
        ("dev_dependencies_in_production", "development dependency in production"),
        ("stdlib_dependencies", "standard library declared as dependency"),
    ):
        values = data.get(key)
        if not isinstance(values, list):
            continue
        for value in cast(list[object], values):
            location = manifest
            if isinstance(value, Mapping):
                item = cast(Mapping[str, object], value)
                package = item.get("package_name", item.get("name", "?"))
                source = item.get("source")
                if isinstance(source, Mapping):
                    requirement = cast(Mapping[str, object], source).get("Requirements")
                    if isinstance(requirement, str):
                        location = Path(requirement)
                        if not location.is_absolute():
                            location = directory / location
            else:
                package = value
            result.setdefault(str(location), []).append(f"  {label}: {package}")
    return result
