from __future__ import annotations

import sys
from pathlib import Path
from typing import cast


def warn(message: str) -> None:
    """Write a diagnostic to stderr so it never mixes with report output."""
    sys.stderr.write(f"{message}\n")


def normalize_path(p: str) -> str:
    """Normalize path separator to forward slashes."""
    return Path(p.replace("\\", "/")).as_posix().strip("/")


def _as_str(value: object) -> str:
    return value if isinstance(value, str) else ""


def _as_int(value: object) -> int | None:
    if isinstance(value, bool):
        return None
    return value if isinstance(value, int) else None


def _as_float(value: object) -> float | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, int | float):
        return float(value)
    return None


def as_dict_list(value: object) -> list[dict[str, object]]:
    if not isinstance(value, list):
        return []
    raw_list = cast(list[object], value)
    return [
        cast(dict[str, object], item) for item in raw_list if isinstance(item, dict)
    ]
