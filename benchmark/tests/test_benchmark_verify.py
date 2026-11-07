import subprocess
import sys
from pathlib import Path
from unittest.mock import MagicMock, patch

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
