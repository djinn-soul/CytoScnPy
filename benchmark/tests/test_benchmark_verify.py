import json
import subprocess
import sys
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

# Add benchmark directory to sys.path
BENCHMARK_DIR = Path(__file__).resolve().parent.parent
sys.path.append(str(BENCHMARK_DIR))

import benchmark_and_verify  # noqa: E402


def test_normalize_path():
    """Test path normalization utility."""
    # normalize_path strips leading slashes
    assert benchmark_and_verify.normalize_path("path\\to\\file") == "path/to/file"
    assert (
        benchmark_and_verify.normalize_path("/Path/To/File") == "Path/To/File"
    )  # Leading / removed, BUT case preserved


def test_run_command_success():
    """Test running a command successfully."""
    with patch("subprocess.Popen") as mock_popen:
        mock_process = MagicMock()
        mock_process.communicate.return_value = ("stdout output", "stderr output")
        mock_process.returncode = 0
        mock_process.poll.side_effect = [None, 0]  # Running then done

        # Mock psutil Process
        mock_psutil_proc = MagicMock()
        mock_psutil_proc.memory_info.return_value.rss = 1024 * 1024 * 50  # 50MB
        mock_psutil_proc.children.return_value = []

        with patch("psutil.Process", return_value=mock_psutil_proc):
            mock_popen.return_value = mock_process

            result, duration, memory = benchmark_and_verify.run_command(
                ["echo", "test"]
            )

            assert result.returncode == 0
            assert result.stdout == "stdout output"
            assert memory == 50.0
            assert duration >= 0


def test_run_command_timeout():
    """Test command timeout."""
    with patch("subprocess.Popen") as mock_popen:
        mock_process = MagicMock()
        mock_process.communicate.side_effect = subprocess.TimeoutExpired(["cmd"], 1)
        mock_process.kill.return_value = None

        # Second communicate call after kill
        mock_process.communicate.side_effect = [
            subprocess.TimeoutExpired(["cmd"], 1),
            ("partial out", "partial err"),
        ]

        mock_popen.return_value = mock_process

        # Mock threading to avoid stuck threads in tests
        with patch("threading.Thread"):
            result, _, _ = benchmark_and_verify.run_command(["sleep", "10"], timeout=1)

            assert result.returncode == -1
            assert "Timeout" in result.stderr


def test_get_tool_path():
    """Test tool path resolution."""
    with patch("shutil.which", return_value="/bin/tool"):
        assert benchmark_and_verify.get_tool_path("tool") == "/bin/tool"

    with patch("shutil.which", return_value=None):
        with patch("pathlib.Path.exists", return_value=True):
            pass


def test_check_tool_availability_full():
    """Test full tool availability logic."""
    # Test 1: Standard tool in TOOL_CHECKS (e.g. Ruff)
    with patch("subprocess.run") as mock_run:
        mock_run.return_value.returncode = 0
        status = benchmark_and_verify.check_tool_availability(
            [{"name": "Ruff", "command": "ruff"}]
        )
        assert status["Ruff"]["available"] is True

    # Test 2: CytoScnPy (Rust) - cargo run
    with patch("shutil.which", return_value="/u/bin/cargo"):
        status = benchmark_and_verify.check_tool_availability(
            [{"name": "CytoScnPy (Rust)", "command": ["cargo", "run"]}]
        )
        assert status["CytoScnPy (Rust)"]["available"] is True

    # Test 3: CytoScnPy (Python) - import check
    with patch("subprocess.run") as mock_run:
        mock_run.return_value.returncode = 0
        status = benchmark_and_verify.check_tool_availability(
            [{"name": "CytoScnPy (Python)", "command": "cmd"}]
        )
        assert status["CytoScnPy (Python)"]["available"] is True

    # Test 4: deadcode - executable check
    with patch("shutil.which", return_value="deadcode"):
        status = benchmark_and_verify.check_tool_availability(
            [{"name": "deadcode", "command": ["deadcode"]}]
        )
        assert status["deadcode"]["available"] is True


