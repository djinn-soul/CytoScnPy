from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path

from .models import ToolConfig


def configure_tools(
    project_root: Path, target_dir: Path
) -> tuple[list[ToolConfig], Path]:
    """Prepare the same tool commands, environment, and Rust binary path."""
    # Setup Python Environment
    env = os.environ.copy()
    python_path_entries: list[str] = []

    # 1. CytoScnPy Python Wrapper
    python_src = project_root / "python"
    if python_src.exists():
        python_path_entries.append(str(python_src))

        # Try to copy the built extension to the python package for it to work
        # Look for cytoscnpy.dll / .so in target/release
        ext_src = project_root / "target" / "release" / "cytoscnpy.dll"
        if not ext_src.exists():
            ext_src = (
                project_root / "cytoscnpy" / "target" / "release" / "cytoscnpy.dll"
            )

        if ext_src.exists():
            ext_dest = python_src / "cytoscnpy" / "cytoscnpy.pyd"
            try:
                shutil.copy2(ext_src, ext_dest)
            except OSError:
                # Preserve the existing best effort copy for the optional wrapper.
                pass

    # 2. Skylos
    skylos_src = project_root / "other_library" / "skylos"
    if skylos_src.exists():
        python_path_entries.append(str(skylos_src))

    if python_path_entries:
        env["PYTHONPATH"] = (
            os.pathsep.join(python_path_entries)
            + os.pathsep
            + env.get("PYTHONPATH", "")
        )

    # Rust Binary Path
    # Try project_root/target/release first (workspace root)
    rust_bin = project_root / "target" / "release" / "cytoscnpy-bin"
    if not rust_bin.exists() and not rust_bin.with_suffix(".exe").exists():
        # Fallback to cytoscnpy/target/release
        rust_bin = project_root / "cytoscnpy" / "target" / "release" / "cytoscnpy-bin"

    # Second fallback: maybe it was built as 'cytoscnpy'
    if not rust_bin.exists() and not rust_bin.with_suffix(".exe").exists():
        rust_bin = project_root / "target" / "release" / "cytoscnpy"

    if sys.platform == "win32":
        rust_bin = rust_bin.with_suffix(".exe")

    # Convert paths to strings for commands
    target_dir_str = str(target_dir)
    rust_bin_str = str(rust_bin)
    interpreter_dir = Path(sys.executable).parent
    skylos_bin = interpreter_dir / ("skylos.exe" if os.name == "nt" else "skylos")
    deadcode_bin = interpreter_dir / ("deadcode.exe" if os.name == "nt" else "deadcode")

    all_tools: list[ToolConfig] = [
        {
            "name": "CytoScnPy (Rust)",
            "command": [rust_bin_str, target_dir_str, "--json"],
        },
        {
            "name": "CytoScnPy (Python)",
            "command": [
                sys.executable,
                "-m",
                "cytoscnpy.cli",
                target_dir_str,
                "--json",
            ],
            "env": env,
        },
        {
            "name": "Skylos",
            # Use skylos executable from venv with full path
            "command": [
                str(skylos_bin),
                target_dir_str,
                "--json",
                "--confidence",
                "0",
            ],
            "env": env,
        },
        {
            "name": "Vulture (0%)",
            "command": [
                sys.executable,
                "-m",
                "vulture",
                target_dir_str,
                "--min-confidence",
                "0",
            ],
        },
        {
            "name": "Vulture (60%)",
            "command": [
                sys.executable,
                "-m",
                "vulture",
                target_dir_str,
                "--min-confidence",
                "60",
            ],
        },
        {"name": "Flake8", "command": [sys.executable, "-m", "flake8", target_dir_str]},
        {
            "name": "Pylint",
            "command": [
                sys.executable,
                "-m",
                "pylint",
                target_dir_str,
                "--output-format=json",
                "-j",
                "4",
            ],
        },
        {
            "name": "Ruff",
            "command": [
                sys.executable,
                "-m",
                "ruff",
                "check",
                target_dir_str,
                "--output-format=json",
            ],
        },
        {
            "name": "uncalled",
            "command": [sys.executable, "-m", "uncalled", target_dir_str],
        },
        {
            "name": "dead",
            # dead uses --files regex, not positional path. It runs from CWD.
            "command": [sys.executable, "-m", "dead", "--files", ".*\\.py$"],
            "cwd": target_dir_str,
        },
        {
            "name": "deadcode",
            # deadcode doesn't support 'python -m deadcode', use executable directly
            # Use --no-color to avoid ANSI codes breaking parsing
            "command": [
                str(deadcode_bin),
                target_dir_str,
                "--no-color",
            ],
        },
    ]

    return all_tools, rust_bin
