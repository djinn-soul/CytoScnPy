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
    """Audit the managed application and benchmark requirement sets."""
    repo_root = Path(__file__).parent.parent
    ignore_file = repo_root / ".pip-audit-ignore"
    requirement_sets = (
        (repo_root / "requirements.txt", repo_root / "requirements-dev.txt"),
        (repo_root / "benchmark" / "requirements.txt",),
    )

    for requirements in requirement_sets:
        cmd = ["uv", "run", "pip-audit"]
        for requirement_file in requirements:
            cmd += ["-r", str(requirement_file)]
        for vuln_id in load_ignores(ignore_file):
            cmd += ["--ignore-vuln", vuln_id]
        cmd.extend(sys.argv[1:])

        result = subprocess.run(cmd)
        if result.returncode != 0:
            return result.returncode

    return 0


if __name__ == "__main__":
    sys.exit(main())
