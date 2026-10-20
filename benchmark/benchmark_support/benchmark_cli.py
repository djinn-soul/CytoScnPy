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


def run_main(
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
    args = _parse_args()

    # We run on the examples directory which contains multiple subdirectories with ground truth
    target_dir = script_dir / "examples"
    if not target_dir.exists():
        print(f"[-] Target directory not found: {target_dir}")
        return

    # Prepare tool commands before listing, checking availability, or filtering.
    all_tools, rust_bin = configure_tools(project_root, target_dir)

    if args.list:
        print("Available tools:")
        for tool in all_tools:
            print(f"  - {tool['name']}")
        return

    if args.check:
        _ = check_tool_availability(all_tools)
        return

    tools_to_run = _select_tools(all_tools, args.include, args.exclude)
    if not tools_to_run:
        print("[-] No tools selected to run.")
        return

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
    if any("CytoScnPy (Rust)" in t["name"] for t in tools_to_run):
        if not _build_rust(project_root, rust_bin):
            return

    # Pass the directory so ground truth loads recursively.
    print(f"\n[+] Loading Ground Truth recursively from {target_dir}...")
    verifier = verifier_factory(str(target_dir))
    results, verification_results = _run_tools(
        tools_to_run, run_benchmark_tool, verifier
    )
    report_results(args, results, verification_results)


def _parse_args() -> Args:
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
    return cast(Args, parser.parse_args())


def _select_tools(
    tools: list[ToolConfig],
    include: list[str] | None,
    exclude: list[str] | None,
) -> list[ToolConfig]:
    """Keep tools matching any include pattern and no exclude pattern (case-insensitive)."""

    def matches(name: str, patterns: list[str]) -> bool:
        return any(pattern.lower() in name for pattern in patterns)

    return [
        tool
        for tool in tools
        if not (exclude and matches(tool["name"].lower(), exclude))
        and (not include or matches(tool["name"].lower(), include))
    ]


def _build_rust(project_root: Path, rust_bin: Path) -> bool:
    """Build the Rust CLI in release mode; return False when no Cargo.toml exists."""
    print("\n[+] Building Rust project...")
    cargo_toml = project_root / "Cargo.toml"
    if not cargo_toml.exists():
        # Fallback to sub-directory if not in root
        cargo_toml = project_root / "cytoscnpy" / "Cargo.toml"

    if not cargo_toml.exists():
        print(
            f"[-] Cargo.toml not found in {project_root} or {project_root / 'cytoscnpy'}"
        )
        return False

    build_cmd = ["cargo", "build", "--release", "-p", "cytoscnpy"]
    if cargo_toml.parent != project_root:
        build_cmd.extend(["--manifest-path", str(cargo_toml)])
    _ = subprocess.run(build_cmd, shell=False, check=True)
    print("[+] Rust build successful.")

    if not rust_bin.exists():
        print(f"[-] Rust binary still not found at {rust_bin} after build.")
    return True


def _run_tools(
    tools: list[ToolConfig],
    run_benchmark_tool: Callable[..., BenchmarkResult | None],
    verifier: Verification,
) -> tuple[list[BenchmarkResult], list[VerificationResult]]:
    results: list[BenchmarkResult] = []
    verification_results: list[VerificationResult] = []

    print(f"\n[+] Running {len(tools)} tools...")

    for tool in tools:
        if not tool["command"]:
            print(f"\n[-] Skipping {tool['name']} (not found)")
            continue
        cwd_value = tool.get("cwd")
        env_value = tool.get("env")
        res = run_benchmark_tool(
            tool["name"],
            tool["command"],
            cwd=cwd_value if isinstance(cwd_value, str) else None,
            env=env_value if isinstance(env_value, Mapping) else None,
        )
        if not res:
            continue
        results.append(res)
        # Prefer clean stdout so stderr logging mixed into the output does not break JSON parsing.
        metrics = verifier.compare(tool["name"], res["stdout"] or res["output"])
        verification_entry: VerificationResult = dict(metrics)
        verification_entry["Tool"] = tool["name"]
        verification_results.append(verification_entry)

    return results, verification_results
