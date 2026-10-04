from __future__ import annotations

import re

from .models import Finding
from .values import normalize_path


class TextParsers:
    """Parse line based findings emitted by benchmark tools."""

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
