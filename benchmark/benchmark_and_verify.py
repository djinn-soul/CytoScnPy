import subprocess
import json
import time
import sys
import os
import shutil
from pathlib import Path

try:
    import psutil
except ImportError:
    psutil = None

import threading

def run_command(command, cwd=None, env=None, timeout=300):
    """
    Runs a command and returns (result, duration, max_rss_mb).
    """
    start_time = time.time()
    
    # Determine if we should use shell=True
    use_shell = True
    if isinstance(command, list):
        use_shell = False
    
    # We need to use Popen to track memory usage with psutil
    process = subprocess.Popen(
        command,
        cwd=cwd,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        stdin=subprocess.DEVNULL, # Prevent interactive prompts
        text=True,
        shell=use_shell
    )
    
    max_rss = [0] # Use list for mutable closure
    stop_monitoring = threading.Event()
    
    def monitor_memory():
        if not psutil:
            return
            
        try:
            p = psutil.Process(process.pid)
            while not stop_monitoring.is_set():
                try:
                    if process.poll() is not None:
                        break
                        
                    # memory_info().rss is in bytes
                    rss = p.memory_info().rss
                    if rss > max_rss[0]:
                        max_rss[0] = rss
                    # Check children too
                    for child in p.children(recursive=True):
                        try:
                            child_rss = child.memory_info().rss
                            # Summing for total footprint approximation
                            rss += child_rss
                        except (psutil.NoSuchProcess, psutil.AccessDenied):
                            pass
                    
                    if rss > max_rss[0]:
                        max_rss[0] = rss
                        
                except (psutil.NoSuchProcess, psutil.AccessDenied):
                    pass
                time.sleep(0.01) # Poll interval
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            pass

    # Start memory monitoring thread
    monitor_thread = threading.Thread(target=monitor_memory)
    monitor_thread.start()
    
    try:
        stdout, stderr = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        process.kill()
        stdout, stderr = process.communicate()
        stop_monitoring.set()
        monitor_thread.join()
        return subprocess.CompletedProcess(command, -1, stdout, stderr + "\nTimeout"), timeout, max_rss[0] / (1024 * 1024)
        
    stop_monitoring.set()
    monitor_thread.join()
    
    end_time = time.time()
    duration = end_time - start_time
    
    # Create a result object similar to subprocess.run
    result = subprocess.CompletedProcess(command, process.returncode, stdout, stderr)
    
    return result, duration, max_rss[0] / (1024 * 1024)

def normalize_path(p):
    return str(Path(p).as_posix()).strip("/")

def get_tool_path(tool_name):
    # Check PATH first
    path = shutil.which(tool_name)
    if path:
        return path
    
    # Check current environment scripts
    scripts_dir = Path(sys.prefix) / "Scripts" if sys.platform == "win32" else Path(sys.prefix) / "bin"
    possible_path = scripts_dir / (tool_name + ".exe" if sys.platform == "win32" else tool_name)
    
    if possible_path.exists():
        return str(possible_path)
        
    return None