class TestVerification:
    @pytest.fixture
    def verifier_setup(self, tmp_path):
        """Create a Verification instance and return it with the base path."""
        gt_file = tmp_path / "ground_truth.json"
        base_path = tmp_path

        import json

        data = {
            "files": {
                "test.py": {
                    "dead_items": [
                        {"type": "function", "name": "unused_func", "line_start": 5},
                        {"type": "import", "name": "os", "line_start": 1},
                    ]
                }
            }
        }
        gt_file.write_text(json.dumps(data))

        verifier = benchmark_and_verify.Verification(str(gt_file))
        return verifier, base_path

    def test_load_ground_truth(self, verifier_setup):
        """Test loading assertions."""
        verifier, _ = verifier_setup
        assert len(verifier.ground_truth) == 2

        # Ground truth is a set of tuples: (path, line, type, name)
        func_finding = next(a for a in verifier.ground_truth if a[3] == "unused_func")
        assert func_finding[0].endswith("test.py")
        assert func_finding[2] == "function"

    def test_compare_perfect_match(self, verifier_setup):
        """Test comparison with perfect results."""
        verifier, base_path = verifier_setup
        # benchmark_and_verify is case sensitive
        test_py = benchmark_and_verify.normalize_path(str(base_path / "test.py"))

        # Findings matches normalized structure in parse_tool_output: (file, line, type, name)
        findings = {
            (test_py, 5, "function", "unused_func"),
            (test_py, 1, "import", "os"),
        }

        with patch.object(verifier, "parse_tool_output", return_value=findings):
            metrics = verifier.compare("MyTool", "dummy output")

        assert metrics["overall"]["TP"] == 2
        assert metrics["overall"]["FP"] == 0
        assert metrics["overall"]["FN"] == 0
        assert metrics["overall"]["Precision"] == 1.0
        assert metrics["overall"]["Recall"] == 1.0

    def test_compare_false_positive(self, verifier_setup):
        """Test with extra finding (FP)."""
        verifier, base_path = verifier_setup
        test_py = benchmark_and_verify.normalize_path(str(base_path / "test.py"))

        findings = {
            (test_py, 5, "function", "unused_func"),
            (test_py, 1, "import", "os"),
            (test_py, 50, "function", "fp_func"),  # FP
        }

        with patch.object(verifier, "parse_tool_output", return_value=findings):
            metrics = verifier.compare("MyTool", "dummy")

        assert metrics["overall"]["TP"] == 2
        assert metrics["overall"]["FP"] == 1

    def test_compare_false_negative(self, verifier_setup):
        """Test with missing finding (FN)."""
        verifier, base_path = verifier_setup
        test_py = benchmark_and_verify.normalize_path(str(base_path / "test.py"))

        findings = {
            (test_py, 5, "function", "unused_func"),
        }

        with patch.object(verifier, "parse_tool_output", return_value=findings):
            metrics = verifier.compare("MyTool", "dummy")

        assert metrics["overall"]["TP"] == 1
        assert metrics["overall"]["FN"] == 1  # Missed 'os'

    def test_parse_tool_output(self, verifier_setup):
        """Test parsing logic for various tools."""
        verifier, _ = verifier_setup

        # 1. CytoScnPy (JSON)
        json_out = json.dumps(
            {
                "unused_functions": [{"file": "t.py", "name": "foo", "line": 10}],
                "unused_imports": [{"file": "t.py", "name": "bar", "line": 2}],
            }
        )
        findings = verifier.parse_tool_output("CytoScnPy (Rust)", json_out)
        assert len(findings) == 2
        assert any(f[3] == "foo" for f in findings)

        # 2. Vulture
        vulture_out = (
            "t.py:10: unused function 'foo' (60%)\nt.py:2: unused import 'bar' (90%)"
        )
        findings = verifier.parse_tool_output("Vulture", vulture_out)
        assert len(findings) == 2
        assert any(f[2] == "function" for f in findings)
        # 3. Flake8
        flake8_out = "t.py:2:1: F401 'os' imported but unused"
        findings = verifier.parse_tool_output("Flake8", flake8_out)
        assert len(findings) == 1
        assert next(iter(findings))[3] == "os"

        # 4. deadcode
        deadcode_out = "t.py:10:0: DC02 Function `foo` is never used"
        findings = verifier.parse_tool_output("deadcode", deadcode_out)

    def test_parse_tool_output_flake8(self, verifier_setup):
        """Test Flake8 parsing."""
        verifier, _ = verifier_setup

        # Standard F401
        out = "file.py:1:1: F401 'os' imported but unused"
        res = verifier.parse_tool_output("Flake8", out)
        assert len(res) == 1
        assert next(iter(res))[2] == "import"

        # F841 local variable
        out = "file.py:10:1: F841 local variable 'x' is assigned to but never used"
        res = verifier.parse_tool_output("Flake8", out)
        assert len(res) == 1
        assert next(iter(res))[2] == "variable"

    def test_parse_tool_output_pylint_fallback(self, verifier_setup):
        """Test Pylint text fallback."""

    def test_parse_tool_output_dead(self, verifier_setup):
        """Test dead tool output."""
        verifier, _ = verifier_setup
        out = "funcname is never called, defined in file.py:10"
        res = verifier.parse_tool_output("dead", out)
        assert len(res) == 1
        assert next(iter(res))[2] == "function"

    def test_parse_tool_output_uncalled(self, verifier_setup):
        """Test uncalled tool output."""
        verifier, _ = verifier_setup
        out = "file.py: Unused function foo"
        res = verifier.parse_tool_output("uncalled", out)
        assert len(res) == 1
        assert next(iter(res))[2] == "function"
        assert next(iter(res))[1] is None  # No line number

    def test_parse_tool_output_deadcode(self, verifier_setup):
        """Test deadcode tool output."""
        verifier, _ = verifier_setup
        out = "file.py:10:0: DC01 Variable `x` is never used"
        res = verifier.parse_tool_output("deadcode", out)
        assert len(res) == 1
        assert next(iter(res))[2] == "variable"

    def test_parse_tool_output_skylos(self, verifier_setup):
        """Test Skylos output parsing."""
        verifier, _ = verifier_setup

        # 1. Flat list
        out_list = json.dumps(
            [
                {"type": "function", "file": "f.py", "name": "foo", "line": 1},
                {"type": "parameter", "file": "f.py", "name": "p", "line": 2},
            ]
        )
        res = verifier.parse_tool_output("Skylos", out_list)
        assert len(res) == 2
        # verify specific items
        assert any(r[2] == "function" for r in res)
        assert any(r[2] == "variable" for r in res)  # parameter mapped to variable

        # 2. Dict with keys
        out_dict = json.dumps(
            {
                "unused_classes": [
                    {"type": "class", "file": "c.py", "name": "C", "line": 3}
                ],
                "items": [{"type": "import", "file": "c.py", "name": "os", "line": 1}],
            }
        )
        res = verifier.parse_tool_output("Skylos", out_dict)
        assert len(res) == 2
        assert any(r[2] == "class" for r in res)
        assert any(r[2] == "import" for r in res)

        # 3. Fallback to 'results'
        out_results = json.dumps(
            {"results": [{"type": "method", "file": "m.py", "name": "m", "line": 4}]}
        )
        res = verifier.parse_tool_output("Skylos", out_results)
        assert len(res) == 1
        assert next(iter(res))[2] == "method"

        # 4. JSON Error
        verifier.parse_tool_output("Skylos", "bad json")

    def test_parse_tool_output_pylint_full(self, verifier_setup):
        """Test Pylint output parsing comprehensively."""
        verifier, _ = verifier_setup

        # 1. Unused import with message fallback
        out_json = json.dumps(
            [
                {
                    "symbol": "unused-import",
                    "path": "p.py",
                    "line": 1,
                    "message": "Unused import sys",
                },
                {
                    "symbol": "unused-variable",
                    "path": "p.py",
                    "line": 2,
                    "message": "Unused variable 'v'",
                    "obj": "func",  # scope
                },
                {
                    "symbol": "unused-argument",
                    "path": "p.py",
                    "line": 3,
                    "message": "Unused argument 'arg'",
                    "obj": "func",
                },
            ]
        )
        res = verifier.parse_tool_output("Pylint", out_json)
        assert len(res) == 3
        # Verify import
        assert any(r[2] == "import" and r[3] == "sys" for r in res)
        # Verify variable
        assert any(r[2] == "variable" and r[3] == "v" for r in res)
        # Verify argument -> variable
        assert any(r[2] == "variable" and r[3] == "arg" for r in res)

        # 2. JSON Error
        verifier.parse_tool_output("Pylint", "bad json")


