"""Tests for benchmark result verification against ground truth."""

import json
import sys
from pathlib import Path
from unittest.mock import patch

import pytest

# Add benchmark directory to sys.path
BENCHMARK_DIR = Path(__file__).resolve().parent.parent
sys.path.append(str(BENCHMARK_DIR))

import benchmark_and_verify  # noqa: E402


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
