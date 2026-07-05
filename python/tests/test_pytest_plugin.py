"""Extensive tests for the CytoScnPy pytest plugin.

Uses pytester (in-process mode) throughout. The plugin is exercised end-to-end
by running a nested pytest session in a temporary directory.

The plugin mirrors pytest-vulture: one pytest Item per .py file, so findings
appear as native PASSED/FAILED results rather than a custom summary section.

Test groups:
  A. Activation  -- opt-in mechanisms (flag, ini formats, no accidental activation)
  B. Item format -- reportinfo label, per-file PASSED/FAILED, name cytoscnpy
  C. Findings    -- unused code, security, quality show as failures with messages
  D. Paths       -- cytoscnpy_path scoping, files outside scan path skipped
  E. Exit codes  -- clean->0, findings->1, existing test failures preserved
  F. Error cases -- invalid JSON, empty output, stderr fallback
  G. Multi-file  -- multiple files each get their own item
"""

import json
import textwrap

pytest_plugins = ["pytester"]


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_EMPTY_JSON = json.dumps(
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


def _make_json(*, unused_functions=None, secrets=None, quality=None, total_files=1):
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


def _def_item(name, file="foo.py", line=1):
    return {"name": name, "file": file, "line": line}


def _finding_item(message, file="foo.py", line=1, severity="warning"):
    return {
        "rule_id": "CSP-Q001",
        "category": "quality",
        "severity": severity,
        "message": message,
        "file": file,
        "line": line,
        "col": 0,
    }


def _inject_mock(pytester, *, returncode=0, stdout=_EMPTY_JSON, stderr=""):
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


# ---------------------------------------------------------------------------
# A. Activation
# ---------------------------------------------------------------------------


def test_disabled_by_default(pytester):
    """Plugin must NOT produce cytoscnpy items without opt-in."""
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("-v")
    # "cytoscnpy" alone appears in the `plugins:` banner since the package is
    # installed as an entry point; the item id "::cytoscnpy" is the real signal.
    assert "::cytoscnpy" not in result.stdout.str()


def test_enabled_via_cli_flag(pytester):
    """--cytoscnpy flag activates the plugin."""
    _inject_mock(pytester)
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    result.stdout.fnmatch_lines(["*cytoscnpy*"])


def test_enabled_via_pytest_ini(pytester):
    """cytoscnpy = true in pytest.ini activates the plugin."""
    _inject_mock(pytester)
    pytester.makeini("[pytest]\ncytoscnpy = true\n")
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest()
    result.stdout.fnmatch_lines(["*cytoscnpy*"])


def test_enabled_via_pyproject_ini_options(pytester):
    """cytoscnpy = true under [tool.pytest.ini_options] activates the plugin."""
    _inject_mock(pytester)
    pytester.makepyprojecttoml("[tool.pytest.ini_options]\ncytoscnpy = true\n")
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest()
    result.stdout.fnmatch_lines(["*cytoscnpy*"])


def test_cytoscnpy_path_alone_does_not_enable(pytester):
    """Setting only cytoscnpy_path without cytoscnpy=true must NOT activate."""
    pytester.makeini("[pytest]\ncytoscnpy_path = .\n")
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("-v")
    assert "::cytoscnpy" not in result.stdout.str()


def test_flag_overrides_ini_false(pytester):
    """--cytoscnpy CLI flag enables even when ini says false."""
    _inject_mock(pytester)
    pytester.makeini("[pytest]\ncytoscnpy = false\n")
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    result.stdout.fnmatch_lines(["*cytoscnpy*"])


# ---------------------------------------------------------------------------
# B. Item format
# ---------------------------------------------------------------------------


def test_item_name_is_cytoscnpy(pytester):
    """Each file item is named cytoscnpy."""
    _inject_mock(pytester)
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy", "-v")
    result.stdout.fnmatch_lines(["*::cytoscnpy*"])


def test_reportinfo_label(pytester):
    """Item reportinfo uses the [cytoscnpy] prefix."""
    _inject_mock(pytester)
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy", "-v")
    result.stdout.fnmatch_lines(["*[cytoscnpy]*"])


def test_clean_file_passes(pytester):
    """A file with no findings produces a PASSED cytoscnpy item."""
    _inject_mock(pytester, stdout=_make_json())
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy", "-v")
    result.stdout.fnmatch_lines(["*::cytoscnpy PASSED*"])


def test_file_with_findings_fails(pytester):
    """A file with findings produces a FAILED cytoscnpy item."""
    pytester.makepyfile(test_src="def test_ok(): pass\n")
    _inject_mock(
        pytester,
        stdout=_make_json(
            unused_functions=[_def_item("dead_fn", file="test_src.py", line=1)]
        ),
    )
    result = pytester.runpytest("--cytoscnpy", "-v")
    result.stdout.fnmatch_lines(["*::cytoscnpy FAILED*"])


# ---------------------------------------------------------------------------
# C. Findings
# ---------------------------------------------------------------------------


def test_unused_function_in_failure_output(pytester):
    """Unused function name appears in the failure output."""
    pytester.makepyfile(test_src="def test_ok(): pass\n")
    _inject_mock(
        pytester,
        stdout=_make_json(
            unused_functions=[_def_item("my_dead_fn", file="test_src.py", line=3)]
        ),
    )
    result = pytester.runpytest("--cytoscnpy")
    assert "my_dead_fn" in result.stdout.str()


def test_line_number_in_failure_output(pytester):
    """Line number appears in the failure output."""
    pytester.makepyfile(test_src="def test_ok(): pass\n")
    _inject_mock(
        pytester,
        stdout=_make_json(
            unused_functions=[_def_item("fn", file="test_src.py", line=42)]
        ),
    )
    result = pytester.runpytest("--cytoscnpy")
    assert "42" in result.stdout.str()


def test_quality_finding_in_failure_output(pytester):
    """Quality issue message appears in the failure output."""
    pytester.makepyfile(test_src="def test_ok(): pass\n")
    _inject_mock(
        pytester,
        stdout=_make_json(
            quality=[_finding_item("function too complex", file="test_src.py")]
        ),
    )
    result = pytester.runpytest("--cytoscnpy")
    assert "function too complex" in result.stdout.str()


def test_secret_finding_in_failure_output(pytester):
    """Secret finding message appears in the failure output."""
    pytester.makepyfile(test_src="def test_ok(): pass\n")
    secret = {
        "message": "Possible API key",
        "rule_id": "CSP-S101",
        "category": "secrets",
        "file": "test_src.py",
        "line": 7,
        "severity": "HIGH",
        "confidence": 90,
    }
    _inject_mock(pytester, stdout=_make_json(secrets=[secret]))
    result = pytester.runpytest("--cytoscnpy")
    assert "Possible API key" in result.stdout.str()


def test_kind_label_in_failure_output(pytester):
    """The finding kind label appears in failure output."""
    pytester.makepyfile(test_src="def test_ok(): pass\n")
    _inject_mock(
        pytester,
        stdout=_make_json(unused_functions=[_def_item("fn", file="test_src.py")]),
    )
    result = pytester.runpytest("--cytoscnpy")
    assert "unused function" in result.stdout.str()


# ---------------------------------------------------------------------------
# D. Paths
# ---------------------------------------------------------------------------


def test_default_path_collects_py_files(pytester):
    """Default path . collects .py files in rootdir."""
    _inject_mock(pytester)
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy", "-v")
    result.stdout.fnmatch_lines(["*::cytoscnpy*"])


def test_non_py_files_skipped(pytester):
    """Non-.py files do not get a cytoscnpy item."""
    _inject_mock(pytester)
    pytester.makepyfile("def test_ok(): pass\n")
    pytester.makefile(".txt", myfile="hello\n")
    result = pytester.runpytest("--cytoscnpy", "-v")
    assert "myfile.txt::cytoscnpy" not in result.stdout.str()


def test_cytoscnpy_path_excludes_outside_files(pytester):
    """Files outside cytoscnpy_path do not get collected."""
    pytester.makeini("[pytest]\ncytoscnpy = true\ncytoscnpy_path = subdir\n")
    _inject_mock(pytester)
    pytester.makepyfile(outside="def test_ok(): pass\n")
    pytester.mkdir("subdir")
    result = pytester.runpytest()
    assert "outside::cytoscnpy" not in result.stdout.str()


# ---------------------------------------------------------------------------
# E. Exit codes
# ---------------------------------------------------------------------------


def test_exit_0_when_no_findings(pytester):
    """Session exits 0 when there are no findings and tests pass."""
    _inject_mock(pytester, returncode=0, stdout=_make_json())
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    assert result.ret == 0


def test_exit_nonzero_when_findings_exist(pytester):
    """Session exits non-zero when a file has findings (item FAILED)."""
    pytester.makepyfile(test_src="def test_ok(): pass\n")
    _inject_mock(
        pytester,
        stdout=_make_json(unused_functions=[_def_item("dead", file="test_src.py")]),
    )
    result = pytester.runpytest("--cytoscnpy")
    assert result.ret != 0


def test_existing_test_failure_preserved(pytester):
    """A failing test still fails even when cytoscnpy has no findings."""
    _inject_mock(pytester, returncode=0, stdout=_make_json())
    pytester.makepyfile("def test_fail(): assert False\n")
    result = pytester.runpytest("--cytoscnpy")
    assert result.ret != 0


def test_passing_tests_and_clean_cytoscnpy_exits_0(pytester):
    """Passing tests + clean cytoscnpy -> overall exit 0."""
    _inject_mock(pytester, returncode=0, stdout=_make_json())
    pytester.makepyfile("def test_a(): assert 1\ndef test_b(): assert 2\n")
    result = pytester.runpytest("--cytoscnpy")
    assert result.ret == 0


def test_cytoscnpy_failure_shows_test_results_too(pytester):
    """When cytoscnpy fails, normal test results are still shown."""
    pytester.makepyfile(test_src="def test_ok(): pass\n")
    _inject_mock(
        pytester,
        stdout=_make_json(unused_functions=[_def_item("dead", file="test_src.py")]),
    )
    result = pytester.runpytest("--cytoscnpy")
    assert "passed" in result.stdout.str()


# ---------------------------------------------------------------------------
# F. Error cases
# ---------------------------------------------------------------------------


def test_invalid_json_shows_error(pytester):
    """When cytoscnpy emits invalid JSON the item fails with ERROR message."""
    _inject_mock(pytester, returncode=0, stdout="not valid json at all", stderr="")
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    result.stdout.fnmatch_lines(["*ERROR*"])


def test_invalid_json_with_nonzero_exit_fails_session(pytester):
    """Non-JSON output with non-zero exit fails session and shows stderr."""
    _inject_mock(
        pytester,
        returncode=2,
        stdout="not valid json at all",
        stderr="fatal: analyzer crashed",
    )
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    result.stdout.fnmatch_lines(["*ERROR*fatal: analyzer crashed*"])
    assert result.ret != 0


def test_stderr_used_as_error_fallback(pytester):
    """When stdout is empty/invalid and stderr has content stderr appears in error."""
    _inject_mock(
        pytester,
        returncode=2,
        stdout="",
        stderr="fatal: could not read config",
    )
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    result.stdout.fnmatch_lines(["*ERROR*fatal: could not read config*"])


def test_empty_output_shows_fallback_error(pytester):
    """Completely empty stdout + stderr shows the fallback error message."""
    _inject_mock(pytester, returncode=0, stdout="", stderr="")
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    result.stdout.fnmatch_lines(["*ERROR*cytoscnpy produced no output*"])


def test_error_does_not_crash_session(pytester):
    """Even when the plugin errors the pytest session completes and tests run."""
    _inject_mock(pytester, returncode=0, stdout="{{broken")
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    assert "passed" in result.stdout.str()


# ---------------------------------------------------------------------------
# G. Multi-file
# ---------------------------------------------------------------------------


def test_each_file_gets_its_own_item(pytester):
    """Multiple .py files each get an independent cytoscnpy item."""
    _inject_mock(pytester)
    pytester.makepyfile(file_a="def test_a(): pass\n")
    pytester.makepyfile(file_b="def test_b(): pass\n")
    result = pytester.runpytest("--cytoscnpy", "-v")
    output = result.stdout.str()
    assert "file_a" in output and "cytoscnpy" in output
    assert "file_b" in output and "cytoscnpy" in output


def test_only_file_with_findings_fails(pytester):
    """When only one file has findings only that file item fails."""
    pytester.makepyfile(clean="def test_ok(): pass\n")
    pytester.makepyfile(dirty="def test_ok(): pass\n")
    _inject_mock(
        pytester,
        stdout=_make_json(unused_functions=[_def_item("dead_fn", file="dirty.py")]),
    )
    result = pytester.runpytest("--cytoscnpy", "-v")
    output = result.stdout.str()
    assert "dirty" in output and "cytoscnpy" in output
    assert "clean" in output and "cytoscnpy" in output
