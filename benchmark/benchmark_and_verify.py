import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import threading
import time
from collections.abc import Iterable, Mapping
from pathlib import Path
from typing import (
    Callable,
    Optional,
    Protocol,
    TypedDict,
    Union,
    cast,
)

Finding = tuple[str, Optional[int], str, str]


class ToolStatus(TypedDict):
    available: bool
    reason: str


class ToolConfigRequired(TypedDict):
    name: str
    command: list[str]


class ToolConfig(ToolConfigRequired, total=False):
    env: Mapping[str, str]
    cwd: str


ToolCheckResults = dict[str, ToolStatus]


class ToolCheckInfo(TypedDict):
    module: str
    arg: str


class BenchmarkResult(TypedDict):
    name: str
    time: float
    memory_mb: float
    issues: int
    output: str
    stdout: str


class MetricResult(TypedDict):
    TP: int
    FP: int
    FN: int
    Precision: float
    Recall: float
    F1: float
    missed_items: list[str]


VerificationValue = Union[MetricResult, str]
VerificationResult = dict[str, VerificationValue]
MetricResults = dict[str, MetricResult]


class FinalReportEntry(TypedDict):
    name: str
    time: float
    memory_mb: float
    issues: int
    f1_score: float
    precision: float
    recall: float
    stats: Mapping[str, object]


class FinalReport(TypedDict):
    timestamp: float
    platform: str
    results: list[FinalReportEntry]


class _Args(Protocol):
    list: bool
    check: bool
    include: Optional[list[str]]
    exclude: Optional[list[str]]
    save_json: Optional[str]
    compare_json: Optional[str]
    threshold: float


class _MemoryInfo(Protocol):
    rss: int


class _PsutilProcessLike(Protocol):
    def memory_info(self) -> _MemoryInfo: ...

    def children(self, *, recursive: bool = ...) -> Iterable["_PsutilProcessLike"]: ...


class _PsutilModule(Protocol):
    NoSuchProcess: type[BaseException]
    AccessDenied: type[BaseException]

    # psutil.Process is a callable/class; model it as an attribute to avoid
    # style warnings about method naming while keeping the external API shape.
    Process: Callable[[int], _PsutilProcessLike]


try:
    import psutil as _psutil
except ImportError:
    psutil: Optional[_PsutilModule] = None
else:
    psutil = cast(_PsutilModule, _psutil)


class _PsutilMissingError(Exception):
    """Placeholder exception type used when psutil is unavailable."""


if psutil is not None:
    _psutil_exception_types: tuple[type[BaseException], ...] = (
        psutil.NoSuchProcess,
        psutil.AccessDenied,
    )
else:
    _psutil_exception_types = (_PsutilMissingError,)


def _safe_child_rss(child: _PsutilProcessLike) -> int:
    """Return the child's RSS or 0 if it cannot be accessed."""
    try:
        return child.memory_info().rss
    except _psutil_exception_types:
        return 0


def _update_max_rss(p: _PsutilProcessLike, max_rss: list[int]) -> int:
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
    command: Union[str, list[str]],
    cwd: Optional[str] = None,
    env: Optional[Mapping[str, str]] = None,
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


def normalize_path(p: str) -> str:
    """Normalize path separator to forward slashes."""
    return str(Path(p).as_posix()).strip("/")


def _as_str(value: object) -> str:
    return value if isinstance(value, str) else ""


def _as_int(value: object) -> Optional[int]:
    if isinstance(value, bool):
        return None
    return value if isinstance(value, int) else None


def _as_float(value: object) -> Optional[float]:
    if isinstance(value, bool):
        return None
    if isinstance(value, (int, float)):
        return float(value)
    return None


def get_tool_path(tool_name: str) -> Optional[str]:
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
    except Exception:  # noqa: BLE001
        return {"available": False, "reason": f"Not installed (pip install {module})"}


def _check_cytoscnpy_rust(command: list[str]) -> ToolStatus:
    status: ToolStatus = {"available": False, "reason": "Unknown"}
    if isinstance(command, list):  # pyright: ignore[reportUnnecessaryIsInstance]
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
    else:
        match = re.search(r'"([^"]+)"', command)
        bin_path = Path(match.group(1)) if match else Path(command)
        if bin_path.exists() or shutil.which(str(command)):
            status = {"available": True, "reason": "Binary found"}
        else:
            status["reason"] = f"Binary not found: {bin_path if bin_path else command}"
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


def _check_deadcode(command: Union[str, list[str]]) -> ToolStatus:
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
        except Exception:  # noqa: BLE001
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
    command: Union[str, list[str]],
    cwd: Optional[str] = None,
    env: Optional[Mapping[str, str]] = None,
) -> Optional[BenchmarkResult]:
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


def _count_cytoscnpy_issues(stdout: str) -> int:
    """Count issues from CytoScnPy output."""
    try:
        data = cast(object, json.loads(stdout))
        if not isinstance(data, dict):
            return 0
        data_dict = cast(dict[str, object], data)
        categories = [
            "unused_functions",
            "unused_methods",
            "unused_imports",
            "unused_classes",
            "unused_variables",
            "unused_parameters",
        ]
        total = 0
        for key in categories:
            items = data_dict.get(key)
            if isinstance(items, list):
                items_list = cast(list[object], items)
                total += len(items_list)
        return total
    except (json.JSONDecodeError, KeyError, TypeError):
        return 0


