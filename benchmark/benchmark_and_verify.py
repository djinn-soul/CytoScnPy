"""Benchmark and verify tools while preserving the established script API."""

from __future__ import annotations

import shutil
import subprocess
import sys
from collections.abc import Mapping
from pathlib import Path

if __package__:
    from .benchmark_support import api
else:
    from benchmark_support import api

run_main = api.run_main
_count_cytoscnpy_issues = api.count_cytoscnpy_issues
_count_dead_issues = api.count_dead_issues
_count_deadcode_issues = api.count_deadcode_issues
_count_issues = api.count_issues
_count_pylint_issues = api.count_pylint_issues
_count_ruff_issues = api.count_ruff_issues
_count_skylos_issues = api.count_skylos_issues
BenchmarkResult = api.BenchmarkResult
FinalReport = api.FinalReport
FinalReportEntry = api.FinalReportEntry
Finding = api.Finding
MetricResult = api.MetricResult
MetricResults = api.MetricResults
ToolCheckInfo = api.ToolCheckInfo
ToolCheckResults = api.ToolCheckResults
ToolConfig = api.ToolConfig
ToolConfigRequired = api.ToolConfigRequired
ToolStatus = api.ToolStatus
VerificationResult = api.VerificationResult
VerificationValue = api.VerificationValue
_handle_timeout = api.handle_timeout
_monitor_memory = api.monitor_memory
_psutil_exception_types = api.psutil_exception_types
_PsutilMissingError = api.PsutilMissingError
_safe_child_rss = api.safe_child_rss
_update_max_rss = api.update_max_rss
psutil = api.psutil
run_command = api.run_command
TOOL_CHECKS = api.TOOL_CHECKS
_check_cytoscnpy_python = api.check_cytoscnpy_python
_check_cytoscnpy_rust = api.check_cytoscnpy_rust
_check_python_module = api.check_python_module
get_tool_path = api.get_tool_path
_as_float = api.as_float
_as_int = api.as_int
_as_str = api.as_str
normalize_path = api.normalize_path
Verification = api.Verification
_Args = api.Args
_MemoryInfo = api.MemoryInfo
_PsutilModule = api.PsutilModule
_PsutilProcessLike = api.PsutilProcessLike


def _check_deadcode(command: str | list[str]) -> ToolStatus:
    status: ToolStatus = {"available": False, "reason": "Unknown"}
    if isinstance(command, list) and command:
        exe_path = Path(command[0])
        if exe_path.exists() or shutil.which(command[0]):
            status = {"available": True, "reason": "Executable found"}
        else:
            status["reason"] = f"Executable not found: {command[0]}"
    else:
        deadcode_path = get_tool_path("deadcode")
        if deadcode_path:
            status = {"available": True, "reason": f"Found at {deadcode_path}"}
        else:
            status["reason"] = "Not installed (pip install deadcode)"
    return status


def _check_skylos() -> ToolStatus:
    status: ToolStatus = {"available": False, "reason": "Unknown"}
    skylos_path = get_tool_path("skylos")
    if skylos_path:
        status = {"available": True, "reason": f"Found at {skylos_path}"}
    else:
        try:
            result = subprocess.run(
                [sys.executable, "-m", "skylos", "--help"],
                capture_output=True,
                text=True,
                timeout=10,
            )
            if result.returncode == 0:
                status = {"available": True, "reason": "Available as module"}
            else:
                status["reason"] = "Not installed (pip install skylos)"
        except Exception as e:  # noqa: BLE001
            print(f"[-] Check failed for skylos: {e}")
            status["reason"] = "Not installed (pip install skylos)"
    return status


def check_tool_availability(tools_config: list[ToolConfig]) -> ToolCheckResults:
    """Pre-check all tools to verify they are installed and available.

    Returns a dict with tool status: {name: {"available": bool, "reason": str}}
    """
    print("\n[+] Checking tool availability...")
    results: ToolCheckResults = {}

    for tool in tools_config:
        name = tool["name"]
        command = tool["command"]

        status: ToolStatus = {"available": False, "reason": "Unknown"}

        if not command:
            status["reason"] = "No command configured"
            results[name] = status
            continue

        if name in TOOL_CHECKS:
            check_info = TOOL_CHECKS[name]
            status = _check_python_module(check_info["module"], check_info["arg"])
        elif name == "CytoScnPy (Rust)":
            status = _check_cytoscnpy_rust(command)
        elif name == "CytoScnPy (Python)":
            status = _check_cytoscnpy_python()
        elif name == "deadcode":
            status = _check_deadcode(command)
        elif name == "Skylos":
            status = _check_skylos()
        else:
            status = {"available": True, "reason": "Command configured"}

        results[name] = status

    available_count = sum(1 for s in results.values() if s["available"])
    print(f"\n    Tool Availability: {available_count}/{len(results)} tools ready")
    print("-" * 60)
    for name, status in results.items():
        icon = "[OK]" if status["available"] else "[X] "
        print(f"    {icon} {name}: {status['reason']}")
    print("-" * 60)

    return results


def run_benchmark_tool(
    name: str,
    command: str | list[str],
    cwd: str | None = None,
    env: Mapping[str, str] | None = None,
) -> BenchmarkResult | None:
    """Run a specific benchmark tool command."""
    print(f"\n[+] Running {name}...")
    print(f"    Command: {command}")
    if not command:
        print(f"[-] {name} command not found/configured.")
        return None

    result, duration, max_rss = run_command(command, cwd, env)
    print(f"    [OK] Completed in {duration:.2f}s (Memory: {max_rss:.1f} MB)")

    output = result.stdout + result.stderr

    issue_count = _count_issues(name, result.stdout, result.stderr, output)

    return {
        "name": name,
        "time": duration,
        "memory_mb": max_rss,
        "issues": issue_count,
        "output": output,
        "stdout": result.stdout,  # Keep separate for JSON parsing
    }


def main() -> None:
    """Run the CLI with the script's public dependency hooks."""
    run_main(
        Path(__file__).parent.resolve(),
        check_tool_availability,
        run_benchmark_tool,
        Verification,
        psutil_available=psutil is not None,
    )


if __name__ == "__main__":
    main()
