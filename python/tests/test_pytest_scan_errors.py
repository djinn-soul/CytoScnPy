"""A scanner protocol failure must fail pytest even without synthetic items."""

from types import SimpleNamespace

import pytest

from cytoscnpy import pytest_plugin


@pytest.mark.parametrize("output", ["", "invalid JSON", "[]", "null"])
def test_invalid_scan_output_fails_without_python_files(monkeypatch, tmp_path, output):
    config = SimpleNamespace(
        rootpath=tmp_path,
        getoption=lambda *args, **kwargs: True,
        getini=lambda name: "." if name == "cytoscnpy_path" else True,
    )
    session = SimpleNamespace(config=config, stash={}, exitstatus=0)
    monkeypatch.setattr(pytest_plugin, "_run_scan", lambda path: (0, output, ""))

    pytest_plugin.pytest_sessionstart(session)
    pytest_plugin.pytest_sessionfinish(session, 0)

    assert pytest_plugin._iter_python_files(tmp_path) == []
    assert session.stash[pytest_plugin.ERROR_KEY]
    assert session.exitstatus == pytest.ExitCode.TESTS_FAILED
