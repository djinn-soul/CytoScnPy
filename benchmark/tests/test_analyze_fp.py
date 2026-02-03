import json
import sys
from pathlib import Path
from unittest.mock import patch

import pytest

# Add benchmark directory to sys.path so we can import the script
BENCHMARK_DIR = Path(__file__).resolve().parent.parent
sys.path.append(str(BENCHMARK_DIR))

try:
    import analyze_fp_fn
except ImportError:
    pytest.fail("Could not import analyze_fp_fn from benchmark directory")


def test_normalize_path():
    """Test path normalization."""
    # Test strict equality, case insensitivity, and separator handling
    assert (
        analyze_fp_fn.normalize_path("C:\\Users\\Test\\File.py")
        == "c:/users/test/file.py"
    )
    # normalize_path uses strip("/") so it removes leading slashes
    assert (
        analyze_fp_fn.normalize_path("/usr/local/bin/script.py")
        == "usr/local/bin/script.py"
    )
    assert analyze_fp_fn.normalize_path("Mixed/Case/Path.PY") == "mixed/case/path.py"
    assert analyze_fp_fn.normalize_path("/Trailing/Slash/") == "trailing/slash"


def test_match_items_exact():
    """Test exact matching logic."""
    f_key = ("path/to/file.py", "function", "my_func", 10)
    truth_keys = [
        ("path/to/file.py", "function", "my_func", 10),
        ("other/file.py", "function", "other", 20),
    ]

    match = analyze_fp_fn.match_items(f_key, truth_keys)
    assert match == truth_keys[0]


def test_match_items_fuzzy_line():
    """Test matching with line tolerance."""
    f_key = ("path/to/file.py", "function", "my_func", 10)
    # Line 12 is within +/- 2 tolerance of 10
    truth_keys = [
        ("path/to/file.py", "function", "my_func", 12),
    ]

    match = analyze_fp_fn.match_items(f_key, truth_keys)
    assert match == truth_keys[0]


def test_match_items_fuzzy_type():
    """Test function/method type equivalence."""
    f_key = ("path/to/file.py", "method", "my_func", 10)
    truth_keys = [
        ("path/to/file.py", "function", "my_func", 10),
    ]
    match = analyze_fp_fn.match_items(f_key, truth_keys)
    assert match == truth_keys[0]


def test_match_items_no_match():
    """Test no match scenarios."""
    f_key = ("path/to/file.py", "function", "my_func", 10)
    truth_keys = [
        ("path/to/file.py", "function", "other_func", 10),  # Dif name
        ("path/to/file.py", "class", "my_func", 10),  # Dif type
        (
            "other/diff_file.py",
            "function",
            "my_func",
            10,
        ),  # Dif path AND distinct basename
        ("path/to/file.py", "function", "my_func", 20),  # Dif line > 2
    ]

    match = analyze_fp_fn.match_items(f_key, truth_keys)
    assert match is None


def test_load_ground_truth(tmp_path):
    """Test loading ground truth from JSON files."""
    # Create struct:
    # root/
    #   subdir/
    #     ground_truth.json

    subdir = tmp_path / "subdir"
    subdir.mkdir()

    gt_file = subdir / "ground_truth.json"
    content = {
        "files": {
            "test.py": {
                "dead_items": [
                    {"type": "function", "name": "foo", "line_start": 5},
                    {"type": "variable", "name": "bar", "suppressed": True},
                    {"type": "class", "name": "Baz"},
                ]
            }
        }
    }

    import json

    gt_file.write_text(json.dumps(content))

    truth, covered = analyze_fp_fn.load_ground_truth(str(tmp_path))

    # Verify covered files
    expected_path = analyze_fp_fn.normalize_path(str(subdir / "test.py"))
    assert expected_path in covered

    # Verify truth items (suppressed should be skipped)
    # Key format: (norm_path, type, name, line)
    keys = list(truth.keys())
    assert len(keys) == 2  # foo and Baz, bar is suppressed

    foo_key = next(k for k in keys if k[2] == "foo")
    assert foo_key[1] == "function"
    assert foo_key[3] == 5

    baz_key = next(k for k in keys if k[2] == "Baz")
    assert baz_key[1] == "class"
    assert baz_key[3] is None


