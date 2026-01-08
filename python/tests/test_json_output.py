import pytest
import subprocess
import sys
import json


def run_json_analysis(cwd):
    """Run JSON analysis integration test."""
    cmd = [sys.executable, "-m", "cytoscnpy", ".", "--json"]
    return subprocess.run(
        cmd, cwd=cwd, capture_output=True, text=True, encoding="utf-8", errors="replace"
    )


def test_json_schema_validity(tmp_path):
    """Ensure the JSON output adheres to the expected schema."""
    # Setup: Create a file with known issues
    (tmp_path / "code.py").write_text(
        """
import os  # Unused import
def unused_func(): pass
x = 1 # Unused var
""",
        encoding="utf-8",
    )

    result = run_json_analysis(tmp_path)
    assert result.returncode == 0

    try:
        data = json.loads(result.stdout)
    except json.JSONDecodeError:
        pytest.fail("Failed to parse JSON output")

    print("DEBUG JSON content:", json.dumps(data, indent=2))
    print(
        "DEBUG Found functions:",
        [f.get("name") for f in data.get("unused_functions", [])],
    )

    # Schema Validation
    required_keys = [
        "unused_functions",
        "unused_imports",
        "unused_classes",
        "unused_variables",
        "unused_parameters",
        "analysis_summary",
    ]
    for key in required_keys:
        assert key in data, f"Missing key: {key}"

    # Check Summary
    summary = data["analysis_summary"]
    assert "total_files" in summary
    assert "total_lines_analyzed" in summary
    assert summary["total_files"] >= 1

    # Check Findings
    # The 'name' field includes the module prefix (e.g. 'code.unused_func')
    # Use simple_name if available, or check for substring
    assert any(f["name"].endswith("unused_func") for f in data["unused_functions"])
    assert any(i["name"] == "os" for i in data["unused_imports"])


def test_json_empty_project(tmp_path):
    """Ensure JSON is valid even for empty results."""
    # No python files
    result = run_json_analysis(tmp_path)
    data = json.loads(result.stdout)

    assert data["analysis_summary"]["total_files"] == 0
    assert data["unused_functions"] == []
