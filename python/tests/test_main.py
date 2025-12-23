import subprocess
import sys
from cytoscnpy.cli import main
import pytest
import cytoscnpy

def test_main_execution():
    """Test that python -m cytoscnpy runs correctly."""
    result = subprocess.run(
        [sys.executable, "-m", "cytoscnpy", "--version"],
        capture_output=True,
        text=True
    )
    assert result.returncode == 0
    assert "cytoscnpy" in result.stdout.lower()

def test_main_function_direct(monkeypatch):
    """Test calling the main function directly to ensure it's covered."""
    monkeypatch.setattr(sys, "argv", ["cytoscnpy", "--version"])
    with pytest.raises(SystemExit) as excinfo:
        main()
    assert excinfo.value.code == 0

def test_main_module():
    """Test the __main__.py module entry point."""
    import cytoscnpy.__main__
    assert cytoscnpy.__main__.main is main