def test_load_cytoscnpy_output(tmp_path):
    """Test loading tool output."""
    data = {
        "unused_functions": [
            {"file": str(tmp_path / "test.py"), "name": "foo", "line": 10}
        ]
    }

    with patch("subprocess.run") as mock_run:
        mock_run.return_value.stdout = json.dumps(data)
        mock_run.return_value.stderr = ""
        mock_run.return_value.returncode = 0

        findings = analyze_fp_fn.load_cytoscnpy_output(str(tmp_path))
        assert len(findings) == 1

        f = next(iter(findings.values()))
        assert f["name"] == "foo"


def test_main(capsys):
    """Test main execution logic via mocks."""
    # Patch main script functions
    with patch("analyze_fp_fn.load_ground_truth") as mock_gt:
        with patch("analyze_fp_fn.load_cytoscnpy_output") as mock_load:
            # Setup data: 1 TP, 1 FP, 1 FN
            # Truth: foo (matched), baz (missed)
            mock_gt.return_value = (
                {
                    ("path.py", "function", "foo", 10): {
                        "suppressed": False,
                        "type": "function",
                        "name": "foo",
                        "file": "path.py",
                        "line_start": 10,
                    },
                    ("path.py", "function", "baz", 30): {
                        "suppressed": False,
                        "type": "function",
                        "name": "baz",
                        "file": "path.py",
                        "line_start": 30,
                    },
                },
                ["path.py"],
            )

            # Findings: foo (correct), bar (extra)
            mock_load.return_value = {
                ("path.py", "function", "foo", 10): {
                    "file": "path.py",
                    "name": "foo",
                    "line": 10,
                    "def_type": "function",
                },
                ("path.py", "function", "bar", 20): {
                    "file": "path.py",
                    "name": "bar",
                    "line": 20,
                    "def_type": "function",
                },
            }

            # Run main - we need to patch sys.argv or just rely on default args?
            # Main calls load_ground_truth(base_dir) with hardcoded path usually or arg?
            # Looking at source: base_dir = r"..." (hardcoded).
            # So arguments don't matter, it mocks the calls.

            # We patch print locally to ensure we catch output? capsys does that.
            analyze_fp_fn.main()

            captured = capsys.readouterr()

            # Verify that metrics are correctly calculated

            assert "FALSE POSITIVES" in captured.out
            assert "FALSE NEGATIVES" in captured.out


def test_load_cytoscnpy_output_empty(tmp_path):
    """Test handling of empty/failed tool output."""
    with patch("subprocess.run") as mock_run:
        mock_run.return_value.stdout = ""
        mock_run.return_value.stderr = "Error"
        mock_run.return_value.returncode = 1

        with patch("builtins.print"):
            findings = analyze_fp_fn.load_cytoscnpy_output(str(tmp_path))
            assert findings == {}


def test_load_cytoscnpy_output_parameter(tmp_path):
    """Test parameter type fallback in loading."""
    # Data with "parameter" def_type
    data = {
        "unused_variables": [
            {"file": "t.py", "name": "p", "line": 1, "def_type": "parameter"}
        ]
    }
    with patch("subprocess.run") as mock_run:
        mock_run.return_value.stdout = json.dumps(data)
        mock_run.return_value.returncode = 0

        findings = analyze_fp_fn.load_cytoscnpy_output(str(tmp_path))
        key = next(iter(findings.keys()))
        assert key[1] == "variable"


def test_main_filtering_covered_files(capsys):
    """Test that main logic filters findings based on covered files."""
    # Scenario:
    # Ground Truth covers 'covered.py'.
    # Findings include 'covered.py' (FP) and 'ignored.py' (FP).
    # 'ignored.py' finding should be dropped and not counted in FP stats.

    with patch("analyze_fp_fn.load_ground_truth") as mock_gt:
        with patch("analyze_fp_fn.load_cytoscnpy_output") as mock_load:
            # 1. Ground Truth
            mock_gt.return_value = (
                {},  # No truth items
                {"covered.py"},  # Only covered.py is in scope
            )

            # 2. Findings
            mock_load.return_value = {
                ("covered.py", "function", "fp1", 1): {
                    "file": "covered.py",
                    "name": "fp1",
                    "line": 1,
                },
                ("ignored.py", "function", "fp2", 1): {
                    "file": "ignored.py",
                    "name": "fp2",
                    "line": 1,
                },
            }

            analyze_fp_fn.main()
            captured = capsys.readouterr()

            # FP should be 1 (covered.py), not 2
            assert "FP: 1" in captured.out
