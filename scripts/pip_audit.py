"""pip-audit wrapper that reads ignore IDs from .pip-audit-ignore."""

import subprocess
import sys
from pathlib import Path


def load_ignores(ignore_file: Path) -> list[str]:
    """Read CVE/GHSA IDs to ignore from the given file, skipping blank lines and comments."""
    if not ignore_file.exists():
        return []
    ids = []
    for line in ignore_file.read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            ids.append(line)
    return ids


def main() -> int:
    """Run pip-audit with ignore list loaded from .pip-audit-ignore."""
    repo_root = Path(__file__).parent.parent
    ignore_file = repo_root / ".pip-audit-ignore"

    cmd = ["uv", "run", "pip-audit"]
    for vuln_id in load_ignores(ignore_file):
        cmd += ["--ignore-vuln", vuln_id]
    # Forward any additional arguments passed to this script
    cmd.extend(sys.argv[1:])

    result = subprocess.run(cmd)
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