def check_tool_availability(tools_config, env=None):
    """
    Pre-check all tools to verify they are installed and available.
    Returns a dict with tool status: {name: {"available": bool, "reason": str}}
    """
    print("\n[+] Checking tool availability...")
    results = {}
    
    for tool in tools_config:
        name = tool["name"]
        command = tool.get("command", "")
        tool_env = tool.get("env", env)
        
        status = {"available": False, "reason": "Unknown"}
        
        if not command:
            status["reason"] = "No command configured"
            results[name] = status
            continue
        
        # Special checks for each tool type
        if name == "CytoScnPy (Rust)":
            # Check if binary exists
            bin_path = None
            if isinstance(command, list):
                bin_path = Path(command[0])
            else:
                import re
                match = re.search(r'"([^"]+)"', command)
                if match:
                    bin_path = Path(match.group(1))
            
            if bin_path:
                if bin_path.exists():
                    status = {"available": True, "reason": "Binary found"}
                else:
                    status["reason"] = f"Binary not found: {bin_path}"
            else:
                status["reason"] = "Could not parse binary path"
                
        elif name == "CytoScnPy (Python)":
            # Check if cytoscnpy module is importable
            try:
                result = subprocess.run(
                    [sys.executable, "-c", "import cytoscnpy"],
                    capture_output=True, text=True, timeout=10
                )
                if result.returncode == 0:
                    status = {"available": True, "reason": "Module importable"}
                else:
                    status["reason"] = "Module not installed (pip install -e .)"
            except Exception as e:
                status["reason"] = f"Check failed: {e}"
                
        elif name == "Skylos":
            # Check if skylos is installed
            skylos_path = get_tool_path("skylos")
            if skylos_path:
                status = {"available": True, "reason": f"Found at {skylos_path}"}
            else:
                # Also try as module
                try:
                    result = subprocess.run(
                        [sys.executable, "-m", "skylos", "--help"],
                        capture_output=True, text=True, timeout=10
                    )
                    if result.returncode == 0:
                        status = {"available": True, "reason": "Available as module"}
                    else:
                        status["reason"] = "Not installed (pip install skylos)"
                except Exception:
                    status["reason"] = "Not installed (pip install skylos)"
                    
        elif "Vulture" in name:
            try:
                result = subprocess.run(
                    [sys.executable, "-m", "vulture", "--version"],
                    capture_output=True, text=True, timeout=10
                )
                if result.returncode == 0:
                    status = {"available": True, "reason": "Installed"}
                else:
                    status["reason"] = "Not installed (pip install vulture)"
            except Exception:
                status["reason"] = "Not installed (pip install vulture)"
                
        elif name == "Flake8":
            try:
                result = subprocess.run(
                    [sys.executable, "-m", "flake8", "--version"],
                    capture_output=True, text=True, timeout=10
                )
                if result.returncode == 0:
                    status = {"available": True, "reason": "Installed"}
                else:
                    status["reason"] = "Not installed (pip install flake8)"
            except Exception:
                status["reason"] = "Not installed (pip install flake8)"
                
        elif name == "Pylint":
            try:
                result = subprocess.run(
                    [sys.executable, "-m", "pylint", "--version"],
                    capture_output=True, text=True, timeout=10
                )
                if result.returncode == 0:
                    status = {"available": True, "reason": "Installed"}
                else:
                    status["reason"] = "Not installed (pip install pylint)"
            except Exception:
                status["reason"] = "Not installed (pip install pylint)"
                
        elif name == "Ruff":
            try:
                result = subprocess.run(
                    [sys.executable, "-m", "ruff", "--version"],
                    capture_output=True, text=True, timeout=10
                )
                if result.returncode == 0:
                    status = {"available": True, "reason": "Installed"}
                else:
                    status["reason"] = "Not installed (pip install ruff)"
            except Exception:
                status["reason"] = "Not installed (pip install ruff)"
                
        elif name == "uncalled":
            try:
                result = subprocess.run(
                    [sys.executable, "-m", "uncalled", "--help"],
                    capture_output=True, text=True, timeout=10
                )
                if result.returncode == 0:
                    status = {"available": True, "reason": "Installed"}
                else:
                    status["reason"] = "Not installed (pip install uncalled)"
            except Exception:
                status["reason"] = "Not installed (pip install uncalled)"
                
        elif name == "dead":
            try:
                result = subprocess.run(
                    [sys.executable, "-m", "dead", "--help"],
                    capture_output=True, text=True, timeout=10
                )
                if result.returncode == 0:
                    status = {"available": True, "reason": "Installed"}
                else:
                    status["reason"] = "Not installed (pip install dead)"
            except Exception:
                status["reason"] = "Not installed (pip install dead)"
        else:
            # Generic check - assume available if command is set
            status = {"available": True, "reason": "Command configured"}
            
        results[name] = status
    
    # Print summary
    available_count = sum(1 for s in results.values() if s["available"])
    print(f"\n    Tool Availability: {available_count}/{len(results)} tools ready")
    print("-" * 60)
    for name, status in results.items():
        icon = "[OK]" if status["available"] else "[X] "
        print(f"    {icon} {name}: {status['reason']}")
    print("-" * 60)
    
    return results


def run_benchmark_tool(name, command, cwd=None, env=None):
    print(f"\n[+] Running {name}...")
    print(f"    Command: {command}")
    if not command:
        print(f"[-] {name} command not found/configured.")
        return None

    result, duration, max_rss = run_command(command, cwd, env)
    print(f"    [OK] Completed in {duration:.2f}s (Memory: {max_rss:.1f} MB)")
    
    if result.returncode != 0 and name not in ["Pylint", "Flake8", "Vulture", "uncalled", "dead", "deadcode"]: 
        # Some tools return non-zero on finding issues, which is expected.
        # But if it crashes (like Rust build), we want to know.
        # For linters, we assume non-zero means "issues found" or "error".
        # We'll log stderr if it looks like a crash.
        pass

    # Try to parse issue count (very rough heuristics)
    issue_count = 0
    output = result.stdout + result.stderr
    
    if name == "CytoScnPy (Rust)":
        try:
            data = json.loads(result.stdout)
            issue_count = sum(len(data.get(k, [])) for k in ["unused_functions", "unused_imports", "unused_classes", "unused_variables"])
        except:
            pass
    elif name == "CytoScnPy (Python)":
        try:
            data = json.loads(result.stdout)
            issue_count = sum(len(data.get(k, [])) for k in ["unused_functions", "unused_imports", "unused_classes", "unused_variables"])
        except:
            pass
    elif name == "Ruff":
        # Ruff output lines usually indicate issues? Or use --json?
        # Let's assume standard output lines = issues
        issue_count = len(output.strip().splitlines())
    elif name == "Flake8":
        issue_count = len(output.strip().splitlines())
    elif name == "Pylint":
        # Pylint output is verbose. Hard to count without parsing.
        issue_count = len([l for l in output.splitlines() if ": " in l]) # Heuristic
    elif "Vulture" in name:
        issue_count = len(output.strip().splitlines())
    elif name == "uncalled":
        issue_count = len([l for l in output.splitlines() if "unused" in l.lower()])
    elif name == "dead":
        # dead outputs lines like "func is never read, defined in file.py:line"
        issue_count = len([l for l in output.splitlines() if "is never" in l.lower() or "never read" in l.lower()])
    elif name == "deadcode":
        # deadcode outputs lines like "file.py:10:0: DC02 Function `func_name` is never used"
        # DC codes range from DC01 to DC13
        import re
        issue_count = len([l for l in output.splitlines() if re.search(r': DC\d+', l)])
    elif name == "Skylos":
        try:
            data = json.loads(result.stdout)
            issue_count = sum(len(data.get(k, [])) for k in ["unused_functions", "unused_imports", "unused_classes", "unused_variables"])
        except:
            pass

    return {
        "name": name,
        "time": duration,
        "memory_mb": max_rss,
        "issues": issue_count,
        "output": output,
        "stdout": result.stdout  # Keep separate for JSON parsing
    }

