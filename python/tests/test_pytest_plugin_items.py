"""Unit tests for the pytest plugin's per-file items, finding grouping, and file discovery."""

from __future__ import annotations

from pathlib import Path
from types import SimpleNamespace
from typing import Any, cast

import pytest

import cytoscnpy.pytest_plugin as pytest_plugin


def test_iter_python_files_handles_file_missing_and_skipped_dirs(tmp_path):
    py_file = tmp_path / "one.py"
    py_file.write_text("VALUE = 1\n", encoding="utf-8")
    text_file = tmp_path / "notes.txt"
    text_file.write_text("ignore\n", encoding="utf-8")
    skipped_dir = tmp_path / ".venv"
    skipped_dir.mkdir()
    (skipped_dir / "ignored.py").write_text("VALUE = 2\n", encoding="utf-8")

    assert pytest_plugin._iter_python_files(py_file) == [py_file]
    assert pytest_plugin._iter_python_files(text_file) == []
    assert pytest_plugin._iter_python_files(tmp_path / "missing") == []
    assert pytest_plugin._iter_python_files(tmp_path) == [py_file]


def test_group_by_file_includes_parse_errors():
    data = {
        "parse_errors": [
            {
                "file": "src/broken.py",
                "error": "unexpected EOF while parsing",
            }
        ]
    }

    grouped = pytest_plugin._group_by_file(data)

    assert grouped == {
        "src/broken.py": ["  parse error: unexpected EOF while parsing"],
    }


def test_group_by_file_includes_all_finding_categories():
    data = {
        "unused_methods": [{"file": "a.py", "name": "m", "line": 2}],
        "unused_classes": [{"file": "a.py", "name": "C", "line": 3}],
        "unused_imports": [{"file": "a.py", "name": "os", "line": 4}],
        "unused_variables": [{"file": "a.py", "name": "x", "line": 5}],
        "unused_parameters": [{"file": "a.py", "name": "arg", "line": 6}],
        "danger": [
            {"file": "b.py", "message": "danger", "rule_id": "CSP-D001", "line": 7}
        ],
        "quality": [{"file": "b.py", "message": "quality", "line": 8}],
        "secrets": [{"file": "c.py", "message": "secret", "line": 9}],
        "taint_findings": [{"file": "d.py", "source": "user", "source_line": 10}],
    }

    grouped = pytest_plugin._group_by_file(data)

    assert "  2: unused method: m" in grouped["a.py"]
    assert "  3: unused class: C" in grouped["a.py"]
    assert "  4: unused import: os" in grouped["a.py"]
    assert "  5: unused variable: x" in grouped["a.py"]
    assert "  6: unused parameter: arg" in grouped["a.py"]
    assert grouped["b.py"] == ["  7: CSP-D001: danger", "  8: quality: quality"]
    assert grouped["c.py"] == ["  9: secret: secret"]
    assert grouped["d.py"] == ["  10: taint: user"]


def test_runtest_fails_when_file_has_parse_error():
    session = SimpleNamespace(
        stash={
            pytest_plugin.ERROR_KEY: None,
            pytest_plugin.BY_FILE_KEY: {
                "src/broken.py": ["  parse error: unexpected EOF while parsing"],
            },
        }
    )
    fake_item: Any = SimpleNamespace(session=session, fspath="src/broken.py")

    with pytest.raises(pytest_plugin.CytoScnPyError, match="parse error"):
        pytest_plugin.CytoScnPyItem.runtest(fake_item)


def test_cytoscnpy_error_str_joins_findings():
    error = pytest_plugin.CytoScnPyError(["one", "two"])

    assert str(error) == "one\ntwo"


def test_item_setup_raises_cached_error():
    session = SimpleNamespace(stash={pytest_plugin.ERROR_KEY: "bad json"})
    fake_item: Any = SimpleNamespace(session=session)

    with pytest.raises(pytest_plugin.CytoScnPyError, match="bad json"):
        pytest_plugin.CytoScnPyItem.setup(fake_item)


def test_repr_failure_uses_cytoscnpy_error_string():
    error = pytest_plugin.CytoScnPyError(["finding"])
    excinfo = SimpleNamespace(
        errisinstance=lambda cls: cls is pytest_plugin.CytoScnPyError,
        value=error,
    )
    fake_item: Any = SimpleNamespace()

    rendered = pytest_plugin.CytoScnPyItem.repr_failure(fake_item, cast(Any, excinfo))

    assert rendered == "finding"


def test_reportinfo_returns_item_path(tmp_path):
    fake_item: Any = SimpleNamespace(path=tmp_path / "module.py")

    assert pytest_plugin.CytoScnPyItem.reportinfo(fake_item) == (
        tmp_path / "module.py",
        None,
        f"[cytoscnpy] {tmp_path / 'module.py'}",
    )


def test_resolve_file_returns_none_on_invalid_path(monkeypatch, tmp_path):
    def _raise(self: Path) -> Path:
        raise OSError("cannot resolve")

    monkeypatch.setattr(Path, "resolve", _raise)

    assert pytest_plugin._resolve_file(tmp_path / "broken.py") is None
