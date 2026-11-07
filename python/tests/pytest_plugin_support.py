"""Shared fakes for the pytest plugin tests: canned scan JSON and a mock binary."""

import json
import textwrap

EMPTY_JSON = json.dumps(
    {
        "unused_functions": [],
        "unused_methods": [],
        "unused_classes": [],
        "unused_imports": [],
        "unused_variables": [],
        "unused_parameters": [],
        "unused_dependencies": [],
        "missing_dependencies": [],
        "secrets": [],
        "danger": [],
        "quality": [],
        "taint_findings": [],
        "parse_errors": [],
        "clones": [],
        "file_metrics": [],
        "analysis_summary": {
            "total_files": 1,
            "secrets_count": 0,
            "danger_count": 0,
            "quality_count": 0,
            "taint_count": 0,
        },
    }
)


def make_json(*, unused_functions=None, secrets=None, quality=None, total_files=1):
    return json.dumps(
        {
            "unused_functions": unused_functions or [],
            "unused_methods": [],
            "unused_classes": [],
            "unused_imports": [],
            "unused_variables": [],
            "unused_parameters": [],
            "unused_dependencies": [],
            "missing_dependencies": [],
            "secrets": secrets or [],
            "danger": [],
            "quality": quality or [],
            "taint_findings": [],
            "parse_errors": [],
            "clones": [],
            "file_metrics": [],
            "analysis_summary": {
                "total_files": total_files,
                "secrets_count": len(secrets or []),
                "danger_count": 0,
                "quality_count": len(quality or []),
                "taint_count": 0,
            },
        }
    )


def def_item(name, file="foo.py", line=1):
    return {"name": name, "file": file, "line": line}


def finding_item(message, file="foo.py", line=1, severity="warning"):
    return {
        "rule_id": "CSP-Q001",
        "category": "quality",
        "severity": severity,
        "message": message,
        "file": file,
        "line": line,
        "col": 0,
    }


def inject_mock(pytester, *, returncode=0, stdout=EMPTY_JSON, stderr=""):
    """Patch plugin subprocess.run in pytester in-process session."""
    pytester.makeconftest(
        textwrap.dedent(
            f"""
            import cytoscnpy.pytest_plugin as _pm

            class _R:
                returncode = {returncode}
                stdout = {stdout!r}
                stderr = {stderr!r}

            _pm.subprocess = type("_M", (), {{"run": staticmethod(lambda *a, **kw: _R())}})()
            """
        )
    )
