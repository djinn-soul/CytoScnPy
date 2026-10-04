"""Every supported scanner category must survive pytest normalization."""

import json
from pathlib import Path
from types import SimpleNamespace
from typing import cast

import pytest

from cytoscnpy import pytest_plugin
from cytoscnpy.pytest_findings import group_by_file


@pytest.mark.parametrize(
    "category, finding",
    [
        ("missing_dependencies", "requests"),
        ("unused_dependencies", {"package_name": "requests", "source": "Pyproject"}),
        ("clones", {"file": "app.py", "line": 1, "message": "duplicate function"}),
        ("scan_errors", {"file": "app.py", "error": "failed to read"}),
    ],
)
def test_new_categories_make_pytest_fail(
    monkeypatch, tmp_path: Path, category: str, finding: object
) -> None:
    """Non-source findings fail even if no synthetic file item can be created."""
    config = SimpleNamespace(
        rootpath=tmp_path,
        getoption=lambda *args, **kwargs: True,
        getini=lambda name: "." if name == "cytoscnpy_path" else True,
    )
    session = cast(
        pytest.Session, SimpleNamespace(config=config, stash={}, exitstatus=0)
    )
    file = tmp_path / "app.py"
    if isinstance(finding, dict):
        finding = {**finding, "file": str(file)} if "file" in finding else finding
    data = {category: [finding]}
    monkeypatch.setattr(
        pytest_plugin, "_run_scan", lambda path: (0, json.dumps(data), "")
    )
    pytest_plugin.pytest_sessionstart(session)
    by_file = session.stash[pytest_plugin.BY_FILE_KEY]
    assert by_file
    if category in {"missing_dependencies", "unused_dependencies"}:
        assert str(tmp_path / "pyproject.toml") in by_file
        pytest_plugin.pytest_sessionfinish(session, 0)
        assert session.exitstatus == pytest.ExitCode.TESTS_FAILED
    else:
        assert str(file) in by_file
        item = cast(
            pytest_plugin.CytoScnPyItem, SimpleNamespace(session=session, fspath=file)
        )
        with pytest.raises(pytest_plugin.CytoScnPyError):
            pytest_plugin.CytoScnPyItem.runtest(item)


def test_requirements_and_structured_scan_errors_keep_their_paths(
    tmp_path: Path,
) -> None:
    """Paths and error context remain visible in normalized messages."""
    result = group_by_file(
        {
            "unused_dependencies": [
                {
                    "package_name": "pytest",
                    "source": {"Requirements": "requirements.txt"},
                }
            ],
            "scan_errors": [{"path": "locked", "reason": "permission denied"}],
        },
        tmp_path,
    )
    assert "pytest" in result[str(tmp_path / "requirements.txt")][0]
    assert "permission denied" in result["locked"][0]
