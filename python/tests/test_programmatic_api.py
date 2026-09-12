"""Tests for the first-class programmatic Python API (scan, scan_code)."""

from __future__ import annotations

import json
from pathlib import Path

import cytoscnpy


def test_scan_code_detects_unused_function() -> None:
    """Test that scan_code detects unused functions directly from code snippet."""
    code = "def dead_func():\n    pass\n"
    res = cytoscnpy.scan_code(code, filename="example.py")
    assert isinstance(res, dict)
    assert "unused_functions" in res
    assert len(res["unused_functions"]) == 1
    finding = res["unused_functions"][0]
    assert finding["simple_name"] == "dead_func"
    assert finding["file"] == "example.py"
    assert finding["line"] == 1


def test_scan_code_detects_dangerous_pattern() -> None:
    """Test that scan_code detects dangerous code patterns like eval."""
    code = "def runner(user_input):\n    eval(user_input)\n"
    res = cytoscnpy.scan_code(code, filename="danger.py", danger=True)
    assert isinstance(res, dict)
    assert "danger" in res
    assert len(res["danger"]) >= 1
    assert any("eval" in str(d.get("message", "")).lower() or d.get("rule_id", "") == "CSP-D001" for d in res["danger"])


def test_scan_code_detects_secret() -> None:
    """Test that scan_code detects hard-coded secrets."""
    code = 'AWS_KEY = "AKIA1234567890ABCDEF"\n'
    res = cytoscnpy.scan_code(code, filename="secret.py", secrets=True)
    assert isinstance(res, dict)
    assert "secrets" in res
    assert len(res["secrets"]) >= 1


def test_scan_file_path(tmp_path: Path) -> None:
    """Test that cytoscnpy.scan analyzes a Python file path."""
    file_path = tmp_path / "sample.py"
    file_path.write_text("def unused_a(): pass\ndef unused_b(): pass\n", encoding="utf-8")

    res = cytoscnpy.scan(file_path)
    assert isinstance(res, dict)
    assert "unused_functions" in res
    names = {f["simple_name"] for f in res["unused_functions"]}
    assert "unused_a" in names
    assert "unused_b" in names
    assert res["analysis_summary"]["total_files"] == 1


def test_scan_directory_path(tmp_path: Path) -> None:
    """Test that cytoscnpy.scan analyzes a directory containing Python files."""
    f1 = tmp_path / "mod1.py"
    f2 = tmp_path / "mod2.py"
    f1.write_text("def dead_mod1(): pass\n", encoding="utf-8")
    f2.write_text("def live_mod2(): pass\nlive_mod2()\n", encoding="utf-8")

    res = cytoscnpy.scan(str(tmp_path))
    assert isinstance(res, dict)
    assert "unused_functions" in res
    unused = [f["simple_name"] for f in res["unused_functions"]]
    assert "dead_mod1" in unused
    assert "live_mod2" not in unused


def test_scan_sequence_of_paths(tmp_path: Path) -> None:
    """Test that scan accepts a sequence of Path and str objects."""
    f1 = tmp_path / "a.py"
    f2 = tmp_path / "b.py"
    f1.write_text("import sys\n", encoding="utf-8")
    f2.write_text("import os\n", encoding="utf-8")

    res = cytoscnpy.scan([f1, str(f2)])
    assert isinstance(res, dict)
    assert res["analysis_summary"]["total_files"] == 2


def test_scan_json_raw_string(tmp_path: Path) -> None:
    """Test that scan_json returns a valid JSON string."""
    f = tmp_path / "test.py"
    f.write_text("x = 1\n", encoding="utf-8")

    raw_json = cytoscnpy.scan_json(paths=[str(f)])
    assert isinstance(raw_json, str)
    data = json.loads(raw_json)
    assert isinstance(data, dict)
    assert "analysis_summary" in data


def test_scan_code_json_raw_string() -> None:
    """Test that scan_code_json returns a valid JSON string."""
    raw_json = cytoscnpy.scan_code_json(code="y = 2\n")
    assert isinstance(raw_json, str)
    data = json.loads(raw_json)
    assert isinstance(data, dict)
    assert "analysis_summary" in data