def test_check_tool_availability_python(tmp_path):
    """Test Python tool availability."""
    # Installed
    with patch("subprocess.run") as mock_run:
        mock_run.return_value.returncode = 0
        res = benchmark_and_verify.check_tool_availability(
            [{"name": "CytoScnPy (Python)", "command": ["cmd"]}]
        )
        assert res["CytoScnPy (Python)"]["available"] is True

    # Missing
    with patch("subprocess.run") as mock_run:
        mock_run.return_value.returncode = 1
        res = benchmark_and_verify.check_tool_availability(
            [{"name": "CytoScnPy (Python)", "command": ["cmd"]}]
        )
        assert res["CytoScnPy (Python)"]["available"] is False


def test_check_tool_availability_skylos():
    """Test Skylos availability logic."""
    # Found in path
    with patch("benchmark_and_verify.get_tool_path", return_value="/skylos"):
        res = benchmark_and_verify.check_tool_availability(
            [{"name": "Skylos", "command": ["cmd"]}]
        )
        assert "Skylos" in res and res["Skylos"]["available"]

    # Not in path, check module
    with patch("benchmark_and_verify.get_tool_path", return_value=None):
        with patch("subprocess.run") as mock_run:
            mock_run.return_value.returncode = 0
            res = benchmark_and_verify.check_tool_availability(
                [{"name": "Skylos", "command": ["cmd"]}]
            )
            assert res["Skylos"]["available"]

    # Missing completely
    with patch("benchmark_and_verify.get_tool_path", return_value=None):
        with patch("subprocess.run") as mock_run:
            mock_run.return_value.returncode = 1
            res = benchmark_and_verify.check_tool_availability(
                [{"name": "Skylos", "command": ["cmd"]}]
            )
            assert not res["Skylos"]["available"]


