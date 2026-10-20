"""pytest plugin for CytoScnPy static analysis.

Runs cytoscnpy once at session start and reports findings as pytest items.

Enable via CLI flag:
    pytest --cytoscnpy

Or via pytest ini options (pyproject.toml, pytest.ini, setup.cfg):
    [tool.pytest.ini_options]
    cytoscnpy = true
    cytoscnpy_path = "src/"   # optional, defaults to project root
"""

from __future__ import annotations

import json
import logging
import subprocess
import sys
from collections.abc import Callable, Iterable
from pathlib import Path
from typing import TYPE_CHECKING, TypeVar, cast

import pytest

from .pytest_findings import JsonObject
from .pytest_findings import detect_non_py_findings as _detect_non_py_findings
from .pytest_findings import group_by_file as _group_by_file
from .pytest_findings import iter_python_files as _iter_python_files
from .pytest_findings import resolve_file as _resolve_file

LOGGER = logging.getLogger(__name__)

if TYPE_CHECKING:
    from typing import Literal

    from _pytest._code.code import ExceptionInfo, TerminalRepr
    from _pytest.config import Config
    from _pytest.config.argparsing import Parser
    from _pytest.main import Session
    from typing_extensions import override

    _TracebackStyle = Literal["long", "short", "line", "no", "native", "value", "auto"]
else:
    _T = TypeVar("_T")

    def override(method: _T) -> _T:
        """Return method unchanged on Python versions without runtime override."""
        return method


SCAN_PATH_KEY = pytest.StashKey[Path]()
ERROR_KEY = pytest.StashKey[str | None]()
FORCE_FAIL_KEY = pytest.StashKey[bool]()
BY_FILE_KEY = pytest.StashKey[dict[str, list[str]]]()


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
    option_value = cast(object, config.getoption("--cytoscnpy", default=False))
    ini_value = cast(object, config.getini("cytoscnpy"))
    return option_value is True or ini_value is True


# ---------------------------------------------------------------------------
# Session start: run cytoscnpy once, store results on the session object
# ---------------------------------------------------------------------------


def _run_scan(scan_path: Path) -> tuple[int, str, str]:
    """Execute CytoScnPy scan.

    Prefers fast in-process scan when available; falls back to subprocess
    if subprocess has been patched (e.g. in test suites) or in-process scan fails.
    """
    is_mocked = (
        getattr(subprocess, "__name__", None) != "subprocess"
        or getattr(subprocess.run, "__module__", "") != "subprocess"
    )

    if not is_mocked:
        try:
            from .cytoscnpy import scan_json

            raw_json = scan_json(paths=[str(scan_path)])
            return 0, raw_json, ""
        except ImportError:
            pass
        except (FileNotFoundError, ValueError) as exc:
            return 1, "", str(exc)
        except (RuntimeError, OSError):
            LOGGER.debug(
                "In-process scan failed; falling back to subprocess", exc_info=True
            )

    result = subprocess.run(  # noqa: S603
        [sys.executable, "-m", "cytoscnpy", str(scan_path), "--json"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    return result.returncode, result.stdout, result.stderr


def pytest_sessionstart(session: Session) -> None:
    """Run CytoScnPy once and cache its results on the pytest session."""
    if not _is_enabled(session.config):
        return

    ini_path_value = cast(object, session.config.getini("cytoscnpy_path"))
    ini_path = ini_path_value if isinstance(ini_path_value, str) else "."
    scan_path = session.config.rootpath / ini_path

    by_file, error = _parse_scan(*_run_scan(scan_path), session.config.rootpath)
    session.stash[SCAN_PATH_KEY] = scan_path
    session.stash[BY_FILE_KEY] = by_file
    session.stash[ERROR_KEY] = error
    session.stash[FORCE_FAIL_KEY] = error is not None


def _parse_scan(
    returncode: int, raw_stdout: str, raw_stderr: str, rootpath: Path
) -> tuple[dict[str, list[str]], str | None]:
    """Group scan findings by file and describe why the scan failed, if it did."""
    try:
        data = cast(object, json.loads(raw_stdout))
    except json.JSONDecodeError:
        return {}, raw_stderr.strip() or raw_stdout[
            :200
        ] or "cytoscnpy produced no output"
    if not isinstance(data, dict):
        return {}, "cytoscnpy produced non-object JSON"

    by_file = _group_by_file(cast(JsonObject, data), rootpath)

    # Findings on non-Python files (e.g. invalid custom secret regex on `.cytoscnpy.toml`)
    # get no item from `_iter_python_files`, so they must fail the session here.
    non_py_findings = _detect_non_py_findings(by_file)
    if non_py_findings:
        return by_file, "\n".join(non_py_findings)

    # A non-zero exit with valid JSON still means the analyzer failed.
    if returncode != 0:
        return (
            by_file,
            raw_stderr.strip() or f"cytoscnpy exited with status {returncode}",
        )
    return by_file, None


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
        from_parent = cast(
            Callable[..., CytoScnPyFile],
            CytoScnPyFile.from_parent,
        )
        collector = from_parent(parent=session, path=file_path)
        items.extend(collector.collect())


class CytoScnPyError(Exception):
    """Raised by CytoScnPyItem.runtest() when findings exist for a file."""

    def __init__(self, findings: list[str]) -> None:
        """Store the rendered findings for pytest failure output."""
        super().__init__("\n".join(findings))
        self.findings = findings

    @override
    def __str__(self) -> str:
        """Render all findings as a newline-delimited failure body."""
        return "\n".join(self.findings)


class CytoScnPyFile(pytest.File):
    """One collector per Python file — yields a single CytoScnPyItem."""

    @override
    def collect(self) -> Iterable[pytest.Item]:
        """Yield the synthetic CytoScnPy item for this file."""
        from_parent = cast(
            Callable[..., CytoScnPyItem],
            CytoScnPyItem.from_parent,
        )
        yield from_parent(parent=self, name="cytoscnpy")


class CytoScnPyItem(pytest.Item):
    """Test item representing the cytoscnpy result for one file."""

    @override
    def setup(self) -> None:
        """Raise during setup when CytoScnPy itself failed.

        Raising here (rather than in runtest) makes pytest report the item as
        an ERROR instead of a FAILED, which correctly distinguishes a broken
        analyzer run from a file that simply has findings.
        """
        error = self.session.stash.get(ERROR_KEY, None)
        if error:
            raise CytoScnPyError([f"cytoscnpy error: {error}"])

    @override
    def runtest(self) -> None:
        """Fail when CytoScnPy reported issues for this file."""
        empty_findings: dict[str, list[str]] = {}
        by_file = self.session.stash.get(BY_FILE_KEY, empty_findings)
        current_path = str(self.fspath)
        resolved_current = _resolve_file(Path(current_path))

        # Match findings to this file by resolving to absolute paths
        findings = [
            msg
            for key, msgs in by_file.items()
            if key == current_path
            or (
                resolved_current is not None
                and _resolve_file(Path(key)) == resolved_current
            )
            for msg in msgs
        ]

        if findings:
            raise CytoScnPyError(findings)

    @override
    def repr_failure(
        self,
        excinfo: ExceptionInfo[BaseException],
        style: _TracebackStyle | None = None,
    ) -> str | TerminalRepr:
        """Show CytoScnPy findings directly in pytest's failure output."""
        if excinfo.errisinstance(CytoScnPyError):
            return str(excinfo.value)
        return super().repr_failure(excinfo, style)

    @override
    def reportinfo(self) -> tuple[Path, None, str]:
        """Describe this synthetic item in pytest reports."""
        return self.path, None, f"[cytoscnpy] {self.path}"
