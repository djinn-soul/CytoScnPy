"""Regression tests for Python scan validation and clone error propagation."""

from pathlib import Path

import pytest

import cytoscnpy


@pytest.mark.parametrize("confidence", [101, 255])
def test_scan_rejects_confidence_above_100(tmp_path: Path, confidence: int) -> None:
    """Both programmatic entry points enforce the documented percentage range."""
    (tmp_path / "app.py").write_text("value = 1\n", encoding="utf-8")
    with pytest.raises(ValueError, match=r"confidence.*0.*100"):
        cytoscnpy.scan(tmp_path, confidence=confidence)
    with pytest.raises(ValueError, match=r"confidence.*0.*100"):
        cytoscnpy.scan_code("value = 1\n", confidence=confidence)


def test_scan_rejects_invalid_config_confidence(tmp_path: Path) -> None:
    """Resolved configuration is validated even without a Python override."""
    (tmp_path / "pyproject.toml").write_text(
        "[tool.cytoscnpy]\nconfidence = 101\n", encoding="utf-8"
    )
    with pytest.raises(ValueError, match=r"confidence.*0.*100"):
        cytoscnpy.scan(tmp_path)


@pytest.mark.parametrize("similarity", [-0.1, 1.1, float("nan"), float("inf")])
def test_requested_clone_failure_is_not_an_empty_success(
    tmp_path: Path, similarity: float
) -> None:
    """A failed requested analyzer raises its original error context."""
    (tmp_path / "app.py").write_text("def run():\n    return 1\n", encoding="utf-8")
    with pytest.raises(RuntimeError, match=r"Clone analysis failed.*similarity"):
        cytoscnpy.scan(tmp_path, clones=True, clone_similarity=similarity)


@pytest.mark.parametrize("confidence", [0, 100])
def test_valid_confidence_boundaries_remain_supported(
    tmp_path: Path, confidence: int
) -> None:
    """Inclusive endpoints remain valid for both entry points."""
    assert isinstance(cytoscnpy.scan(tmp_path, confidence=confidence), dict)
    assert isinstance(cytoscnpy.scan_code("value = 1\n", confidence=confidence), dict)
