from __future__ import annotations

import importlib
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import pytest

import cytoscnpy.pytest_plugin as pytest_plugin


class _DummyGroup:
    def __init__(self) -> None:
        self.options: list[tuple[tuple[Any, ...], dict[str, Any]]] = []

    def addoption(self, *args: Any, **kwargs: Any) -> None:
        self.options.append((args, kwargs))


class _DummyParser:
    def __init__(self) -> None:
        self.group = _DummyGroup()
        self.ini: list[tuple[tuple[Any, ...], dict[str, Any]]] = []

    def getgroup(self, name: str, description: str) -> _DummyGroup:
        assert name == "cytoscnpy"
        assert "static analysis" in description
        return self.group

    def addini(self, *args: Any, **kwargs: Any) -> None:
        self.ini.append((args, kwargs))


class _DummyConfig:
    def __init__(
        self,
        rootpath: Path,
        *,
        enabled: bool = True,
        ini_enabled: bool = False,
        scan_path: str = ".",
    ) -> None:
        self.rootpath = rootpath
        self.enabled = enabled
        self.ini_enabled = ini_enabled
        self.scan_path = scan_path

    def getini(self, name: str) -> Any:
        if name == "cytoscnpy_path":
            return self.scan_path
        if name == "cytoscnpy":
            return self.ini_enabled
        raise AssertionError(name)

    def getoption(self, name: str, default: object | None = None) -> Any:
        assert name == "--cytoscnpy"
        return self.enabled


def test_package_imports_reload_under_coverage():
    import cytoscnpy

    reloaded_package = importlib.reload(cytoscnpy)
    reloaded_plugin = importlib.reload(pytest_plugin)

    assert "run" in reloaded_package.__all__
    assert reloaded_plugin.BY_FILE_KEY is pytest_plugin.BY_FILE_KEY


def test_pytest_addoption_registers_flag_and_ini_options():
    parser = _DummyParser()

    pytest_plugin.pytest_addoption(parser)  # type: ignore[arg-type]

    assert parser.group.options[0][0] == ("--cytoscnpy",)
    assert [entry[0][0] for entry in parser.ini] == [
        "cytoscnpy",
        "cytoscnpy_path",
    ]


def test_is_enabled_accepts_cli_or_ini(tmp_path):
    assert pytest_plugin._is_enabled(_DummyConfig(tmp_path, enabled=True))
    assert pytest_plugin._is_enabled(
        _DummyConfig(tmp_path, enabled=False, ini_enabled=True)
    )
    assert not pytest_plugin._is_enabled(
        _DummyConfig(tmp_path, enabled=False, ini_enabled=False)
    )


def test_sessionstart_returns_when_plugin_disabled(tmp_path):
    session: Any = SimpleNamespace(
        config=_DummyConfig(tmp_path, enabled=False, ini_enabled=False),
        stash={},
    )

    pytest_plugin.pytest_sessionstart(session)

    assert session.stash == {}


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


def test_sessionstart_valid_json_nonzero_sets_force_fail(monkeypatch, tmp_path):
    session: Any = SimpleNamespace(
        config=_DummyConfig(tmp_path), exitstatus=0, stash={}
    )
    result = SimpleNamespace(returncode=3, stdout="{}", stderr="")

    monkeypatch.setattr(pytest_plugin.subprocess, "run", lambda *args, **kwargs: result)

    pytest_plugin.pytest_sessionstart(session)

    assert session.stash[pytest_plugin.FORCE_FAIL_KEY] is True
    assert session.stash[pytest_plugin.ERROR_KEY] == "cytoscnpy exited with status 3"


def test_sessionstart_uses_configured_scan_path(monkeypatch, tmp_path):
    session: Any = SimpleNamespace(
        config=_DummyConfig(tmp_path, scan_path="src"), exitstatus=0, stash={}
    )
    result = SimpleNamespace(returncode=0, stdout="{}", stderr="")
    calls: list[list[str]] = []

    def _run(args: list[str], **kwargs: Any) -> Any:
        calls.append(args)
        return result

    monkeypatch.setattr(pytest_plugin.subprocess, "run", _run)

    pytest_plugin.pytest_sessionstart(session)

    assert session.stash[pytest_plugin.SCAN_PATH_KEY] == tmp_path / "src"
    assert calls[0][-2:] == [str(tmp_path / "src"), "--json"]


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


def test_collection_modifyitems_returns_when_not_enabled_or_no_scan_path(tmp_path):
    items: list[Any] = []
    disabled_config: Any = _DummyConfig(tmp_path, enabled=False, ini_enabled=False)
    enabled_config: Any = _DummyConfig(tmp_path)

    pytest_plugin.pytest_collection_modifyitems(
        SimpleNamespace(stash={}), disabled_config, items
    )
    pytest_plugin.pytest_collection_modifyitems(
        SimpleNamespace(stash={}), enabled_config, items
    )

    assert items == []


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

    rendered = pytest_plugin.CytoScnPyItem.repr_failure(fake_item, excinfo)

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
