from __future__ import annotations

from collections.abc import Callable
from pathlib import Path

from .ground_truth import GroundTruth
from .json_parsers import JsonParsers
from .models import Finding, MetricResults
from .text_parsers import TextParsers
from .values import normalize_path


class Verification(GroundTruth, JsonParsers, TextParsers):
    """Verify tool findings against ground truth using the existing matching rules."""

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
    ) -> Finding | None:
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
    def _matches_line(f_line: int | None, t_line: int | None) -> bool:
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
