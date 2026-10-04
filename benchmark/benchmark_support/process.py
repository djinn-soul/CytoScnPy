from __future__ import annotations

import shlex
import subprocess
import threading
import time
from collections.abc import Mapping
from typing import cast

from .models import PsutilModule, PsutilProcessLike

try:
    import psutil as _psutil
except ImportError:
    psutil: PsutilModule | None = None
else:
    psutil = cast(PsutilModule, _psutil)


class _PsutilMissingError(Exception):
    """Placeholder exception type used when psutil is unavailable."""


if psutil is not None:
    _psutil_exception_types: tuple[type[BaseException], ...] = (
        psutil.NoSuchProcess,
        psutil.AccessDenied,
    )
else:
    _psutil_exception_types = (_PsutilMissingError,)


def _safe_child_rss(child: PsutilProcessLike) -> int:
    """Return the child's RSS or 0 if it cannot be accessed."""
    try:
        return child.memory_info().rss
    except _psutil_exception_types:
        return 0


def _update_max_rss(p: PsutilProcessLike, max_rss: list[int]) -> int:
    """Update max RSS with current process and children memory usage."""
    rss = p.memory_info().rss
    if rss > max_rss[0]:
        max_rss[0] = rss
    # Check children too
    for child in p.children(recursive=True):
        child_rss = _safe_child_rss(child)
        rss += child_rss

    if rss > max_rss[0]:
        max_rss[0] = rss
    return rss


def _monitor_memory(
    process: subprocess.Popen[str],
    max_rss: list[int],
    stop_monitoring: threading.Event,
) -> None:
    """Monitor memory usage of a process and its children."""
    if psutil is None:
        return

    try:
        p = psutil.Process(process.pid)
    except _psutil_exception_types:
        return

    while not stop_monitoring.is_set():
        if process.poll() is not None:
            break

        try:
            _update_max_rss(p, max_rss)
        except _psutil_exception_types:
            break
        time.sleep(0.01)  # Poll interval


def _handle_timeout(
    command: list[str],
    process: subprocess.Popen[str],
    max_rss: list[int],
    timeout: int,
) -> tuple[subprocess.CompletedProcess[str], int, float]:
    """Handle process timeout."""
    process.kill()
    stdout, stderr = process.communicate()
    return (
        subprocess.CompletedProcess(command, -1, stdout, stderr + "\nTimeout"),
        timeout,
        max_rss[0] / (1024 * 1024),
    )


def run_command(
    command: str | list[str],
    cwd: str | None = None,
    env: Mapping[str, str] | None = None,
    timeout: int = 300,
) -> tuple[subprocess.CompletedProcess[str], float, float]:
    """Runs a command and returns (result, duration, max_rss_mb)."""
    start_time = time.time()
    use_shell = False

    if isinstance(command, str):
        # Securely split the string command
        command_list = shlex.split(command)
    else:
        command_list = command

    try:
        # We need to use Popen to track memory usage with psutil
        process = subprocess.Popen(
            command_list,
            cwd=cwd,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            stdin=subprocess.DEVNULL,  # Prevent interactive prompts
            text=True,
            shell=use_shell,  # nosec B602
        )
    except FileNotFoundError:
        return (
            subprocess.CompletedProcess(command, 2, "", f"File not found: {command}"),
            0,
            0,
        )

    max_rss = [0]  # Use list for mutable closure
    stop_monitoring = threading.Event()

    # Start memory monitoring thread
    monitor_thread = threading.Thread(
        target=_monitor_memory, args=(process, max_rss, stop_monitoring)
    )
    monitor_thread.start()

    try:
        stdout, stderr = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        stop_monitoring.set()
        monitor_thread.join()
        return _handle_timeout(command_list, process, max_rss, timeout)

    stop_monitoring.set()
    monitor_thread.join()

    end_time = time.time()
    duration = end_time - start_time

    # Create a result object similar to subprocess.run
    result = subprocess.CompletedProcess(
        command_list, process.returncode, stdout, stderr
    )

    return result, duration, max_rss[0] / (1024 * 1024)
