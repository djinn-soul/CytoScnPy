import json
import sys
from pathlib import Path
from unittest.mock import patch

import pytest

# Add benchmark directory to sys.path
BENCHMARK_DIR = Path(__file__).resolve().parent.parent
sys.path.append(str(BENCHMARK_DIR))

try:
    import verify_ground_truth
except ImportError:
    pytest.fail("Could not import verify_ground_truth from benchmark directory")


def test_verify_item_missing_fields():
    """Test handling of missing fields in item dict."""
    item = {"type": "function"}  # Missing name and line
    lines = ["code"]
    issue = verify_ground_truth.verify_item(item, lines)
    assert "MISSING FIELDS" in issue


def test_verify_item_line_out_of_bounds():
    """Test line number out of bounds."""
    item = {"type": "function", "name": "foo", "line_start": 100}
    lines = ["line 1", "line 2"]
    issue = verify_ground_truth.verify_item(item, lines)
    assert "LINE OUT OF BOUNDS" in issue


def test_verify_item_function_match():
    """Test matching function definition."""
    item = {"type": "function", "name": "process_data", "line_start": 1}

    # Correct case
    lines = ["def process_data():"]
    assert verify_ground_truth.verify_item(item, lines) is None

    # Async case
    lines = ["async def process_data():"]
    assert verify_ground_truth.verify_item(item, lines) is None

    # Mismatch case
    lines = ["def other_function():"]
    issue = verify_ground_truth.verify_item(item, lines)
    assert "TYPE MISMATCH" in issue
    assert "looking for 'def process_data'" in issue


def test_verify_item_class_match():
    """Test matching class definition."""
    item = {"type": "class", "name": "MyClass", "line_start": 1}

    # Correct
    lines = ["class MyClass:"]
    assert verify_ground_truth.verify_item(item, lines) is None

    # Inherited
    lines = ["class MyClass(Base):"]
    assert verify_ground_truth.verify_item(item, lines) is None

    # Mismatch
    lines = ["class OtherClass:"]
    issue = verify_ground_truth.verify_item(item, lines)
    assert "TYPE MISMATCH" in issue


def test_verify_item_import_match():
    """Test matching imports."""
    item = {"type": "import", "name": "json", "line_start": 1}

    # Direct import
    lines = ["import json"]
    assert verify_ground_truth.verify_item(item, lines) is None

    # From import
    item_from = {"type": "import", "name": "pathlib.Path", "line_start": 1}
    lines = ["from pathlib import Path"]
    assert verify_ground_truth.verify_item(item_from, lines) is None

    # Let's test specific failure
    item_fail = {"type": "import", "name": "missing_module", "line_start": 1}
    lines = ["import os"]
    assert verify_ground_truth.verify_item(item_fail, lines) is None

    # Test REAL failure where neither name nor "import" keyword is present
    lines_fail = ["x = 1"]
    issue = verify_ground_truth.verify_item(item_fail, lines_fail)
    assert "TYPE MISMATCH" in issue


def test_verify_item_variable_match():
    """Test matching variables."""
    item = {"type": "variable", "name": "my_var", "line_start": 1}

    # Assignment
    lines = ["my_var = 10"]
    assert verify_ground_truth.verify_item(item, lines) is None

    # Mismatch
    lines = ["other_var = 10"]
    issue = verify_ground_truth.verify_item(item, lines)
    assert "NAME NOT FOUND" in issue


def test_find_ground_truth_files(tmp_path):
    """Test walking directory for GT files."""
    (tmp_path / "subdir").mkdir()
    (tmp_path / "subdir" / "ground_truth.json").write_text("{}")
    (tmp_path / "other.json").write_text("{}")

    files = verify_ground_truth.find_ground_truth_files(tmp_path)
    assert str(files[0]).endswith("ground_truth.json")


def test_verify_ground_truth_file(tmp_path):
    """Test full file verification logic."""
    (tmp_path / "test.py").write_text("def foo():\n    pass")

    gt_file = tmp_path / "ground_truth.json"
    data = {
        "files": {
            "test.py": {
                "dead_items": [{"type": "function", "name": "foo", "line_start": 1}]
            }
        }
    }
    gt_file.write_text(json.dumps(data))

    # Run verification - should be empty list (no issues)
    issues = verify_ground_truth.verify_ground_truth(gt_file)
    assert issues == []

    # Test with missing file
    data["files"]["missing.py"] = {}
    gt_file.write_text(json.dumps(data))
    issues = verify_ground_truth.verify_ground_truth(gt_file)
    assert len(issues) == 1
    assert "FILE MISSING" in issues[0]

    # Test with verification failure
    (tmp_path / "fail.py").write_text("def bar(): pass")
    data["files"] = {
        "fail.py": {
            "dead_items": [{"type": "function", "name": "baz", "line_start": 1}]
        }
    }
    gt_file.write_text(json.dumps(data))
    issues = verify_ground_truth.verify_ground_truth(gt_file)
    assert len(issues) == 1
    assert "TYPE MISMATCH" in issues[0]


def test_main(capsys):
    """Test main execution logic via mocks."""
    with patch("verify_ground_truth.find_ground_truth_files", return_value=["A", "B"]):
        with patch("verify_ground_truth.verify_ground_truth") as mock_verify:
            # First file has issues, second has none
            mock_verify.side_effect = [["Issue 1"], []]

            verify_ground_truth.main()

            captured = capsys.readouterr()
            assert "Issues in A" in captured.out
            assert "Issue 1" in captured.out
            assert "Total Issues Found: 1" in captured.out
