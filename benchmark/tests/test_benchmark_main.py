import sys
from pathlib import Path
from unittest.mock import patch

# Add benchmark directory to sys.path
BENCHMARK_DIR = Path(__file__).resolve().parent.parent
sys.path.append(str(BENCHMARK_DIR))

import benchmark_and_verify  # noqa: E402


def test_main_no_tools_selected(capsys):
    """Test when no tools are selected via filters."""
    """Test when no tools are selected via filters."""
    # We use a tool name that definitely doesn't exist in the real config
    with patch("sys.argv", ["script", "--include", "NonExistentTool"]):
        benchmark_and_verify.main()
        captured = capsys.readouterr()
        assert "[-] No tools selected to run." in captured.out


def test_main_no_available_tools(capsys):
    """Test when tools are selected but none are available."""
    """Test when tools are selected but none are available."""
    # We select "Ruff" but mock it as unavailable
    with patch("sys.argv", ["script", "--include", "Ruff"]):
        # Mock check_tool_availability to return False for Ruff
        with patch(
            "benchmark_and_verify.check_tool_availability",
            return_value={"Ruff": {"available": False, "reason": "No"}},
        ):
            benchmark_and_verify.main()
            captured = capsys.readouterr()
            assert "[-] No available tools to run." in captured.out


def test_main_full_flow(tmp_path):
    """Test full execution flow with mocked tools."""
    # Create dummy ground truth
    (tmp_path / "examples").mkdir()
    (tmp_path / "examples/ground_truth.json").write_text("{}")

    # Use Ruff as the target tool
    with patch(
        "sys.argv",
        ["script", "--include", "Ruff", "--save-json", str(tmp_path / "res.json")],
    ):
        with patch("benchmark_and_verify.check_tool_availability") as mock_check:
            mock_check.return_value = {"Ruff": {"available": True, "reason": "Test"}}

            with patch("benchmark_and_verify.run_benchmark_tool") as mock_run:
                mock_run.return_value = {
                    "name": "Ruff",
                    "time": 1.0,
                    "memory_mb": 10.0,
                    "issues": 0,
                    "output": "out",
                    "stdout": "out",
                }

                # Mock target_dir.exists()
                with patch("pathlib.Path.exists", return_value=True):
                    # Mock Verification
                    with patch("benchmark_and_verify.Verification") as mock_ver:
                        inst = mock_ver.return_value
                        inst.compare.return_value = {
                            "overall": {
                                "F1": 1.0,
                                "TP": 0,
                                "FP": 0,
                                "FN": 0,
                                "Precision": 0.0,
                                "Recall": 0.0,
                            }
                        }

                        benchmark_and_verify.main()

                        assert (tmp_path / "res.json").exists()
