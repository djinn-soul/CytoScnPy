"""Tests for running benchmark tools and the main benchmark flow."""

import json
import sys
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

# Add benchmark directory to sys.path
BENCHMARK_DIR = Path(__file__).resolve().parent.parent
sys.path.append(str(BENCHMARK_DIR))

import benchmark_and_verify  # noqa: E402


def test_run_benchmark_tool_parsing():
    """Test output parsing for all supported tools in run_benchmark_tool."""

    test_cases = [
        (
            "CytoScnPy (Rust)",
            json.dumps({"unused_functions": [1], "unused_imports": [2, 3]}),
            3,
        ),
        ("CytoScnPy (Python)", json.dumps({"unused_classes": [1]}), 1),
        (
            "Ruff",
            json.dumps([{}, {}, {}]),  # List format
            3,
        ),
        ("Flake8", "file.py:1:1: F401 unused\nfile.py:2:1: F401 unused", 2),
        ("Vulture", "file.py:1: unused function 'foo' (60%)", 1),
        ("deadcode", "file.py:1:0: DC02 Function `foo` is never used", 1),
        (
            "Pylint",  # JSON format
            json.dumps([{}, {}]),
            2,
        ),
        (
            "uncalled",
            "Functions unused: 10\n file.py: unused function",  # Heuristic based on line count containing 'unused'?
            # Logic: lines with 'unused' in lower()
            2,
        ),
    ]

    for tool_name, stdout, expected_count in test_cases:
        with patch("benchmark_and_verify.run_command") as mock_run:
            mock_res = MagicMock()
            mock_res.returncode = 0 if expected_count == 0 else 1
            mock_res.stdout = stdout
            mock_res.stderr = ""

            # result, duration, rss
            mock_run.return_value = (mock_res, 0.5, 100.0)

            # Need to patch print to avoid noise?
            with patch("builtins.print"):
                result = benchmark_and_verify.run_benchmark_tool(tool_name, ["cmd"])

            assert result["name"] == tool_name
            assert result["issues"] == expected_count
            assert result["memory_mb"] == 100.0


def test_run_benchmark_tool_missing():
    """Test behavior when command is missing."""
    with patch("builtins.print"):
        # run_benchmark_tool returns None if command is falsy
        assert benchmark_and_verify.run_benchmark_tool("Tool", None) is None
        assert benchmark_and_verify.run_benchmark_tool("Tool", []) is None


def test_main_flow(tmp_path):
    """Test full main execution flow."""
    # Create dummy GT file
    gt_path = tmp_path / "gt.json"
    gt_path.write_text('{"files": {}}')

    with patch(
        "sys.argv",
        ["script", "-i", "Ruff"],
    ):
        with patch("benchmark_and_verify.check_tool_availability") as mock_check:
            # Setup tool availability
            mock_check.return_value = {"Ruff": {"available": True, "reason": "OK"}}

            with patch("benchmark_and_verify.run_benchmark_tool") as mock_run:
                # Setup tool result
                mock_run.return_value = {
                    "name": "Ruff",
                    "time": 0.5,
                    "memory_mb": 50,
                    "issues": 5,
                    "stdout": json.dumps([]),
                    "output": "",
                }

                # Mock Verification class completely to avoid file reads
                with patch("benchmark_and_verify.Verification") as mock_verifier:
                    mock_v_instance = mock_verifier.return_value
                    # Setup compare result
                    mock_v_instance.compare.return_value = {
                        "overall": {
                            "TP": 5,
                            "FP": 0,
                            "FN": 0,
                            "Precision": 1.0,
                            "Recall": 1.0,
                            "F1": 1.0,
                            "missed_items": [],
                        }
                    }

                    # No need to patch load_config, we use existing configuration logic with filtered tools
                    with patch("builtins.print"):
                        benchmark_and_verify.main()

                    # Verify calls
                    mock_check.assert_called()
                    mock_run.assert_called()
                    mock_v_instance.compare.assert_called()


def test_main_regression(tmp_path):
    """Test regression detection."""
    baseline_path = tmp_path / "baseline.json"
    baseline_path.write_text(
        json.dumps(
            {
                "results": [
                    {
                        "name": "CytoScnPy (Rust)",
                        "time": 0.1,  # Much faster than 2.0 mocked return
                        "memory_mb": 10.0,  # Much smaller than 100
                        "f1_score": 1.0,
                        "precision": 1.0,
                        "recall": 1.0,
                    }
                ]
            }
        )
    )

    # We force a regression
    gt_path = tmp_path / "gt.json"
    with patch(
        "sys.argv",
        [
            "script",
            "-i",
            "CytoScnPy (Rust)",
            "--compare-json",
            str(baseline_path),
            "--threshold",
            "0.1",
        ],
    ):
        gt_path.write_text('{"files": {}}')
        with patch(
            "benchmark_and_verify.check_tool_availability",
            return_value={"CytoScnPy (Rust)": {"available": True, "reason": "OK"}},
        ):
            with patch("benchmark_and_verify.run_benchmark_tool") as mock_run:
                mock_run.return_value = {
                    "name": "CytoScnPy (Rust)",
                    "time": 2.0,
                    "memory_mb": 100.0,
                    "issues": 5,
                    "stdout": "",
                    "output": "",
                }
                with patch("benchmark_and_verify.Verification") as mock_v:
                    # Simulate score regression
                    mock_v.return_value.compare.return_value = {
                        "overall": {
                            "TP": 5,
                            "FP": 0,
                            "FN": 0,
                            "Precision": 0.8,
                            "Recall": 0.8,
                            "F1": 0.8,
                            "missed_items": [],
                        },
                    }

                    with patch("builtins.print"):
                        with patch("subprocess.run") as mock_subprocess:
                            mock_subprocess.return_value.returncode = 0
                            with pytest.raises(SystemExit) as exc:
                                benchmark_and_verify.main()
                            assert exc.value.code == 1


def test_main_save_json(tmp_path):
    """Test saving results."""
    out_path = tmp_path / "results.json"

    gt_path = tmp_path / "gt.json"
    with patch(
        "sys.argv",
        [
            "script",
            "-i",
            "Ruff",
            "--save-json",
            str(out_path),
        ],
    ):
        gt_path.write_text('{"files": {}}')
        with patch(
            "benchmark_and_verify.check_tool_availability",
            return_value={"Ruff": {"available": True}},
        ):
            with patch(
                "benchmark_and_verify.run_benchmark_tool",
                return_value={
                    "name": "Ruff",
                    "time": 0.1,
                    "memory_mb": 10,
                    "issues": 0,
                    "stdout": "",
                    "output": "",
                },
            ):
                with patch("benchmark_and_verify.Verification") as mock_v:
                    mock_v.return_value.compare.return_value = {
                        "overall": {
                            "TP": 0,
                            "FP": 0,
                            "FN": 0,
                            "Precision": 0,
                            "Recall": 0,
                            "F1": 0,
                            "missed_items": [],
                        }
                    }
                    with patch("builtins.print"):
                        benchmark_and_verify.main()

    assert out_path.exists()
