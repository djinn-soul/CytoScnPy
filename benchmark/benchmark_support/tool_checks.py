from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

from .models import ToolCheckInfo, ToolStatus


def get_tool_path(tool_name: str) -> str | None:
    """Locate tool executable in PATH or specific locations."""
    # Check PATH first
    path = shutil.which(tool_name)
    if path:
        return path

    # Check current environment scripts
    scripts_dir = (
        Path(sys.prefix) / "Scripts"
        if sys.platform == "win32"
        else Path(sys.prefix) / "bin"
    )
    possible_path = scripts_dir / (
        tool_name + ".exe" if sys.platform == "win32" else tool_name
    )

    if possible_path.exists():
        return str(possible_path)

    return None


# Data-driven tool check configuration
# Maps tool names to their check command info
TOOL_CHECKS: dict[str, ToolCheckInfo] = {
    "Vulture (0%)": {"module": "vulture", "arg": "--version"},
    "Vulture (60%)": {"module": "vulture", "arg": "--version"},
    "Flake8": {"module": "flake8", "arg": "--version"},
    "Pylint": {"module": "pylint", "arg": "--version"},
    "Ruff": {"module": "ruff", "arg": "--version"},
    "uncalled": {"module": "uncalled", "arg": "--help"},
    "dead": {"module": "dead", "arg": "--help"},
}


def _check_python_module(module: str, arg: str) -> ToolStatus:
    """Check if a Python module is installed and callable."""
    try:
        result = subprocess.run(
            [sys.executable, "-m", module, arg],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            return {"available": True, "reason": "Installed"}
        return {
            "available": False,
            "reason": f"Not installed (pip install {module})",
        }
    except Exception as e:  # noqa: BLE001
        print(f"[-] Check failed for {module}: {e}")
        return {"available": False, "reason": f"Not installed (pip install {module})"}


def _check_cytoscnpy_rust(command: list[str]) -> ToolStatus:
    status: ToolStatus = {"available": False, "reason": "Unknown"}

    if command[0] == "cargo":
        if shutil.which("cargo"):
            status = {"available": True, "reason": "Cargo found"}
        else:
            status["reason"] = "Cargo not found in PATH"
    else:
        bin_path = Path(str(command[0]))
        if bin_path.exists() or shutil.which(command[0]):
            status = {"available": True, "reason": "Binary found"}
        else:
            status["reason"] = f"Binary not found: {command[0]}"

    return status


def _check_cytoscnpy_python() -> ToolStatus:
    status: ToolStatus = {"available": False, "reason": "Unknown"}
    try:
        result = subprocess.run(
            [sys.executable, "-c", "import cytoscnpy"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            status = {"available": True, "reason": "Module importable"}
        else:
            status["reason"] = "Module not installed (pip install -e .)"
    except Exception as e:  # noqa: BLE001
        status["reason"] = f"Check failed: {e}"
    return status
