from __future__ import annotations

import json
from collections.abc import Mapping
from typing import cast

from .models import Finding
from .values import _as_int, _as_str, as_dict_list, normalize_path, warn

_CYTOSCNPY_FALLBACK_TYPES = {
    "unused_functions": "function",
    "unused_methods": "method",
    "unused_imports": "import",
    "unused_classes": "class",
    "unused_variables": "variable",
    "unused_parameters": "variable",
}


class JsonParsers:
    """Parse the JSON formats emitted by benchmark tools."""

    @staticmethod
    def _parse_cytoscnpy_output(output: str) -> set[Finding]:
        try:
            data = cast(object, json.loads(output))
        except json.JSONDecodeError as e:
            warn(f"[-] JSON Decode Error for CytoScnPy: {e}")
            warn(f"    Output start: {output[:100]}")
            return set()
        if not isinstance(data, dict):
            return set()
        data_dict = cast(dict[str, object], data)
        return {
            JsonParsers._cytoscnpy_finding(item, fallback_type)
            for key, fallback_type in _CYTOSCNPY_FALLBACK_TYPES.items()
            for item in as_dict_list(data_dict.get(key))
        }

    @staticmethod
    def _cytoscnpy_finding(item: dict[str, object], fallback_type: str) -> Finding:
        name = _as_str(item.get("name"))
        item_name = _as_str(item.get("simple_name")) or (
            name.split(".")[-1] if name else ""
        )
        type_name = _as_str(item.get("def_type")) or fallback_type
        if type_name == "parameter":
            type_name = "variable"
        return (
            normalize_path(_as_str(item.get("file"))),
            _as_int(item.get("line")),
            type_name,
            item_name,
        )

    @staticmethod
    def _parse_skylos_output(output: str) -> set[Finding]:
        try:
            data = cast(object, json.loads(output))
        except json.JSONDecodeError as e:
            warn(f"[-] JSON Decode Error for Skylos: {e}")
            warn(f"    Output start: {output[:200]}")
            return set()

        findings: set[Finding] = set()
        for item in JsonParsers._skylos_items(data):
            finding = JsonParsers._skylos_item_to_finding(item)
            if finding is not None:
                findings.add(finding)
        return findings

    @staticmethod
    def _skylos_items(data: object) -> list[dict[str, object]]:
        if isinstance(data, list):
            return as_dict_list(cast(list[object], data))
        if not isinstance(data, dict):
            return []

        data_dict = cast(dict[str, object], data)
        items_list: list[dict[str, object]] = []
        for key in (
            "unused_functions",
            "unused_imports",
            "unused_classes",
            "unused_variables",
            "items",
        ):
            items_list.extend(as_dict_list(data_dict.get(key)))

        if not items_list:
            items_list.extend(as_dict_list(data_dict.get("results")))

        return items_list

    @staticmethod
    def _skylos_item_to_finding(item: dict[str, object]) -> Finding | None:
        type_map: Mapping[str, str] = {
            "function": "function",
            "class": "class",
            "import": "import",
            "variable": "variable",
            "parameter": "variable",
            "method": "method",
        }

        skylos_type = _as_str(item.get("type")).lower()
        type_name = type_map.get(skylos_type, skylos_type)
        if not type_name:
            return None

        fpath = normalize_path(_as_str(item.get("file")))
        simple_name = _as_str(item.get("simple_name"))
        full_name = _as_str(item.get("name"))
        item_name = simple_name or (full_name.split(".")[-1] if full_name else "")
        if not item_name:
            return None

        lineno = _as_int(item.get("line"))
        return (fpath, lineno, type_name, item_name)

    @staticmethod
    def _parse_pylint_output(output: str) -> set[Finding]:
        findings: set[Finding] = set()
        try:
            data = cast(object, json.loads(output))
            if not isinstance(data, list):
                return findings
            data_list = cast(list[object], data)
            for item in data_list:
                if not isinstance(item, dict):
                    continue
                item_dict = cast(dict[str, object], item)
                symbol = _as_str(item_dict.get("symbol"))
                if symbol == "unused-import":
                    JsonParsers._handle_pylint_unused_import(item_dict, findings)
                elif symbol in {
                    "unused-variable",
                    "unused-argument",
                    "unused-private-member",
                }:
                    JsonParsers._handle_pylint_unused_variable(item_dict, findings)
        except json.JSONDecodeError as e:
            if output.strip():
                warn(f"[-] JSON Decode Error for Pylint: {e}")
        return findings

    @staticmethod
    def _handle_pylint_unused_import(
        item: dict[str, object], findings: set[Finding]
    ) -> None:
        fpath = normalize_path(_as_str(item.get("path")))
        lineno = _as_int(item.get("line"))
        obj_name = _as_str(item.get("obj"))
        if not obj_name:
            msg = _as_str(item.get("message"))
            if "Unused import " in msg:
                obj_name = msg.split("Unused import ")[1].strip()
        findings.add((fpath, lineno, "import", obj_name))

    @staticmethod
    def _handle_pylint_unused_variable(
        item: dict[str, object], findings: set[Finding]
    ) -> None:
        fpath = normalize_path(_as_str(item.get("path")))
        lineno = _as_int(item.get("line"))
        msg = _as_str(item.get("message"))
        obj_name = ""
        if "'" in msg:
            obj_name = msg.split("'")[1]
        elif isinstance(item.get("obj"), str):
            obj_name = _as_str(item.get("obj"))
        if obj_name:
            findings.add((fpath, lineno, "variable", obj_name))

    @staticmethod
    def _parse_ruff_output(output: str) -> set[Finding]:
        findings: set[Finding] = set()
        try:
            data = cast(object, json.loads(output))
        except json.JSONDecodeError as e:
            if output.strip():
                warn(f"[-] JSON Decode Error for Ruff: {e}")
            return findings
        if not isinstance(data, list):
            return findings
        for item in cast(list[object], data):
            if isinstance(item, dict):
                finding = JsonParsers._ruff_finding(cast(dict[str, object], item))
                if finding is not None:
                    findings.add(finding)
        return findings

    @staticmethod
    def _ruff_finding(item_dict: dict[str, object]) -> Finding | None:
        code = _as_str(item_dict.get("code"))
        if code == "F401":
            type_name = "import"
        elif code in {"F841", "B007"} or code.startswith("ARG"):
            type_name = "variable"
        else:
            return None
        msg = _as_str(item_dict.get("message"))
        if "`" not in msg:
            return None
        location = item_dict.get("location")
        lineno = None
        if isinstance(location, dict):
            lineno = _as_int(cast(dict[str, object], location).get("row"))
        fpath = normalize_path(_as_str(item_dict.get("filename")))
        return (fpath, lineno, type_name, msg.split("`")[1])
