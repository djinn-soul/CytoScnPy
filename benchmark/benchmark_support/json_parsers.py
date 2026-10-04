from __future__ import annotations

import json
from collections.abc import Mapping
from typing import cast

from .models import Finding
from .values import _as_int, _as_str, as_dict_list, normalize_path


class JsonParsers:
    """Parse the JSON formats emitted by benchmark tools."""

    @staticmethod
    def _parse_cytoscnpy_output(output: str) -> set[Finding]:
        findings: set[Finding] = set()
        try:
            data = cast(object, json.loads(output))
            if not isinstance(data, dict):
                return findings
            data_dict = cast(dict[str, object], data)
            key_to_fallback_type = {
                "unused_functions": "function",
                "unused_methods": "method",
                "unused_imports": "import",
                "unused_classes": "class",
                "unused_variables": "variable",
                "unused_parameters": "variable",
            }
            for key, fallback_type in key_to_fallback_type.items():
                items = data_dict.get(key)
                if not isinstance(items, list):
                    continue
                items_list = cast(list[object], items)
                for item in items_list:
                    if not isinstance(item, dict):
                        continue
                    item_dict = cast(dict[str, object], item)
                    fpath = normalize_path(_as_str(item_dict.get("file")))
                    simple_name = _as_str(item_dict.get("simple_name"))
                    name = _as_str(item_dict.get("name"))
                    item_name = simple_name or (name.split(".")[-1] if name else "")
                    type_name = _as_str(item_dict.get("def_type")) or fallback_type
                    if type_name == "parameter":
                        type_name = "variable"
                    findings.add(
                        (fpath, _as_int(item_dict.get("line")), type_name, item_name)
                    )
        except json.JSONDecodeError as e:
            print(f"[-] JSON Decode Error for CytoScnPy: {e}")
            print(f"    Output start: {output[:100]}")
        return findings

    @staticmethod
    def _parse_skylos_output(output: str) -> set[Finding]:
        try:
            data = cast(object, json.loads(output))
        except json.JSONDecodeError as e:
            print(f"[-] JSON Decode Error for Skylos: {e}")
            print(f"    Output start: {output[:200]}")
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
        except json.JSONDecodeError:
            # Preserve the existing empty findings result for invalid JSON.
            pass
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
            if not isinstance(data, list):
                return findings
            data_list = cast(list[object], data)
            for item in data_list:
                if not isinstance(item, dict):
                    continue
                item_dict = cast(dict[str, object], item)
                code = _as_str(item_dict.get("code"))
                fpath = normalize_path(_as_str(item_dict.get("filename")))
                location = item_dict.get("location")
                lineno = None
                if isinstance(location, dict):
                    location_dict = cast(dict[str, object], location)
                    lineno = _as_int(location_dict.get("row"))
                if code == "F401":
                    msg = _as_str(item_dict.get("message"))
                    if "`" in msg:
                        obj_name = msg.split("`")[1]
                        findings.add((fpath, lineno, "import", obj_name))
                elif code and (
                    code == "F841" or code.startswith("ARG") or code == "B007"
                ):
                    msg = _as_str(item_dict.get("message"))
                    if "`" in msg:
                        obj_name = msg.split("`")[1]
                        findings.add((fpath, lineno, "variable", obj_name))
        except json.JSONDecodeError:
            # Preserve the existing empty findings result for invalid JSON.
            pass
        return findings