class Verification:
    def __init__(self, ground_truth_path):
        self.ground_truth = self.load_ground_truth(ground_truth_path)

    def load_ground_truth(self, path):
        path_obj = Path(path)
        truth_set = set()
        self.covered_files = set()
        
        files_to_load = []
        if path_obj.is_dir():
            files_to_load = list(path_obj.rglob("ground_truth.json"))
        elif path_obj.exists():
            files_to_load = [path_obj]
            
        for p in files_to_load:
            try:
                with open(p, 'r') as f:
                    data = json.load(f)
                
                # Normalize ground truth into a set of (file, line, type, name)
                # We focus on "dead_items"
                for file_path, content in data.get("files", {}).items():
                    # If file_path is relative, it should be relative to the ground_truth.json location
                    # But our findings normalization might be tricky.
                    # Let's assume file_path in json is relative to the folder containing ground_truth.json
                    # We need to make it consistent with tool output which might be absolute or relative to CWD.
                    # Let's store the absolute path or a robust relative path.
                    
                    # Actually, the tools are run on `target_dir`. 
                    # If we run on `examples`, tools report `examples/complex/file.py`.
                    # The ground truth in `examples/complex/ground_truth.json` might say `file.py`.
                    # We need to join them.
                    
                    base_dir = p.parent
                    full_path = (base_dir / file_path).resolve()
                    # We'll store the resolved path string for matching
                    # But we need to handle how `compare` matches.
                    # `compare` does `f_file.endswith(t_file)`.
                    # If we store absolute path in truth, `endswith` works if finding is absolute.
                    # If finding is relative `examples/complex/file.py`, and truth is `E:/.../examples/complex/file.py`,
                    # `endswith` works if we check `t_file.endswith(f_file)` or similar.
                    # Let's store the full path string.
                    
                    t_file_str = normalize_path(str(full_path))
                    self.covered_files.add(t_file_str)

                    for item in content.get("dead_items", []):
                        truth_set.add((
                            t_file_str,
                            item.get("line_start"),
                            item.get("type"),
                            item.get("name")
                        ))
            except Exception as e:
                print(f"[-] Error loading ground truth from {p}: {e}")
                
        return truth_set

    def parse_tool_output(self, name, output):
        findings = set()
        
        if name in ["CytoScnPy (Rust)", "CytoScnPy (Python)"]:
            try:
                data = json.loads(output)
                # CytoScnPy outputs arrays by type
                # Map JSON keys to fallback types (used if def_type field is missing)
                key_to_fallback_type = {
                    "unused_functions": "function",
                    "unused_methods": "method",
                    "unused_imports": "import",
                    "unused_classes": "class",
                    "unused_variables": "variable"
                }
                
                for key, fallback_type in key_to_fallback_type.items():
                    for item in data.get(key, []):
                        fpath = normalize_path(item.get("file", ""))
                        item_name = item.get("simple_name") or (item.get("name", "").split(".")[-1] if item.get("name") else "")
                        # Use def_type from JSON if available (e.g., "method" vs "function")
                        # Fall back to key-based type for backward compatibility
                        type_name = item.get("def_type", fallback_type)
                        findings.add((fpath, item.get("line"), type_name, item_name))
            except json.JSONDecodeError as e:
                print(f"[-] JSON Decode Error for {name}: {e}")
                print(f"    Output start: {output[:100]}")
                
        elif name == "Skylos":
            # Skylos outputs a flat list with 'type' field per item
            try:
                data = json.loads(output)
                # Skylos JSON: {"unused_functions": [...], ...} OR list of items with 'type' field
                # Check for flat list structure (list of dicts with 'type')
                items_list = []
                
                if isinstance(data, list):
                    items_list = data
                elif isinstance(data, dict):
                    # Could be {"unused_functions": [...], ...} or have an "items" key
                    for key in ["unused_functions", "unused_imports", "unused_classes", "unused_variables", "items"]:
                        if key in data and isinstance(data[key], list):
                            items_list.extend(data[key])
                    # Also check for results under different structure
                    if not items_list and "results" in data:
                        items_list = data.get("results", [])
                
                # Map Skylos types to ground truth types
                type_map = {
                    "function": "function",
                    "class": "class", 
                    "import": "import",
                    "variable": "variable",
                    "parameter": "variable",  # Map parameters to variables
                    "method": "method"
                }
                
                for item in items_list:
                    if isinstance(item, dict):
                        skylos_type = item.get("type", "").lower()
                        type_name = type_map.get(skylos_type, skylos_type)
                        fpath = normalize_path(item.get("file", ""))
                        item_name = item.get("simple_name") or item.get("name", "").split(".")[-1]
                        lineno = item.get("line")
                        if type_name and item_name:
                            findings.add((fpath, lineno, type_name, item_name))
                            
            except json.JSONDecodeError as e:
                print(f"[-] JSON Decode Error for {name}: {e}")
                print(f"    Output start: {output[:200]}")
                
        elif "Vulture" in name:
            # Output: file.py:line: unused function 'foo' (60% confidence)
            for line in output.splitlines():
                # Use rsplit to handle Windows paths (drive letter :) safely
                # Expect: path:line: message
                parts = line.rsplit(":", 2)
                if len(parts) == 3:
                    fpath = normalize_path(parts[0].strip())
                    try:
                        lineno = int(parts[1])
                        msg = parts[2].strip()
                        # Extract name and type from message
                        # "unused function 'foo'"
                        # "unused import 'os'"
                        # "unused class 'Bar'"
                        # "unused variable 'x'"
                        # "unused property 'y'"
                        # "unused attribute 'z'"
                        # "unused method 'm'"
                        
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
                    except ValueError:
                        pass

        elif name == "Flake8":
            # Output: file.py:line:col: code message
            # F401 module imported but unused
            for line in output.splitlines():
                # Use rsplit to handle Windows paths
                # Expect: path:line:col: code message
                parts = line.rsplit(":", 3)
                if len(parts) == 4:
                    fpath = normalize_path(parts[0].strip())
                    try:
                        lineno = int(parts[1])
                        code = parts[3].strip().split()[0]
                        
                        if code == "F401": # Unused import
                            # Flake8 doesn't easily give the name in a standard way without parsing message
                            # "module imported but unused" - doesn't say which one easily?
                            # Actually usually: "F401 'os' imported but unused"
                            msg = parts[3].strip()
                            if "'" in msg:
                                obj_name = msg.split("'")[1]
                                findings.add((fpath, lineno, "import", obj_name))
                    except ValueError:
                        pass

        elif name == "Pylint":
            # JSON output expected
            try:
                data = json.loads(output)
                for item in data:
                    if item["symbol"] == "unused-import":
                        fpath = normalize_path(item["path"])
                        lineno = item["line"]
                        obj_name = item.get("obj", "") or ""
                        
                        # Fallback: extract from message "Unused import json"
                        if not obj_name and "message" in item:
                             msg = item["message"]
                             if "Unused import " in msg:
                                 obj_name = msg.split("Unused import ")[1].strip()
                        
                        findings.add((fpath, lineno, "import", obj_name))
                    elif item["symbol"] == "unused-variable":
                        fpath = normalize_path(item["path"])
                        lineno = item["line"]
                        # Extract variable name from message, not obj (obj is enclosing scope)
                        msg = item.get("message", "")
                        obj_name = ""
                        if "'" in msg:
                            obj_name = msg.split("'")[1]
                        elif item.get("obj"):
                            obj_name = item["obj"]
                        if obj_name:
                            findings.add((fpath, lineno, "variable", obj_name))
                    # Pylint has many unused codes
            except json.JSONDecodeError:
                pass

        elif name == "Ruff":
            # JSON output expected
            try:
                data = json.loads(output)
                for item in data:
                    code = item.get("code")
                    fpath = normalize_path(item.get("filename", ""))
                    lineno = item.get("location", {}).get("row")
                    
                    if code == "F401":  # Unused import
                        # Ruff message: "`os` imported but unused"
                        msg = item.get("message", "")
                        if "`" in msg:
                            obj_name = msg.split("`")[1]
                            findings.add((fpath, lineno, "import", obj_name))
                    elif code == "F841":  # Local variable assigned but never used
                        # Ruff message: "Local variable `x` is assigned but never used"
                        msg = item.get("message", "")
                        if "`" in msg:
                            obj_name = msg.split("`")[1]
                            findings.add((fpath, lineno, "variable", obj_name))
            except json.JSONDecodeError:
                pass

        elif name == "dead":
            # dead output: "func_name is never read, defined in file.py:line"
            import re
            pattern = r"(\w+) is never (?:read|called), defined in (.+):(\d+)"
            for line in output.splitlines():
                match = re.match(pattern, line)
                if match:
                    obj_name = match.group(1)
                    fpath = normalize_path(match.group(2))
                    lineno = int(match.group(3))
                    # dead reports functions/variables, we'll assume function
                    findings.add((fpath, lineno, "function", obj_name))

        elif name == "uncalled":
            # uncalled output format: "file.py: Unused function func_name" (no line number!)
            import re
            pattern = r"(.+\.py):\s*Unused\s+function\s+(\w+)"
            for line in output.splitlines():
                match = re.search(pattern, line, re.IGNORECASE)
                if match:
                    fpath = normalize_path(match.group(1))
                    obj_name = match.group(2)
                    # uncalled doesn't provide line numbers, so we set to None
                    findings.add((fpath, None, "function", obj_name))

        elif name == "deadcode":
            # deadcode output format: "file.py:10:0: DC02 Function `func_name` is never used"
            # Rules: DC01=variable, DC02=function, DC03=class, DC04=method, DC05=attribute
            #        DC06=name, DC07=import, DC08=property
            import re
            # Pattern: path:line:col: DCxx Type `name` is never used
            # Note: deadcode uses backticks (`) not single quotes (')
            pattern = r"(.+\.py):(\d+):\d+:\s*(DC\d+)\s+(\w+)\s+`([^`]+)`"
            for line in output.splitlines():
                match = re.search(pattern, line)
                if match:
                    fpath = normalize_path(match.group(1))
                    lineno = int(match.group(2))
                    code = match.group(3)
                    type_raw = match.group(4).lower()  # "Function", "Variable", etc.
                    obj_name = match.group(5)
                    
                    # Map deadcode types to our standard types based on official docs:
                    # DC01=unused-variable, DC02=unused-function, DC03=unused-class,
                    # DC04=unused-method, DC05=unused-attribute, DC06=unused-name,
                    # DC07=unused-import, DC08=unused-property
                    type_map = {
                        "variable": "variable",
                        "function": "function",
                        "class": "class",
                        "method": "method",
                        "attribute": "variable",
                        "name": "variable",  # DC06 unused-name
                        "import": "import",
                        "property": "method",  # DC08 - treat property like method
                    }
                    type_name = type_map.get(type_raw, type_raw)
                    findings.add((fpath, lineno, type_name, obj_name))

        # Debug output for tools with no parsed findings
        if not findings and output.strip():
            print(f"DEBUG: {name} produced output but no findings parsed:")
            print(f"    First 500 chars: {output[:500]}")
            
        return findings

    def compare(self, tool_name, tool_output):
        findings = self.parse_tool_output(tool_name, tool_output)
        
        # Initialize stats per type
        stats = {
            "overall": {"TP": 0, "FP": 0, "FN": 0},
            "class": {"TP": 0, "FP": 0, "FN": 0},
            "function": {"TP": 0, "FP": 0, "FN": 0},
            "import": {"TP": 0, "FP": 0, "FN": 0},
            "method": {"TP": 0, "FP": 0, "FN": 0},
            "variable": {"TP": 0, "FP": 0, "FN": 0}
        }

        # Create a copy of truth to mark found items
        truth_remaining = list(self.ground_truth)
        
        # Track matched findings to avoid double counting for FP
        matched_findings = set()

        # For each finding, check if it matches a truth item
        for f_item in findings:
            f_file, f_line, f_type, f_name = f_item
            match = None
            
            # Normalize finding type for stats
            stat_type = f_type
            if stat_type not in stats:
                stat_type = "overall" # Should not happen if types are normalized in parse

            # SKYLOS FILTER: If file is not in ground truth covered files, ignore it.
            is_covered = False
            f_file_norm = normalize_path(f_file)
            
            if f_file_norm in self.covered_files:
                is_covered = True
            else:
                for cv in self.covered_files:
                    if f_file_norm.endswith(cv) or cv.endswith(f_file_norm):
                        is_covered = True
                        break
            
            if not is_covered:
                continue

            for t_item in truth_remaining:
                t_file, t_line, t_type, t_name = t_item
                
                # Path matching: compare basenames or check if one path ends with the other
                f_basename = os.path.basename(f_file)
                t_basename = os.path.basename(t_file)
                path_match = (f_basename == t_basename) or f_file.endswith(t_file) or t_file.endswith(f_file)
                
                if path_match:
                    # Line matching: allow small margin, or skip line check if line is None (e.g., uncalled)
                    line_match = (f_line is None) or (f_line is not None and t_line is not None and abs(f_line - t_line) <= 2)
                    if line_match:
                        # Type matching: 
                        # "method" in truth might be reported as "function" by some tools
                        type_match = (f_type == t_type) or \
                                     (t_type == "method" and f_type == "function") or \
                                     (t_type == "function" and f_type == "method")
                        
                        if type_match:
                            # Name matching: compare simple names (both sides)
                            f_simple = f_name.split(".")[-1] if f_name else ""
                            t_simple = t_name.split(".")[-1] if t_name else ""
                            if f_simple == t_simple or f_name == t_name:
                                match = t_item
                                break
            
            if match:
                stats["overall"]["TP"] += 1
                matched_findings.add(f_item)
                truth_remaining.remove(match)
                
                # Update specific type stats (based on Truth type)
                t_type = match[2]
                if t_type in stats:
                    stats[t_type]["TP"] += 1
            else:
                stats["overall"]["FP"] += 1
                if stat_type in stats:
                    stats[stat_type]["FP"] += 1

        # Calculate FN (remaining truth items)
        stats["overall"]["FN"] = len(truth_remaining)
        for t_item in truth_remaining:
            t_type = t_item[2]
            if t_type in stats:
                stats[t_type]["FN"] += 1

        # Calculate metrics for all
        results = {}
        for key, s in stats.items():
            tp = s["TP"]
            fp = s["FP"]
            fn = s["FN"]
            
            precision = tp / (tp + fp) if (tp + fp) > 0 else 0
            recall = tp / (tp + fn) if (tp + fn) > 0 else 0
            f1 = 2 * (precision * recall) / (precision + recall) if (precision + recall) > 0 else 0
            
            results[key] = {
                "TP": tp,
                "FP": fp,
                "FN": fn,
                "Precision": precision,
                "Recall": recall,
                "F1": f1
            }
            
        return results

