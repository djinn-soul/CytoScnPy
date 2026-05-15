from __future__ import annotations

from pathlib import Path
from types import SimpleNamespace
from typing import Any

import pytest

import cytoscnpy.pytest_plugin as pytest_plugin


class _DummyConfig:
    def __init__(self, rootpath: Path) -> None:
        self.rootpath = rootpath

    def getini(self, name: str) -> Any:
        if name == "cytoscnpy_path":
            return "."
        if name == "cytoscnpy":
            return False
        raise AssertionError(name)

    def getoption(self, name: str, default: object | None = None) -> Any:
        assert name == "--cytoscnpy"
        return True


def test_sessionfinish_fails_on_nonjson_nonzero_output(monkeypatch, tmp_path):
    session: Any = SimpleNamespace(
        config=_DummyConfig(tmp_path), exitstatus=0, stash={}
    )
    result = SimpleNamespace(
        returncode=2,
        stdout="not valid json at all",
        stderr="fatal: analyzer crashed",
    )

    monkeypatch.setattr(pytest_plugin.subprocess, "run", lambda *args, **kwargs: result)

    pytest_plugin.pytest_sessionstart(session)
    pytest_plugin.pytest_sessionfinish(session, 0)

    assert session.stash[pytest_plugin.ERROR_KEY] == "fatal: analyzer crashed"
    assert session.exitstatus == 1


def test_collection_modifyitems_adds_files_from_scan_path(monkeypatch, tmp_path):
    scan_path = tmp_path / "src"
    nested = scan_path / "pkg"
    nested.mkdir(parents=True)
    target = nested / "module.py"
    target.write_text("VALUE = 1\n", encoding="utf-8")

    collected_paths: list[Path] = []
    items: list[Any] = []
    session: Any = SimpleNamespace(stash={pytest_plugin.SCAN_PATH_KEY: scan_path})
    config: Any = _DummyConfig(tmp_path)

    class _Collector:
        def __init__(self, path: Path) -> None:
            self.path = path

        def collect(self) -> list[str]:
            return [str(self.path)]

    def _fake_from_parent(*, parent, path):
        assert parent is session
        collected_paths.append(path)
        return _Collector(path)

    monkeypatch.setattr(pytest_plugin.CytoScnPyFile, "from_parent", _fake_from_parent)

    pytest_plugin.pytest_collection_modifyitems(session, config, items)

    assert collected_paths == [target]
    assert items == [str(target)]


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
