"""Normalize dependency findings that belong to manifests rather than source lines."""

from collections.abc import Mapping
from pathlib import Path
from typing import cast

_DEPENDENCY_LABELS = (
    ("missing_dependencies", "missing dependency"),
    ("unused_dependencies", "unused dependency"),
    ("transitive_dependencies", "undeclared transitive dependency"),
    ("dev_dependencies_in_production", "development dependency in production"),
    ("stdlib_dependencies", "standard library declared as dependency"),
)


def dependency_findings(
    data: Mapping[str, object], anchor: Path
) -> dict[str, list[str]]:
    """Retain manifest findings, including scans with no Python source items."""
    result: dict[str, list[str]] = {}
    directory = anchor.parent if anchor.is_file() else anchor
    manifest = _nearest_manifest(directory)
    for key, label in _DEPENDENCY_LABELS:
        values = data.get(key)
        if not isinstance(values, list):
            continue
        for value in cast(list[object], values):
            if isinstance(value, Mapping):
                item = cast(Mapping[str, object], value)
                package = item.get("package_name", item.get("name", "?"))
                location = _requirements_file(item, directory) or manifest
            else:
                package, location = value, manifest
            result.setdefault(str(location), []).append(f"  {label}: {package}")
    return result


def _nearest_manifest(directory: Path) -> Path:
    """Return the closest enclosing pyproject.toml, or the one in `directory`."""
    for parent in (directory, *directory.parents):
        if (parent / "pyproject.toml").is_file():
            return parent / "pyproject.toml"
    return directory / "pyproject.toml"


def _requirements_file(item: Mapping[str, object], directory: Path) -> Path | None:
    """Return the requirements file a finding came from, if it names one."""
    source = item.get("source")
    if not isinstance(source, Mapping):
        return None
    requirement = cast(Mapping[str, object], source).get("Requirements")
    if not isinstance(requirement, str):
        return None
    location = Path(requirement)
    return location if location.is_absolute() else directory / location
