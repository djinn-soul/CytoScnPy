"""pytest plugin for CytoScnPy static analysis.

Mirrors the pytest-vulture pattern: runs cytoscnpy once at session start, then
creates one pytest Item per Python file so findings appear as native PASSED/FAILED
test results rather than a custom terminal section.

Enable via CLI flag:
    pytest --cytoscnpy

Or via pytest ini options (pyproject.toml, pytest.ini, setup.cfg):
    [tool.pytest.ini_options]
    cytoscnpy = true
    cytoscnpy_path = "src/"   # optional, defaults to project root
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from typing import TYPE_CHECKING, Iterable

import pytest

if TYPE_CHECKING:
    from _pytest._code.code import ExceptionInfo
    from _pytest.config import Config, Parser
    from _pytest.main import Session

SCAN_PATH_KEY = pytest.StashKey[Path]()
ERROR_KEY = pytest.StashKey[str | None]()
FORCE_FAIL_KEY = pytest.StashKey[bool]()
BY_FILE_KEY = pytest.StashKey[dict[str, list[str]]]()

# ---------------------------------------------------------------------------
# Registration hooks
# ---------------------------------------------------------------------------


def pytest_addoption(parser: Parser) -> None:
    """Register CytoScnPy CLI and ini configuration."""
    group = parser.getgroup("cytoscnpy", "CytoScnPy static analysis")
    group.addoption(
        "--cytoscnpy",
        action="store_true",
        default=False,
        help="Run CytoScnPy static analysis alongside tests.",
    )
    parser.addini(
        "cytoscnpy",
        type="bool",
        default=False,
        help="Enable CytoScnPy static analysis (equivalent to --cytoscnpy).",
    )
    parser.addini(
        "cytoscnpy_path",
        default=".",
        help="Path to scan with CytoScnPy, relative to the project root (default: '.').",
    )


def _is_enabled(config: Config) -> bool:
    return config.getoption("--cytoscnpy", default=False) or bool(
        config.getini("cytoscnpy")
    )


# ---------------------------------------------------------------------------
# Session start: run cytoscnpy once, store results on the session object
# ---------------------------------------------------------------------------


def pytest_sessionstart(session: Session) -> None:
    """Run CytoScnPy once and cache its results on the pytest session."""
    if not _is_enabled(session.config):
        return

    ini_path = session.config.getini("cytoscnpy_path") or "."
    scan_path = Path(str(session.config.rootdir)) / ini_path

    result = subprocess.run(  # noqa: S603
        [sys.executable, "-m", "cytoscnpy", str(scan_path), "--json"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )

    session.stash[SCAN_PATH_KEY] = scan_path
    session.stash[ERROR_KEY] = None
    session.stash[FORCE_FAIL_KEY] = False
    session.stash[BY_FILE_KEY] = {}

    try:
        data = json.loads(result.stdout)
    except json.JSONDecodeError:
        session.stash[ERROR_KEY] = (
            result.stderr.strip()
            or result.stdout[:200]
            or "cytoscnpy produced no output"
        )
        session.stash[FORCE_FAIL_KEY] = result.returncode != 0
        return

    session.stash[BY_FILE_KEY] = _group_by_file(data)


def pytest_sessionfinish(session: Session, exitstatus: int) -> None:
    """Force a failing pytest exit when CytoScnPy crashed before item execution."""
    if session.stash.get(FORCE_FAIL_KEY, False):
        session.exitstatus = max(int(exitstatus), int(pytest.ExitCode.TESTS_FAILED))


def pytest_collection_modifyitems(
    session: Session, config: Config, items: list[pytest.Item]
) -> None:
    """Append one CytoScnPy item for every analyzed Python file."""
    if not _is_enabled(config):
        return

    scan_path = session.stash.get(SCAN_PATH_KEY, None)
    if scan_path is None:
        return

    for file_path in _iter_python_files(scan_path):
        collector = CytoScnPyFile.from_parent(parent=session, path=file_path)
        items.extend(collector.collect())


def _iter_python_files(scan_path: Path) -> list[Path]:
    if scan_path.is_file():
        return [scan_path] if scan_path.suffix == ".py" else []
    if not scan_path.exists():
        return []
    return sorted(path for path in scan_path.rglob("*.py") if path.is_file())


def _group_by_file(data: dict) -> dict[str, list[str]]:
    """Normalize all finding types into {file_path_str: [message, ...]}."""
    by_file: dict[str, list[str]] = {}

    dead_keys = [
        ("unused_functions", "unused function"),
        ("unused_methods", "unused method"),
        ("unused_classes", "unused class"),
        ("unused_imports", "unused import"),
        ("unused_variables", "unused variable"),
        ("unused_parameters", "unused parameter"),
    ]
    for key, label in dead_keys:
        for item in data.get(key, []):
            file = str(item.get("file", ""))
            name = item.get("name", "?")
            line = item.get("line", "?")
            by_file.setdefault(file, []).append(f"  {line}: {label}: {name}")

    for key in ("danger", "quality"):
        for item in data.get(key, []):
            file = str(item.get("file", ""))
            msg = item.get("message", "?")
            rule = item.get("rule_id", key)
            line = item.get("line", "?")
            by_file.setdefault(file, []).append(f"  {line}: {rule}: {msg}")

    for item in data.get("secrets", []):
        file = str(item.get("file", ""))
        msg = item.get("message", "?")
        line = item.get("line", "?")
        by_file.setdefault(file, []).append(f"  {line}: secret: {msg}")

    for item in data.get("taint_findings", []):
        file = str(item.get("file", ""))
        source = item.get("source", "?")
        line = item.get("source_line", "?")
        by_file.setdefault(file, []).append(f"  {line}: taint: {source}")

    for item in data.get("parse_errors", []):
        file = str(item.get("file", ""))
        error = item.get("error", "parse error")
        by_file.setdefault(file, []).append(f"  parse error: {error}")

    return by_file


# ---------------------------------------------------------------------------
# Collection: one Item per .py file within the scan path
# ---------------------------------------------------------------------------


# ---------------------------------------------------------------------------
# Custom nodes
# ---------------------------------------------------------------------------


class CytoScnPyError(Exception):
    """Raised by CytoScnPyItem.runtest() when findings exist for a file."""

    def __init__(self, findings: list[str]) -> None:
        """Store the rendered findings for pytest failure output."""
        self.findings = findings

    def __str__(self) -> str:
        """Render all findings as a newline-delimited failure body."""
        return "\n".join(self.findings)


class CytoScnPyFile(pytest.File):
    """One collector per Python file — yields a single CytoScnPyItem."""

    def collect(self) -> Iterable[pytest.Item | pytest.Collector]:
        """Yield the synthetic CytoScnPy item for this file."""
        yield CytoScnPyItem.from_parent(parent=self, name="cytoscnpy")


class CytoScnPyItem(pytest.Item):
    """Test item representing the cytoscnpy result for one file."""

    def runtest(self) -> None:
        """Fail when CytoScnPy reported issues for this file or the whole run."""
        error = self.session.stash.get(ERROR_KEY, None)
        if error:
            raise CytoScnPyError([f"cytoscnpy error: {error}"])

        by_file = self.session.stash.get(BY_FILE_KEY, {})
        current_path = str(self.fspath)
        resolved_current = _resolve_file(Path(current_path))

        # Match findings to this file by resolving to absolute paths
        findings: list[str] = []
        for key, msgs in by_file.items():
            resolved_key = _resolve_file(Path(key))
            if (
                resolved_current is not None
                and resolved_key is not None
                and resolved_key == resolved_current
            ) or key == current_path:
                findings.extend(msgs)

        if findings:
            raise CytoScnPyError(findings)

    def repr_failure(
        self,
        excinfo: ExceptionInfo[BaseException],
        *args: object,
        **kwargs: object,
    ) -> str:
        """Show CytoScnPy findings directly in pytest's failure output."""
        if excinfo.errisinstance(CytoScnPyError):
            return str(excinfo.value)
        return super().repr_failure(excinfo, *args, **kwargs)

    def reportinfo(self) -> tuple[Path, None, str]:
        """Describe this synthetic item in pytest reports."""
        return self.fspath, None, f"[cytoscnpy] {self.fspath}"


def _resolve_file(path: Path) -> Path | None:
    try:
        return path.resolve()
    except (OSError, ValueError):
        return None
