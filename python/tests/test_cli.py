import pytest
import sys
from unittest.mock import patch
from cytoscnpy.cli import main


@pytest.fixture
def mock_run():
    """Mock the cytoscnpy.cli.run function."""
    with patch("cytoscnpy.cli.run") as mock:
        yield mock


@pytest.mark.parametrize(
    "args, expected_code",
    [
        (["cytoscnpy", "--help"], 0),
        (["cytoscnpy", "--version"], 0),
        (["cytoscnpy"], 0),  # Assuming 0, but checking mock interaction
    ],
)
def test_main_basic_args(mock_run, args, expected_code):
    """Test basic arguments that shouldn't trigger full analysis logic errors in the wrapper."""
    mock_run.return_value = 0
    with patch.object(sys, "argv", args):
        with pytest.raises(SystemExit) as e:
            main()
        assert e.value.code == expected_code


@pytest.mark.parametrize(
    "args, passed_to_rust",
    [
        (["cytoscnpy", "."], ["."]),
        (["cytoscnpy", ".", "--verbose"], [".", "--verbose"]),
        (["cytoscnpy", "--json", "src/"], ["--json", "src/"]),
        (
            ["cytoscnpy", "--exclude", "venv", "--ignore", "test_*"],
            ["--exclude", "venv", "--ignore", "test_*"],
        ),
    ],
)
def test_args_passed_to_rust_backend(mock_run, args, passed_to_rust):
    """Verify that arguments are correctly passed through to the Rust backend."""
    mock_run.return_value = 0
    with patch.object(sys, "argv", args):
        with pytest.raises(SystemExit) as e:
            main()

        assert e.value.code == 0
        mock_run.assert_called_with(passed_to_rust)


def test_rust_backend_failure_propagates(mock_run):
    """Test that if Rust backend returns 1, Python exits with 1."""
    mock_run.return_value = 1
    with patch.object(sys, "argv", ["cytoscnpy"]):
        with pytest.raises(SystemExit) as e:
            main()
        assert e.value.code == 1


def test_keyboard_interrupt_handled(mock_run):
    """Test that KeyboardInterrupt is handled gracefully (exit 130)."""
    mock_run.side_effect = KeyboardInterrupt
    with patch.object(sys, "argv", ["cytoscnpy"]):
        # We expect the wrapper might not catch this specifically, or it might let it propagate.
        # If the wrapper generally catches Exception, KeyboardInterrupt (BaseException) might pass through.
        # Let's see current implementation behavior or adjust test expectation.
        # The current implementation catches `KeyboardInterrupt` and raises SystemExit(130).
        with pytest.raises(SystemExit) as e:
            main()
        assert e.value.code == 130
