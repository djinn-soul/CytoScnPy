from __future__ import annotations

import subprocess
from collections.abc import Callable, Mapping
from pathlib import Path
from typing import cast

from .configuration import configure_tools
from .models import (
    Args,
    BenchmarkResult,
    ToolCheckResults,
    ToolConfig,
    VerificationResult,
)
from .reports import report_results
from .verification import Verification


def run_main(  # noqa: C901
    script_dir: Path,
    check_tool_availability: Callable[[list[ToolConfig]], ToolCheckResults],
    run_benchmark_tool: Callable[..., BenchmarkResult | None],
    verifier_factory: Callable[[str], Verification],
    *,
    psutil_available: bool,
) -> None:
    """Main entry point."""
    print("CytoScnPy Benchmark & Verification Utility")
    print("==========================================")

    if not psutil_available:
        print(
            "[!] 'psutil' module not found. Memory benchmarking will be inaccurate (0 MB)."
        )
        print("    Install with: pip install psutil")

    # Determine paths relative to this script
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
    args = cast(Args, parser.parse_args())

    # Define tools to run
    # We run on the examples directory which contains multiple subdirectories with ground truth
    target_dir = script_dir / "examples"
    ground_truth_path = target_dir  # Pass directory to load recursively

    if not target_dir.exists():
        print(f"[-] Target directory not found: {target_dir}")
        return

    # Prepare tool commands before listing, checking availability, or filtering.
    all_tools, rust_bin = configure_tools(project_root, target_dir)

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
    verifier = verifier_factory(str(ground_truth_path))

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
                output_to_parse = stdout or res["output"]
                metrics = verifier.compare(tool["name"], output_to_parse)
                verification_entry: VerificationResult = dict(metrics)
                verification_entry["Tool"] = tool["name"]
                verification_results.append(verification_entry)
        else:
            print(f"\n[-] Skipping {tool['name']} (not found)")

    report_results(args, results, verification_results)
