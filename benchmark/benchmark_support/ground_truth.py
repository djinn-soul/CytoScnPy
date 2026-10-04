from __future__ import annotations

import json
from collections.abc import Iterable
from pathlib import Path
from typing import cast

from .models import Finding
from .values import _as_int, _as_str, as_dict_list, normalize_path


class GroundTruth:
    """Load covered files and expected findings without changing their order."""

    def __init__(self, ground_truth_path: str | Path) -> None:
        """Initialize verification with ground truth data."""
        super().__init__()
        self.covered_files: set[str] = set()
        self.ground_truth: set[Finding] = self.load_ground_truth(ground_truth_path)

    _as_dict_list = staticmethod(as_dict_list)

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

    def load_ground_truth(self, path: str | Path) -> set[Finding]:
        """Load ground truth assertions from file."""
        path_obj = Path(path)
        truth_set: set[Finding] = set()
        for p in self._ground_truth_files(path_obj):
            truth_set |= self._load_ground_truth_file(p)
        return truth_set