def _count_ruff_issues(stdout: str, output: str) -> int:
    """Count issues from Ruff output."""
    try:
        data = cast(object, json.loads(stdout))
        if isinstance(data, list):
            data_list = cast(list[object], data)
            return len(data_list)
        if isinstance(data, dict):
            data_dict = cast(dict[str, object], data)
            issues = data_dict.get("issues")
            if isinstance(issues, list):
                issues_list = cast(list[object], issues)
                return len(issues_list)
    except (json.JSONDecodeError, KeyError, TypeError):
        pass
    return len(output.strip().splitlines())


def _count_pylint_issues(stdout: str, output: str) -> int:
    """Count issues from Pylint output."""
    try:
        data = cast(object, json.loads(stdout))
        if isinstance(data, list):
            data_list = cast(list[object], data)
            return len(data_list)
    except (json.JSONDecodeError, KeyError, TypeError):
        pass
    return len([line for line in output.splitlines() if ": " in line])


def _count_dead_issues(output: str) -> int:
    """Count issues from dead output."""
    return len(
        [
            line
            for line in output.splitlines()
            if "is never" in line.lower() or "never read" in line.lower()
        ]
    )


def _count_deadcode_issues(output: str) -> int:
    """Count issues from deadcode output."""
    return len([line for line in output.splitlines() if re.search(r": DC\d+", line)])


def _count_skylos_issues(stdout: str) -> int:
    """Count issues from Skylos output."""
    try:
        data = cast(object, json.loads(stdout))
        if not isinstance(data, dict):
            return 0
        data_dict = cast(dict[str, object], data)
        total = 0
        for key in [
            "unused_functions",
            "unused_imports",
            "unused_classes",
            "unused_variables",
        ]:
            items = data_dict.get(key)
            if isinstance(items, list):
                items_list = cast(list[object], items)
                total += len(items_list)
        return total
    except (json.JSONDecodeError, KeyError, TypeError):
        return 0


def _count_issues(name: str, stdout: str, _stderr: str, output: str) -> int:
    """Helper to count issues for each tool."""
    if name in ["CytoScnPy (Rust)", "CytoScnPy (Python)"]:
        return _count_cytoscnpy_issues(stdout)
    if name == "Ruff":
        return _count_ruff_issues(stdout, output)
    if name == "Flake8":
        return len(output.strip().splitlines())
    if name == "Pylint":
        return _count_pylint_issues(stdout, output)
    if "Vulture" in name:
        return len(output.strip().splitlines())
    if name == "uncalled":
        return len([line for line in output.splitlines() if "unused" in line.lower()])
    if name == "dead":
        return _count_dead_issues(output)
    if name == "deadcode":
        return _count_deadcode_issues(output)
    if name == "Skylos":
        return _count_skylos_issues(stdout)
    return 0


