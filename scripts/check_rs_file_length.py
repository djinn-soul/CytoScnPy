"""Fail when a Rust source file under a crate's ``src/`` exceeds the line limit.

Integration tests under ``tests/`` are exempt (legacy cleanup is deferred).
Run from the repository root: ``python scripts/check_rs_file_length.py``.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

MAX_LINES = 300
SOURCE_GLOBS = (
    "cytoscnpy/src/**/*.rs",
    "cytoscnpy-cli/src/**/*.rs",
    "cytoscnpy-mcp/src/**/*.rs",
)


def tracked_sources() -> list[Path]:
    """Return git-tracked Rust sources under each crate's ``src`` directory."""
    output = subprocess.run(
        ["git", "ls-files", "--", *SOURCE_GLOBS],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return [Path(line) for line in output.splitlines() if line]


def line_count(path: Path) -> int:
    """Count lines the same way ``wc -l`` does for newline-terminated files."""
    with path.open("rb") as handle:
        return sum(1 for _ in handle)


def main() -> int:
    """Report every oversized file and return a failing status if any exist."""
    oversized = [
        (path, count)
        for path in tracked_sources()
        if (count := line_count(path)) > MAX_LINES
    ]
    for path, count in sorted(oversized, key=lambda item: -item[1]):
        print(f"{path}: {count} lines (max {MAX_LINES})")
    if oversized:
        print(
            f"{len(oversized)} Rust source file(s) exceed {MAX_LINES} lines; split them into modules."
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