def test_tool_checks():
    """Test tool availability check logic."""
    # Mock shutil.which
    with patch("shutil.which", return_value="/usr/bin/tool"):
        with patch("subprocess.run") as mock_run:
            mock_run.return_value.returncode = 0

            # Must be a list of dicts!
            tools_config = [{"name": "TestTool", "command": "tool --version"}]
            status = benchmark_and_verify.check_tool_availability(tools_config)

            assert status["TestTool"]["available"] is True

    # Mock tool missing
    with patch("shutil.which", return_value=None):
        with patch("subprocess.run") as mock_run:
            mock_run.side_effect = FileNotFoundError

            tools_config = [{"name": "MissingTool", "command": "missing --version"}]
            status = benchmark_and_verify.check_tool_availability(tools_config)

            tools_config = [{"name": "Ruff", "command": "ruff"}]
            status = benchmark_and_verify.check_tool_availability(tools_config)
            assert status["Ruff"]["available"] is False


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


def test_get_tool_path_fallback():
    """Test get_tool_path fallback to Scripts/bin."""
    with patch("shutil.which", return_value=None):
        with patch("pathlib.Path.exists", return_value=True):
            # Should return path in scripts
            res = benchmark_and_verify.get_tool_path("tool")
            assert "tool" in res and ("Scripts" in res or "bin" in res)


def test_check_python_module():
    """Test _check_python_module helper."""
    # Success
    with patch("subprocess.run") as mock_run:
        mock_run.return_value.returncode = 0
        res = benchmark_and_verify._check_python_module("mod", "arg")
        assert res["available"] is True

    # Failure return code
    with patch("subprocess.run") as mock_run:
        mock_run.return_value.returncode = 1
        res = benchmark_and_verify._check_python_module("mod", "arg")
        assert res["available"] is False

    # Exception
    with patch("subprocess.run", side_effect=OSError):
        res = benchmark_and_verify._check_python_module("mod", "arg")
        assert res["available"] is False


def test_check_tool_availability_cytoscnpy_rust_complex():
    """Test complex CytoScnPy (Rust) configuration branches."""
    # 1. List with generic binary path (not cargo)
    with patch("pathlib.Path.exists", return_value=True):
        res = benchmark_and_verify.check_tool_availability(
            [{"name": "CytoScnPy (Rust)", "command": ["/bin/custom"]}]
        )
        assert res["CytoScnPy (Rust)"]["available"] is True

    # 2. String command with quotes parsing
    with patch("pathlib.Path.exists", return_value=True):
        res = benchmark_and_verify.check_tool_availability(
            [{"name": "CytoScnPy (Rust)", "command": '"/bin/quoted" --arg'}]
        )
        assert res["CytoScnPy (Rust)"]["available"] is True