class Verification:
    """Handles verification of tool output against ground truth."""

    def __init__(self, ground_truth_path: Union[str, Path]) -> None:
        """Initialize verification with ground truth data."""
        super().__init__()
        self.covered_files: set[str] = set()
        self.ground_truth: set[Finding] = self.load_ground_truth(ground_truth_path)

    @staticmethod
    def _as_dict_list(value: object) -> list[dict[str, object]]:
        if not isinstance(value, list):
            return []
        raw_list = cast(list[object], value)
        return [
            cast(dict[str, object], item) for item in raw_list if isinstance(item, dict)
        ]

    @staticmethod
    def _ground_truth_files(path_obj: Path) -> list[Path]:
        if path_obj.is_dir():
            return list(path_obj.rglob("ground_truth.json"))
        if path_obj.exists():
            return [path_obj]
        return []

    @staticmethod
    def _iter_ground_truth_files_section(
        data_dict: dict[str, object],
    ) -> Iterable[tuple[str, dict[str, object]]]:
        files_section = data_dict.get("files")
        if not isinstance(files_section, dict):
            return []
        files_section_dict = cast(dict[str, object], files_section)
        return [
            (file_path, cast(dict[str, object], content))
            for file_path, content in files_section_dict.items()
            if isinstance(content, dict)
        ]

    def _load_ground_truth_file(self, p: Path) -> set[Finding]:
        truth_set: set[Finding] = set()
        try:
            with p.open() as f:
                data = cast(object, json.load(f))
        except (OSError, json.JSONDecodeError) as e:
            print(f"[-] Error loading ground truth from {p}: {e}")
            return truth_set

        if not isinstance(data, dict):
            return truth_set
        data_dict = cast(dict[str, object], data)

        for file_path, content_dict in self._iter_ground_truth_files_section(data_dict):
            base_dir = p.parent
            full_path = (base_dir / file_path).resolve()
            t_file_str = normalize_path(str(full_path))
            self.covered_files.add(t_file_str)

            dead_items_list = self._as_dict_list(content_dict.get("dead_items"))
            for item_dict in dead_items_list:
                if item_dict.get("suppressed"):
                    continue
                truth_set.add(
                    (
                        t_file_str,
                        _as_int(item_dict.get("line_start")),
                        _as_str(item_dict.get("type")),
                        _as_str(item_dict.get("name")),
                    )
                )

        return truth_set

    def load_ground_truth(self, path: Union[str, Path]) -> set[Finding]:
        """Load ground truth assertions from file."""
        path_obj = Path(path)
        truth_set: set[Finding] = set()
        for p in self._ground_truth_files(path_obj):
            truth_set |= self._load_ground_truth_file(p)
        return truth_set

    @staticmethod
    def parse_tool_output(name: str, output: str) -> set[Finding]:
        """Parse raw output from a tool into structured findings."""
        parser_map: dict[str, Callable[[str], set[Finding]]] = {
            "CytoScnPy (Rust)": Verification._parse_cytoscnpy_output,
            "CytoScnPy (Python)": Verification._parse_cytoscnpy_output,
            "Skylos": Verification._parse_skylos_output,
            "Flake8": Verification._parse_flake8_output,
            "Pylint": Verification._parse_pylint_output,
            "Ruff": Verification._parse_ruff_output,
            "dead": Verification._parse_dead_output,
            "uncalled": Verification._parse_uncalled_output,
            "deadcode": Verification._parse_deadcode_output,
        }
        parser = parser_map.get(name)
        if parser is None and "Vulture" in name:
            parser = Verification._parse_vulture_output

        if parser is None:
            findings: set[Finding] = set()
        else:
            findings = parser(output)

        if not findings and output.strip():
            print(f"DEBUG: {name} produced output but no findings parsed:")
            print(f"    First 500 chars: {output[:500]}")

        return findings

    @staticmethod
    def _parse_cytoscnpy_output(output: str) -> set[Finding]:
        findings: set[Finding] = set()
        try:
            data = cast(object, json.loads(output))
            if not isinstance(data, dict):
                return findings
            data_dict = cast(dict[str, object], data)
            key_to_fallback_type = {
                "unused_functions": "function",
                "unused_methods": "method",
                "unused_imports": "import",
                "unused_classes": "class",
                "unused_variables": "variable",
                "unused_parameters": "variable",
            }
            for key, fallback_type in key_to_fallback_type.items():
                items = data_dict.get(key)
                if not isinstance(items, list):
                    continue
                items_list = cast(list[object], items)
                for item in items_list:
                    if not isinstance(item, dict):
                        continue
                    item_dict = cast(dict[str, object], item)
                    fpath = normalize_path(_as_str(item_dict.get("file")))
                    simple_name = _as_str(item_dict.get("simple_name"))
                    name = _as_str(item_dict.get("name"))
                    item_name = simple_name or (name.split(".")[-1] if name else "")
                    type_name = _as_str(item_dict.get("def_type")) or fallback_type
                    if type_name == "parameter":
                        type_name = "variable"
                    findings.add(
                        (fpath, _as_int(item_dict.get("line")), type_name, item_name)
                    )
        except json.JSONDecodeError as e:
            print(f"[-] JSON Decode Error for CytoScnPy: {e}")
            print(f"    Output start: {output[:100]}")
        return findings

    @staticmethod
    def _parse_skylos_output(output: str) -> set[Finding]:
        try:
            data = cast(object, json.loads(output))
        except json.JSONDecodeError as e:
            print(f"[-] JSON Decode Error for Skylos: {e}")
            print(f"    Output start: {output[:200]}")
            return set()

        findings: set[Finding] = set()
        for item in Verification._skylos_items(data):
            finding = Verification._skylos_item_to_finding(item)
            if finding is not None:
                findings.add(finding)
        return findings

    @staticmethod
    def _skylos_items(data: object) -> list[dict[str, object]]:
        if isinstance(data, list):
            return Verification._as_dict_list(cast(list[object], data))
        if not isinstance(data, dict):
            return []

        data_dict = cast(dict[str, object], data)
        items_list: list[dict[str, object]] = []
        for key in (
            "unused_functions",
            "unused_imports",
            "unused_classes",
            "unused_variables",
            "items",
        ):
            items_list.extend(Verification._as_dict_list(data_dict.get(key)))

        if not items_list:
            items_list.extend(Verification._as_dict_list(data_dict.get("results")))

        return items_list

    @staticmethod
    def _skylos_item_to_finding(item: dict[str, object]) -> Optional[Finding]:
        type_map: Mapping[str, str] = {
            "function": "function",
            "class": "class",
            "import": "import",
            "variable": "variable",
            "parameter": "variable",
            "method": "method",
        }

        skylos_type = _as_str(item.get("type")).lower()
        type_name = type_map.get(skylos_type, skylos_type)
        if not type_name:
            return None

        fpath = normalize_path(_as_str(item.get("file")))
        simple_name = _as_str(item.get("simple_name"))
        full_name = _as_str(item.get("name"))
        item_name = simple_name or (full_name.split(".")[-1] if full_name else "")
        if not item_name:
            return None

        lineno = _as_int(item.get("line"))
        return (fpath, lineno, type_name, item_name)

    @staticmethod
    def _parse_vulture_output(output: str) -> set[Finding]:
        findings: set[Finding] = set()
        for line in output.splitlines():
            parts = line.rsplit(":", 2)
            if len(parts) != 3:
                continue
            fpath = normalize_path(parts[0].strip())
            try:
                lineno = int(parts[1])
            except ValueError:
                continue
            msg = parts[2].strip()
            type_name = "unknown"
            obj_name = "unknown"
            if "unused function" in msg:
                type_name = "function"
                obj_name = msg.split("'")[1]
            elif "unused import" in msg:
                type_name = "import"
                obj_name = msg.split("'")[1]
            elif "unused class" in msg:
                type_name = "class"
                obj_name = msg.split("'")[1]
            elif "unused variable" in msg:
                type_name = "variable"
                obj_name = msg.split("'")[1]
            elif "unused method" in msg:
                type_name = "method"
                obj_name = msg.split("'")[1]
            if type_name != "unknown":
                findings.add((fpath, lineno, type_name, obj_name))
        return findings

    @staticmethod
    def _parse_flake8_output(output: str) -> set[Finding]:
        findings: set[Finding] = set()
        for line in output.splitlines():
            parts = line.rsplit(":", 3)
            if len(parts) != 4:
                continue
            fpath = normalize_path(parts[0].strip())
            try:
                lineno = int(parts[1])
            except ValueError:
                continue
            code = parts[3].strip().split()[0]
            if code == "F401":
                msg = parts[3].strip()
                if "'" in msg:
                    obj_name = msg.split("'")[1]
                    findings.add((fpath, lineno, "import", obj_name))
            elif code == "F841":
                msg = parts[3].strip()
                obj_name = None
                if "`" in msg:
                    obj_name = msg.split("`")[1]
                elif "'" in msg:
                    obj_name = msg.split("'")[1]
                if obj_name:
                    findings.add((fpath, lineno, "variable", obj_name))
        return findings

    @staticmethod
    def _parse_pylint_output(output: str) -> set[Finding]:
        findings: set[Finding] = set()
        try:
            data = cast(object, json.loads(output))
            if not isinstance(data, list):
                return findings
            data_list = cast(list[object], data)
            for item in data_list:
                if not isinstance(item, dict):
                    continue
                item_dict = cast(dict[str, object], item)
                symbol = _as_str(item_dict.get("symbol"))
                if symbol == "unused-import":
                    Verification._handle_pylint_unused_import(item_dict, findings)
                elif symbol in {
                    "unused-variable",
                    "unused-argument",
                    "unused-private-member",
                }:
                    Verification._handle_pylint_unused_variable(item_dict, findings)
        except json.JSONDecodeError:
            pass
        return findings

    @staticmethod
    def _handle_pylint_unused_import(
        item: dict[str, object], findings: set[Finding]
    ) -> None:
        fpath = normalize_path(_as_str(item.get("path")))
        lineno = _as_int(item.get("line"))
        obj_name = _as_str(item.get("obj"))
        if not obj_name:
            msg = _as_str(item.get("message"))
            if "Unused import " in msg:
                obj_name = msg.split("Unused import ")[1].strip()
        findings.add((fpath, lineno, "import", obj_name))

    @staticmethod
    def _handle_pylint_unused_variable(
        item: dict[str, object], findings: set[Finding]
    ) -> None:
        fpath = normalize_path(_as_str(item.get("path")))
        lineno = _as_int(item.get("line"))
        msg = _as_str(item.get("message"))
        obj_name = ""
        if "'" in msg:
            obj_name = msg.split("'")[1]
        elif isinstance(item.get("obj"), str):
            obj_name = _as_str(item.get("obj"))
        if obj_name:
            findings.add((fpath, lineno, "variable", obj_name))

    @staticmethod
    def _parse_ruff_output(output: str) -> set[Finding]:
        findings: set[Finding] = set()
        try:
            data = cast(object, json.loads(output))
            if not isinstance(data, list):
                return findings
            data_list = cast(list[object], data)
            for item in data_list:
                if not isinstance(item, dict):
                    continue
                item_dict = cast(dict[str, object], item)
                code = _as_str(item_dict.get("code"))
                fpath = normalize_path(_as_str(item_dict.get("filename")))
                location = item_dict.get("location")
                lineno = None
                if isinstance(location, dict):
                    location_dict = cast(dict[str, object], location)
                    lineno = _as_int(location_dict.get("row"))
                if code == "F401":
                    msg = _as_str(item_dict.get("message"))
                    if "`" in msg:
                        obj_name = msg.split("`")[1]
                        findings.add((fpath, lineno, "import", obj_name))
                elif code and (
                    code == "F841" or code.startswith("ARG") or code == "B007"
                ):
                    msg = _as_str(item_dict.get("message"))
                    if "`" in msg:
                        obj_name = msg.split("`")[1]
                        findings.add((fpath, lineno, "variable", obj_name))
        except json.JSONDecodeError:
            pass
        return findings

    @staticmethod
    def _parse_dead_output(output: str) -> set[Finding]:
        findings: set[Finding] = set()
        pattern = r"(\w+) is never (?:read|called), defined in (.+):(\d+)"
        for line in output.splitlines():
            match = re.match(pattern, line)
            if not match:
                continue
            obj_name = match.group(1)
            fpath = normalize_path(match.group(2))
            lineno = int(match.group(3))
            type_hint = match.group(0).lower()
            type_name = "function" if "called" in type_hint else "variable"
            findings.add((fpath, lineno, type_name, obj_name))
        return findings

    @staticmethod
    def _parse_uncalled_output(output: str) -> set[Finding]:
        findings: set[Finding] = set()
        pattern = r"(.+\.py):\s*Unused\s+function\s+(\w+)"
        for line in output.splitlines():
            match = re.search(pattern, line, re.IGNORECASE)
            if not match:
                continue
            fpath = normalize_path(match.group(1))
            obj_name = match.group(2)
            findings.add((fpath, None, "function", obj_name))
        return findings

    @staticmethod
    def _parse_deadcode_output(output: str) -> set[Finding]:
        findings: set[Finding] = set()
        pattern = r"(.+\.py):(\d+):\d+:\s*(DC\d+)\s+(\w+)\s+`([^`]+)`"
        type_map = {
            "variable": "variable",
            "function": "function",
            "class": "class",
            "method": "method",
            "attribute": "variable",
            "name": "variable",
            "import": "import",
            "property": "method",
        }
        for line in output.splitlines():
            match = re.search(pattern, line)
            if not match:
                continue
            fpath = normalize_path(match.group(1))
            lineno = int(match.group(2))
            type_raw = match.group(4).lower()
            obj_name = match.group(5)
            type_name = type_map.get(type_raw, type_raw)
            findings.add((fpath, lineno, type_name, obj_name))
        return findings

    def compare(self, tool_name: str, tool_output: str) -> MetricResults:
        """Compare tool output against ground truth."""
        findings = self.parse_tool_output(tool_name, tool_output)
        stats = self._init_stats()
        truth_remaining = list(self.ground_truth)

        for f_item in findings:
            f_file, _, f_type, _ = f_item
            stat_type = f_type if f_type in stats else "overall"
            if not self._is_file_covered(f_file):
                continue

            match = self._find_truth_match(f_item, truth_remaining)
            if match:
                stats["overall"]["TP"] += 1
                truth_remaining.remove(match)
                self._increment_type_tp(stats, match[2])
            else:
                stats["overall"]["FP"] += 1
                if stat_type != "overall" and stat_type in stats:
                    stats[stat_type]["FP"] += 1

        stats["overall"]["FN"] = len(truth_remaining)
        for t_item in truth_remaining:
            t_type = t_item[2]
            if t_type in stats:
                stats[t_type]["FN"] += 1

        return self._compute_metric_results(stats, truth_remaining)

    def _init_stats(self) -> dict[str, dict[str, int]]:
        return {
            "overall": {"TP": 0, "FP": 0, "FN": 0},
            "class": {"TP": 0, "FP": 0, "FN": 0},
            "function": {"TP": 0, "FP": 0, "FN": 0},
            "import": {"TP": 0, "FP": 0, "FN": 0},
            "method": {"TP": 0, "FP": 0, "FN": 0},
            "variable": {"TP": 0, "FP": 0, "FN": 0},
        }

    def _is_file_covered(self, f_file: str) -> bool:
        f_norm = normalize_path(f_file)
        if f_norm in self.covered_files:
            return True
        return any(
            f_norm.endswith(cv) or cv.endswith(f_norm) for cv in self.covered_files
        )

    def _find_truth_match(
        self, f_item: Finding, truth_remaining: list[Finding]
    ) -> Optional[Finding]:
        for t_item in truth_remaining:
            if self._matches_truth_item(f_item, t_item):
                return t_item
        return None

    def _matches_truth_item(self, f_item: Finding, t_item: Finding) -> bool:
        f_file, f_line, f_type, f_name = f_item
        t_file, t_line, t_type, t_name = t_item
        if not self._matches_path(f_file, t_file):
            return False
        if not self._matches_line(f_line, t_line):
            return False
        if not self._matches_type(f_type, t_type):
            return False
        return self._matches_name(f_name, t_name)

    @staticmethod
    def _matches_path(f_file: str, t_file: str) -> bool:
        f_basename = Path(f_file).name
        t_basename = Path(t_file).name
        return (
            f_basename == t_basename
            or f_file.endswith(t_file)
            or t_file.endswith(f_file)
        )

    @staticmethod
    def _matches_line(f_line: Optional[int], t_line: Optional[int]) -> bool:
        if f_line is None:
            return True
        if t_line is None:
            return False
        return abs(f_line - t_line) <= 2

    @staticmethod
    def _matches_type(f_type: str, t_type: str) -> bool:
        return (
            (f_type == t_type)
            or (t_type == "method" and f_type == "function")
            or (t_type == "function" and f_type == "method")
            or (f_type == "function" and t_type in ["variable", "class"])
            or (f_type == "variable" and t_type == "function")
        )

    @staticmethod
    def _matches_name(f_name: str, t_name: str) -> bool:
        f_value = f_name or ""
        t_value = t_name or ""
        f_simple = f_value.split(".")[-1]
        t_simple = t_value.split(".")[-1]
        return f_simple == t_simple or (
            f_value != "" and t_value != "" and f_value == t_value
        )

    @staticmethod
    def _increment_type_tp(stats: dict[str, dict[str, int]], truth_type: str) -> None:
        if truth_type in stats:
            stats[truth_type]["TP"] += 1

    def _compute_metric_results(
        self, stats: dict[str, dict[str, int]], truth_remaining: list[Finding]
    ) -> MetricResults:
        results: MetricResults = {}
        for key, s in stats.items():
            tp = s["TP"]
            fp = s["FP"]
            fn = s["FN"]
            precision = tp / (tp + fp) if (tp + fp) > 0 else 0
            recall = tp / (tp + fn) if (tp + fn) > 0 else 0
            f1 = (
                2 * (precision * recall) / (precision + recall)
                if (precision + recall) > 0
                else 0
            )
            results[key] = {
                "TP": tp,
                "FP": fp,
                "FN": fn,
                "Precision": precision,
                "Recall": recall,
                "F1": f1,
                "missed_items": self._format_missed_items(key, truth_remaining),
            }
        return results

    @staticmethod
    def _format_missed_items(key: str, truth_remaining: list[Finding]) -> list[str]:
        return [
            f"{t[3]} ({Path(t[0]).name}:{t[1]})"
            for t in truth_remaining
            if t[2] == key or key == "overall"
        ]