def main():
    print("CytoScnPy Benchmark & Verification Utility")
    print("==========================================")
    
    if not psutil:
        print("[!] 'psutil' module not found. Memory benchmarking will be inaccurate (0 MB).")
        print("    Install with: pip install psutil")

    # Determine paths relative to this script
    script_dir = Path(__file__).parent.resolve()
    project_root = script_dir.parent.resolve()
    
    # Parse CLI Arguments
    import argparse
    parser = argparse.ArgumentParser(description="CytoScnPy Benchmark & Verification Utility")
    parser.add_argument("-l", "--list", action="store_true", help="List available tools and exit")
    parser.add_argument("-c", "--check", action="store_true", help="Check tool availability and exit")
    parser.add_argument("-i", "--include", nargs="+", help="Run only specific tools (substring match, case-insensitive)")
    parser.add_argument("-e", "--exclude", nargs="+", help="Exclude specific tools (substring match, case-insensitive)")
    parser.add_argument("--save-json", help="Save benchmark results to a JSON file")
    parser.add_argument("--compare-json", help="Compare current results against a baseline JSON file")
    parser.add_argument("--threshold", type=float, default=0.10, help="Regression threshold ratio (default: 0.10 = 10%%)")
    args = parser.parse_args()

    # Define tools to run
    # We run on the examples directory which contains multiple subdirectories with ground truth
    target_dir = script_dir / "examples"
    ground_truth_path = target_dir # Pass directory to load recursively
    
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
    python_path_entries = []

    # 1. CytoScnPy Python Wrapper
    python_src = project_root / "python"
    if python_src.exists():
        python_path_entries.append(str(python_src))
        
        # Try to copy the built extension to the python package for it to work
        # Look for cytoscnpy.dll / .so in target/release
        ext_src = project_root / "target" / "release" / "cytoscnpy.dll"
        if not ext_src.exists():
             ext_src = project_root / "cytoscnpy" / "target" / "release" / "cytoscnpy.dll"
        
        if ext_src.exists():
            ext_dest = python_src / "cytoscnpy" / "cytoscnpy.pyd"
            try:
                # print(f"[+] Copying extension from {ext_src} to {ext_dest}")
                shutil.copy2(ext_src, ext_dest)
            except Exception as e:
                # print(f"[-] Failed to copy extension: {e}")
                pass

    # 2. Skylos
    skylos_src = project_root / "other_library" / "skylos"
    if skylos_src.exists():
        python_path_entries.append(str(skylos_src))

    if python_path_entries:
        env["PYTHONPATH"] = os.pathsep.join(python_path_entries) + os.pathsep + env.get("PYTHONPATH", "")

    # Rust Binary Path
    # Try project_root/target/release first (workspace root)
    rust_bin = project_root / "target" / "release" / "cytoscnpy-bin"
    if not rust_bin.exists() and not rust_bin.with_suffix(".exe").exists():
        # Fallback to cytoscnpy/target/release
        rust_bin = project_root / "cytoscnpy" / "target" / "release" / "cytoscnpy-bin"
    
    if sys.platform == "win32":
        rust_bin = rust_bin.with_suffix(".exe")

    # Convert paths to strings for commands
    target_dir_str = str(target_dir)
    rust_bin_str = str(rust_bin)

    all_tools = [
        {
            "name": "CytoScnPy (Rust)",
            "command": [rust_bin_str, target_dir_str, "--json"]
        },
        {
            "name": "CytoScnPy (Python)",
            "command": [sys.executable, "-m", "cytoscnpy.cli", target_dir_str, "--json"],
            "env": env
        },
        {
            "name": "Skylos",
            # Use skylos executable from venv with full path
            "command": [
                (os.path.join(os.path.dirname(sys.executable), "skylos") if os.name != "nt" else os.path.join(os.path.dirname(sys.executable), "skylos.exe")),
                target_dir_str,
                "--json",
                "--confidence", "0"
            ],
            "env": env
        },
        {
            "name": "Vulture (0%)",
            "command": [sys.executable, "-m", "vulture", target_dir_str, "--min-confidence", "0"]
        },
        {
            "name": "Vulture (60%)",
            "command": [sys.executable, "-m", "vulture", target_dir_str, "--min-confidence", "60"]
        },
        {
            "name": "Flake8",
            "command": [sys.executable, "-m", "flake8", target_dir_str]
        },
        {
            "name": "Pylint",
            "command": [sys.executable, "-m", "pylint", target_dir_str, "--output-format=json", "-j", "4"]
        },
        {
            "name": "Ruff",
            "command": [sys.executable, "-m", "ruff", "check", target_dir_str, "--output-format=json"]
        },
        {
            "name": "uncalled",
            "command": [sys.executable, "-m", "uncalled", target_dir_str]
        },
        {
            "name": "dead",
            # dead uses --files regex, not positional path. It runs from CWD.
            "command": f'cd "{target_dir_str}" && "{sys.executable}" -m dead --files ".*\\.py$"'
        },
        {
            "name": "deadcode",
            # deadcode doesn't support 'python -m deadcode', use executable directly
            # Use --no-color to avoid ANSI codes breaking parsing
            "command": [
                (os.path.join(os.path.dirname(sys.executable), "deadcode") if os.name != "nt" else os.path.join(os.path.dirname(sys.executable), "deadcode.exe")),
                target_dir_str,
                "--no-color"
            ]
        }
    ]

    # Handle --list
    if args.list:
        print("Available tools:")
        for tool in all_tools:
            print(f"  - {tool['name']}")
        return
    
    # Handle --check
    if args.check:
        check_tool_availability(all_tools, env)
        return

    # Filter Tools
    tools_to_run = []
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

    # Build Rust project ONLY if we are running CytoScnPy (Rust)
    run_rust_build = any("CytoScnPy (Rust)" in t["name"] for t in tools_to_run)
    
    if run_rust_build:
        print("\n[+] Building Rust project...")
        cargo_toml = project_root / "cytoscnpy" / "Cargo.toml"
        if not cargo_toml.exists():
            print(f"[-] Cargo.toml not found at {cargo_toml}")
            return

        build_cmd = f"cargo build --release --manifest-path \"{cargo_toml}\""
        subprocess.run(build_cmd, shell=True, check=True)
        print("[+] Rust build successful.")
        
        # Check binary again after build
        if not rust_bin.exists():
             print(f"[-] Rust binary still not found at {rust_bin} after build.")

    print(f"\n[+] Loading Ground Truth recursively from {ground_truth_path}...")
    verifier = Verification(str(ground_truth_path))
    
    results = []
    verification_results = []
    
    print(f"\n[+] Running {len(tools_to_run)} tools...")
    
    for tool in tools_to_run:
        if tool["command"]:
            res = run_benchmark_tool(tool["name"], tool["command"], env=tool.get("env"))
            if res:
                results.append(res)
                # Verify
                # Use clean stdout if available to avoid stderr pollution (e.g. logging/errors mixed with JSON)
                output_to_parse = res.get("stdout") if res.get("stdout") is not None else res["output"]
                v_res = verifier.compare(tool["name"], output_to_parse)
                v_res["Tool"] = tool["name"]
                verification_results.append(v_res)
        else:
            print(f"\n[-] Skipping {tool['name']} (not found)")

    # Print Benchmark Results
    print("\n[=] Benchmark Results")
    print(f"{'Tool':<20} | {'Time (s)':<10} | {'Mem (MB)':<10} | {'Issues (Est)':<12}")
    print("-" * 60)
    
    for res in results:
        print(f"{res['name']:<20} | {res['time']:<10.3f} | {res['memory_mb']:<10.2f} | {res['issues']:<12}")
    
    print("-" * 60)

    # Print Verification Results
    print("\n[=] Verification Results (Ground Truth Comparison)")
    
    # Define types to print
    types_to_print = ["overall", "class", "function", "import", "method", "variable"]
    
    for type_key in types_to_print:
        print(f"\n--- {type_key.capitalize()} Detection ---")
        print(f"{'Tool':<20} | {'TP':<5} | {'FP':<5} | {'FN':<5} | {'Precision':<10} | {'Recall':<10} | {'F1 Score':<10}")
        print("-" * 80)
        
        for v in verification_results:
            if type_key in v:
                stats = v[type_key]
                print(f"{v['Tool']:<20} | {stats['TP']:<5} | {stats['FP']:<5} | {stats['FN']:<5} | {stats['Precision']:<10.4f} | {stats['Recall']:<10.4f} | {stats['F1']:<10.4f}")
        print("-" * 80)

    # Compile Final JSON Report
    final_report = {
        "timestamp": time.time(),
        "platform": sys.platform,
        "results": []
    }

    for res in results:
        # Find corresponding verification result
        v_res = next((v for v in verification_results if v["Tool"] == res["name"]), None)
        
        entry = {
            "name": res["name"],
            "time": res["time"],
            "memory_mb": res["memory_mb"],
            "issues": res["issues"],
            "f1_score": v_res["overall"]["F1"] if v_res else 0.0,  # Use overall F1
            "stats": v_res if v_res else {}
        }
        final_report["results"].append(entry)

    # Save JSON if requested
    if args.save_json:
        try:
            with open(args.save_json, "w") as f:
                json.dump(final_report, f, indent=2)
            print(f"\n[+] Results saved to {args.save_json}")
        except Exception as e:
            print(f"[-] Failed to save JSON results: {e}")

    # Compare against baseline if requested
    if args.compare_json:
        print(f"\n[+] Comparing against baseline: {args.compare_json}")
        try:
            with open(args.compare_json, "r") as f:
                baseline = json.load(f)
            
            if "platform" in baseline and baseline["platform"] != sys.platform:
                 print(f"[!] WARNING: Baseline platform ({baseline['platform']}) does not match current system ({sys.platform}). Performance comparison may be inaccurate.")

            cytoscnpy_regressions = []
            other_regressions = []            
            for current in final_report["results"]:
                # specific tool matching
                base = next((b for b in baseline["results"] if b["name"] == current["name"]), None)
                if not base:
                    print(f"    [?] New tool found (no baseline): {current['name']}")
                    continue
                
                # Determine if this is CytoScnPy or a comparison tool
                is_cytoscnpy = "CytoScnPy" in current['name']
                # Check Time
                time_diff = current["time"] - base["time"]
                time_ratio = time_diff / base["time"] if base["time"] > 0 else 0
                if time_ratio > args.threshold:
                    # Ignore small time increases (< 1.0s) to avoid noise
                    if time_diff > 1.0:
                        regression_msg = f"{current['name']} Time: {base['time']:.3f}s -> {current['time']:.3f}s (+{time_ratio*100:.1f}%)"
                        if is_cytoscnpy:
                            cytoscnpy_regressions.append(regression_msg)
                        else:
                            other_regressions.append(regression_msg)

                # Check Memory
                mem_diff = current["memory_mb"] - base["memory_mb"]
                mem_ratio = mem_diff / base["memory_mb"] if base["memory_mb"] > 0 else 0
                if mem_ratio > args.threshold:
                    # Optional: Ignore small memory increases (< 5MB)
                    if mem_diff > 5.0:
                        regression_msg = f"{current['name']} Memory: {base['memory_mb']:.1f}MB -> {current['memory_mb']:.1f}MB (+{mem_ratio*100:.1f}%)"
                        if is_cytoscnpy:
                            cytoscnpy_regressions.append(regression_msg)
                        else:
                            other_regressions.append(regression_msg)

                # Check F1 Score (Regression if strictly lower, handling float precision)
                f1_diff = base["f1_score"] - current["f1_score"]
                if f1_diff > 0.001: # Tolerance for float comparison
                    regression_msg = f"{current['name']} F1 Score: {base['f1_score']:.4f} -> {current['f1_score']:.4f} (-{f1_diff:.4f})"
                    if is_cytoscnpy:
                        cytoscnpy_regressions.append(regression_msg)
                    else:
                        other_regressions.append(regression_msg)

                # Check Precision (Regression if drops more than 0.01)
                if "precision" in base and "precision" in current:
                    prec_diff = base["precision"] - current["precision"]
                    if prec_diff > 0.01:
                        regression_msg = f"{current['name']} Precision: {base['precision']:.4f} -> {current['precision']:.4f} (-{prec_diff:.4f})"
                        if is_cytoscnpy:
                            cytoscnpy_regressions.append(regression_msg)
                        else:
                            other_regressions.append(regression_msg)

                # Check Recall (Regression if drops more than 0.01)
                if "recall" in base and "recall" in current:
                    recall_diff = base["recall"] - current["recall"]
                    if recall_diff > 0.01:
                        regression_msg = f"{current['name']} Recall: {base['recall']:.4f} -> {current['recall']:.4f} (-{recall_diff:.4f})"
                        if is_cytoscnpy:
                            cytoscnpy_regressions.append(regression_msg)
                        else:
                            other_regressions.append(regression_msg)
            
            # Report comparison tool regressions as warnings (informational, non-blocking)
            if other_regressions:
                print("\n[!] WARNING: Comparison tool regressions detected (informational only):")
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
        except Exception as e:
            print(f"[-] Error comparing baseline: {e}")
            sys.exit(1)

if __name__ == "__main__":
    main()
