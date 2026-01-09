import sys
import subprocess
import pytest


# Helper to run cytoscnpy as a subprocess
def run_cytoscnpy(args, cwd=None):
    """Run a cytoscnpy command integration test."""
    cmd = [sys.executable, "-m", "cytoscnpy"] + args
    return subprocess.run(
        cmd,
        cwd=cwd,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )


def test_integration_help():
    """Verify help command runs and contains usage info."""
    result = run_cytoscnpy(["--help"])
    assert result.returncode == 0
    assert "Usage:" in result.stdout or "Options:" in result.stdout


def test_integration_no_files_found(tmp_path):
    """Run on an empty directory."""
    result = run_cytoscnpy(["."], cwd=tmp_path)
    # Depending on implementation, might be 0 (success) or non-zero (if no files found is an error)
    # Rust version usually prints "No folders excluded" or similar.
    assert result.returncode == 0
    # Should probably mention something about files or lines


def test_integration_finds_unused_function(tmp_path):
    """Create a python file with unused function and ensure it's detected."""
    p = tmp_path / "test.py"
    p.write_text(
        "def unused():\n    pass\n\ndef used():\n    pass\n\nused()\n", encoding="utf-8"
    )

    # Run analysis
    result = run_cytoscnpy(["."], cwd=tmp_path)

    assert (
        result.returncode == 0
    ), f"Command failed with {result.returncode}\nStdout: {result.stdout}\nStderr: {result.stderr}"

    # Debug print
    print(f"STDOUT:\n{result.stdout}")

    assert "unused" in result.stdout or "unused" in result.stderr


def test_integration_json_output(tmp_path):
    """Verify --json flag produces valid JSON."""
    import json

    p = tmp_path / "test.py"
    p.write_text("def foo(): pass\n", encoding="utf-8")

    result = run_cytoscnpy([".", "--json"], cwd=tmp_path)
    assert result.returncode == 0

    try:
        data = json.loads(result.stdout)
        assert "unused_functions" in data
        assert isinstance(data["unused_functions"], list)

        found_names = [f["name"] for f in data.get("unused_functions", [])]
        print(f"Found functions: {found_names}")

        # Check if any found name ends with .foo or is foo
        assert any(name.endswith("foo") for name in found_names)

    except json.JSONDecodeError:
        pytest.fail(f"Output was not valid JSON:\n{result.stdout}")