def main():  # noqa: C901
    """Main entry point."""
    print("CytoScnPy Benchmark & Verification Utility")
    print("==========================================")

    if psutil is None:
        print(
            "[!] 'psutil' module not found. Memory benchmarking will be inaccurate (0 MB)."
        )
        print("    Install with: pip install psutil")

    # Determine paths relative to this script
    script_dir = Path(__file__).parent.resolve()
    project_root = script_dir.parent.resolve()

    # Parse CLI Arguments
    import argparse

    parser = argparse.ArgumentParser(
        description="CytoScnPy Benchmark & Verification Utility"
    )
    _ = parser.add_argument(
        "-l", "--list", action="store_true", help="List available tools and exit"
    )
    _ = parser.add_argument(
        "-c", "--check", action="store_true", help="Check tool availability and exit"
    )
    _ = parser.add_argument(
        "-i",
        "--include",
        nargs="+",
        help="Run only specific tools (substring match, case-insensitive)",
    )
    _ = parser.add_argument(
        "-e",
        "--exclude",
        nargs="+",
        help="Exclude specific tools (substring match, case-insensitive)",
    )
    _ = parser.add_argument("--save-json", help="Save benchmark results to a JSON file")
    _ = parser.add_argument(
        "--compare-json", help="Compare current results against a baseline JSON file"
    )
    _ = parser.add_argument(
        "--threshold",
        type=float,
        default=0.10,
        help="Regression threshold ratio (default: 0.10 = 10%%)",
    )
    args = cast(_Args, parser.parse_args())

    # Define tools to run
    # We run on the examples directory which contains multiple subdirectories with ground truth
    target_dir = script_dir / "examples"
    ground_truth_path = target_dir  # Pass directory to load recursively

    if not target_dir.exists():
        print(f"[-] Target directory not found: {target_dir}")
        return

    # Build Rust if generic run or specifically requested
    # Only skip Rust build if we are exclusively running NON-CytoScnPy tools and user didn't ask for it?
    # For simplicity, we always build unless we are careful.
    # Let's keep build step but maybe make it conditional if user only wants to run 'pylint'?
    # For now, keep it simple: always build unless simple filtering suggests otherwise.

    # Actually, let's define tools LIST first so we can use it for --list and filtering

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

    # Handle --list
    if args.list:
        print("Available tools:")
        for tool in all_tools:
            print(f"  - {tool['name']}")
        return

    # Handle --check
    if args.check:
        check_tool_availability(all_tools)
        return

    # Filter Tools
    tools_to_run: list[ToolConfig] = []
    for tool in all_tools:
        name_lower = tool["name"].lower()

        # Check Exclude
        if args.exclude:
            if any(ex.lower() in name_lower for ex in args.exclude):
                continue

        # Check Include (if specified, must match at least one)
        if args.include:
            if not any(inc.lower() in name_lower for inc in args.include):
                continue

        tools_to_run.append(tool)

    if not tools_to_run:
        print("[-] No tools selected to run.")
        return

    # Check tool availability and filter
    availability = check_tool_availability(tools_to_run)
    tools_to_run = [
        t
        for t in tools_to_run
        if availability.get(t["name"], {}).get("available", False)
    ]

    if not tools_to_run:
        print("[-] No available tools to run.")
        return

    # Build Rust project ONLY if we are running CytoScnPy (Rust)
    run_rust_build = any("CytoScnPy (Rust)" in t["name"] for t in tools_to_run)

    if run_rust_build:
        print("\n[+] Building Rust project...")
        cargo_toml = project_root / "Cargo.toml"
        if not cargo_toml.exists():
            # Fallback to sub-directory if not in root
            cargo_toml = project_root / "cytoscnpy" / "Cargo.toml"

        if not cargo_toml.exists():
            print(
                f"[-] Cargo.toml not found in {project_root} or {project_root / 'cytoscnpy'}"
            )
            return

        build_cmd = ["cargo", "build", "--release", "-p", "cytoscnpy"]
        if cargo_toml.parent != project_root:
            build_cmd.extend(["--manifest-path", str(cargo_toml)])
        subprocess.run(build_cmd, shell=False, check=True)
        print("[+] Rust build successful.")

        # Check binary again after build
        if not rust_bin.exists():
            print(f"[-] Rust binary still not found at {rust_bin} after build.")

    print(f"\n[+] Loading Ground Truth recursively from {ground_truth_path}...")
    verifier = Verification(str(ground_truth_path))

    results: list[BenchmarkResult] = []
    verification_results: list[VerificationResult] = []

    print(f"\n[+] Running {len(tools_to_run)} tools...")

    for tool in tools_to_run:
        if tool["command"]:
            cwd_value = tool.get("cwd")
            cwd = cwd_value if isinstance(cwd_value, str) else None
            env_value = tool.get("env")
            env = env_value if isinstance(env_value, Mapping) else None
            res = run_benchmark_tool(
                tool["name"],
                tool["command"],
                cwd=cwd,
                env=env,
            )
            if res:
                results.append(res)
                # Verify
                # Use clean stdout if available to avoid stderr pollution (e.g. logging/errors mixed with JSON)
                stdout = res["stdout"]
                output_to_parse = stdout if stdout else res["output"]
                metrics = verifier.compare(tool["name"], output_to_parse)
                verification_entry: VerificationResult = dict(metrics)
                verification_entry["Tool"] = tool["name"]
                verification_results.append(verification_entry)
        else:
            print(f"\n[-] Skipping {tool['name']} (not found)")

    # Print Benchmark Results
    print("\n[=] Benchmark Results")
    print(f"{'Tool':<20} | {'Time (s)':<10} | {'Mem (MB)':<10} | {'Issues (Est)':<12}")
    print("-" * 60)

    for res in results:
        print(
            f"{res['name']:<20} | {res['time']:<10.3f} | {res['memory_mb']:<10.2f} | {res['issues']:<12}"
        )

    print("-" * 60)

    # Print Verification Results
    print("\n[=] Verification Results (Ground Truth Comparison)")

    # Define types to print
    types_to_print = ["overall", "class", "function", "import", "method", "variable"]

    for type_key in types_to_print:
        print(f"\n--- {type_key.capitalize()} Detection ---")
        print(
            f"{'Tool':<20} | {'TP':<5} | {'FP':<5} | {'FN':<5} | {'Precision':<10} | {'Recall':<10} | {'F1 Score':<10}"
        )
        print("-" * 80)

        for v in verification_results:
            stats_value = v.get(type_key)
            if isinstance(stats_value, dict):
                print(
                    f"{_as_str(v.get('Tool')):<20} | {stats_value['TP']:<5} | {stats_value['FP']:<5} | {stats_value['FN']:<5} | {stats_value['Precision']:<10.4f} | {stats_value['Recall']:<10.4f} | {stats_value['F1']:<10.4f}"
                )
        print("-" * 80)

    # Compile Final JSON Report
    final_report: FinalReport = {
        "timestamp": time.time(),
        "platform": sys.platform,
        "results": [],
    }

    for res in results:
        # Find corresponding verification result
        v_res = next(
            (v for v in verification_results if _as_str(v.get("Tool")) == res["name"]),
            None,
        )

        f1_score = 0.0
        precision = 0.0
        recall = 0.0
        stats_payload: Mapping[str, object] = {}
        if v_res:
            stats_payload = v_res
            overall_value = v_res.get("overall")
            if isinstance(overall_value, dict):
                f1_score = overall_value["F1"]
                precision = overall_value["Precision"]
                recall = overall_value["Recall"]

        entry: FinalReportEntry = {
            "name": res["name"],
            "time": res["time"],
            "memory_mb": res["memory_mb"],
            "issues": res["issues"],
            "f1_score": f1_score,  # Use overall F1
            "precision": precision,
            "recall": recall,
            "stats": stats_payload,
        }
        final_report["results"].append(entry)

    # Save JSON if requested
    if args.save_json:
        try:
            with Path(args.save_json).open("w") as f:
                json.dump(final_report, f, indent=2)
            print(f"\n[+] Results saved to {args.save_json}")
        except OSError as e:
            print(f"[-] Failed to save JSON results: {e}")

    # Compare against baseline if requested
    if args.compare_json:
        print(f"\n[+] Comparing against baseline: {args.compare_json}")
        try:
            with Path(args.compare_json).open() as f:
                baseline = cast(object, json.load(f))

            baseline_dict = (
                cast(dict[str, object], baseline)
                if isinstance(baseline, dict)
                else None
            )
            if (
                baseline_dict is not None
                and baseline_dict.get("platform") != sys.platform
            ):
                print(
                    f"[!] WARNING: Baseline platform ({baseline_dict.get('platform')}) does not match current system ({sys.platform}). Performance comparison may be inaccurate."
                )

            cytoscnpy_regressions: list[str] = []
            other_regressions: list[str] = []
            results_list = final_report["results"]
            for current in results_list:
                base_candidates: list[dict[str, object]] = []
                if baseline_dict is not None:
                    base_results = baseline_dict.get("results")
                    if isinstance(base_results, list):
                        base_results_list = cast(list[object], base_results)
                        for item in base_results_list:
                            if isinstance(item, dict):
                                base_candidates.append(cast(dict[str, object], item))  # noqa: PERF401
                base = next(
                    (
                        b
                        for b in base_candidates
                        if _as_str(b.get("name")) == current["name"]
                    ),
                    None,
                )

                if not base:
                    print(f"    [?] New tool found (no baseline): {current['name']}")
                    continue

                base_time = _as_float(base.get("time"))
                base_memory = _as_float(base.get("memory_mb"))
                base_f1 = _as_float(base.get("f1_score"))
                if base_time is None or base_memory is None or base_f1 is None:
                    raise ValueError(
                        f"Baseline entry missing numeric fields for {current['name']}"
                    )

                # Determine if this is CytoScnPy or a comparison tool
                is_cytoscnpy = "CytoScnPy" in current["name"]
                # Check Time
                time_diff = current["time"] - base_time
                time_ratio = time_diff / base_time if base_time > 0 else 0
                if time_ratio > args.threshold:
                    # Ignore small time increases (< 1.0s) to avoid noise
                    if time_diff > 1.0:
                        regression_msg = f"{current['name']} Time: {base_time:.3f}s -> {current['time']:.3f}s (+{time_ratio * 100:.1f}%)"
                        if is_cytoscnpy:
                            cytoscnpy_regressions.append(regression_msg)
                        else:
                            other_regressions.append(regression_msg)

                # Check Memory
                mem_diff = current["memory_mb"] - base_memory
                mem_ratio = mem_diff / base_memory if base_memory > 0 else 0
                if mem_ratio > args.threshold:
                    # Ignore small memory increases (< 10MB) to avoid CI noise
                    if mem_diff > 10.0:
                        regression_msg = f"{current['name']} Memory: {base_memory:.1f}MB -> {current['memory_mb']:.1f}MB (+{mem_ratio * 100:.1f}%)"
                        if is_cytoscnpy:
                            cytoscnpy_regressions.append(regression_msg)
                        else:
                            other_regressions.append(regression_msg)

                # Check F1 Score (Regression if strictly lower, handling float precision)
                f1_diff = base_f1 - current["f1_score"]
                if f1_diff > 0.001:  # Tolerance for float comparison
                    regression_msg = f"{current['name']} F1 Score: {base_f1:.4f} -> {current['f1_score']:.4f} (-{f1_diff:.4f})"
                    if is_cytoscnpy:
                        cytoscnpy_regressions.append(regression_msg)
                    else:
                        other_regressions.append(regression_msg)

                # Check Precision (Regression if drops more than 0.01)
                base_precision = _as_float(base.get("precision"))
                if base_precision is not None:
                    prec_diff = base_precision - current["precision"]
                    if prec_diff > 0.01:
                        regression_msg = f"{current['name']} Precision: {base_precision:.4f} -> {current['precision']:.4f} (-{prec_diff:.4f})"
                        if is_cytoscnpy:
                            cytoscnpy_regressions.append(regression_msg)
                        else:
                            other_regressions.append(regression_msg)

                # Check Recall (Regression if drops more than 0.01)
                base_recall = _as_float(base.get("recall"))
                if base_recall is not None:
                    recall_diff = base_recall - current["recall"]
                    if recall_diff > 0.01:
                        regression_msg = f"{current['name']} Recall: {base_recall:.4f} -> {current['recall']:.4f} (-{recall_diff:.4f})"
                        if is_cytoscnpy:
                            cytoscnpy_regressions.append(regression_msg)
                        else:
                            other_regressions.append(regression_msg)

            # Report comparison tool regressions as warnings (informational, non-blocking)
            if other_regressions:
                print(
                    "\n[!] WARNING: Comparison tool regressions detected (informational only):"
                )
                for r in other_regressions:
                    print(f"    - {r}")

            # Only fail CI/CD if CytoScnPy itself regressed
            if cytoscnpy_regressions:
                print("\n[!] CYTOSCNPY PERFORMANCE REGRESSIONS DETECTED:")
                for r in cytoscnpy_regressions:
                    print(f"    - {r}")
                sys.exit(1)
            else:
                print("\n[OK] No CytoScnPy regressions detected.")

        except FileNotFoundError:
            print(f"[-] Baseline file not found: {args.compare_json}")
            sys.exit(1)
        except (OSError, json.JSONDecodeError, ValueError) as e:
            print(f"[-] Error comparing baseline: {e}")
            sys.exit(1)


if __name__ == "__main__":
    main()
