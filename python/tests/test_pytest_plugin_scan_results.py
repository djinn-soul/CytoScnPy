"""Pytest plugin tests for scan paths, exit codes, error output, and multi-file runs.

Uses pytester (in-process mode): each test runs a nested pytest session.
Groups D-G of the plugin suite; A-C live in test_pytest_plugin.py.
"""

from pytest_plugin_support import def_item, inject_mock, make_json

pytest_plugins = ["pytester"]


# ---------------------------------------------------------------------------
# D. Paths
# ---------------------------------------------------------------------------


def test_default_path_collects_py_files(pytester):
    """Default path . collects .py files in rootdir."""
    inject_mock(pytester)
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy", "-v")
    result.stdout.fnmatch_lines(["*::cytoscnpy*"])


def test_non_py_files_skipped(pytester):
    """Non-.py files do not get a cytoscnpy item."""
    inject_mock(pytester)
    pytester.makepyfile("def test_ok(): pass\n")
    pytester.makefile(".txt", myfile="hello\n")
    result = pytester.runpytest("--cytoscnpy", "-v")
    assert "myfile.txt::cytoscnpy" not in result.stdout.str()


def test_cytoscnpy_path_excludes_outside_files(pytester):
    """Files outside cytoscnpy_path do not get collected."""
    pytester.makeini("[pytest]\ncytoscnpy = true\ncytoscnpy_path = subdir\n")
    inject_mock(pytester)
    pytester.makepyfile(outside="def test_ok(): pass\n")
    pytester.mkdir("subdir")
    result = pytester.runpytest()
    assert "outside::cytoscnpy" not in result.stdout.str()


# ---------------------------------------------------------------------------
# E. Exit codes
# ---------------------------------------------------------------------------


def test_exit_0_when_no_findings(pytester):
    """Session exits 0 when there are no findings and tests pass."""
    inject_mock(pytester, returncode=0, stdout=make_json())
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    assert result.ret == 0


def test_exit_nonzero_when_findings_exist(pytester):
    """Session exits non-zero when a file has findings (item FAILED)."""
    pytester.makepyfile(test_src="def test_ok(): pass\n")
    inject_mock(
        pytester,
        stdout=make_json(unused_functions=[def_item("dead", file="test_src.py")]),
    )
    result = pytester.runpytest("--cytoscnpy")
    assert result.ret != 0


def test_existing_test_failure_preserved(pytester):
    """A failing test still fails even when cytoscnpy has no findings."""
    inject_mock(pytester, returncode=0, stdout=make_json())
    pytester.makepyfile("def test_fail(): assert False\n")
    result = pytester.runpytest("--cytoscnpy")
    assert result.ret != 0


def test_passing_tests_and_clean_cytoscnpy_exits_0(pytester):
    """Passing tests + clean cytoscnpy -> overall exit 0."""
    inject_mock(pytester, returncode=0, stdout=make_json())
    pytester.makepyfile("def test_a(): assert 1\ndef test_b(): assert 2\n")
    result = pytester.runpytest("--cytoscnpy")
    assert result.ret == 0


def test_cytoscnpy_failure_shows_test_results_too(pytester):
    """When cytoscnpy fails, normal test results are still shown."""
    pytester.makepyfile(test_src="def test_ok(): pass\n")
    inject_mock(
        pytester,
        stdout=make_json(unused_functions=[def_item("dead", file="test_src.py")]),
    )
    result = pytester.runpytest("--cytoscnpy")
    assert "passed" in result.stdout.str()


# ---------------------------------------------------------------------------
# F. Error cases
# ---------------------------------------------------------------------------


def test_invalid_json_shows_error(pytester):
    """When cytoscnpy emits invalid JSON the item fails with ERROR message."""
    inject_mock(pytester, returncode=0, stdout="not valid json at all", stderr="")
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    result.stdout.fnmatch_lines(["*ERROR*"])


def test_invalid_json_with_nonzero_exit_fails_session(pytester):
    """Non-JSON output with non-zero exit fails session and shows stderr."""
    inject_mock(
        pytester,
        returncode=2,
        stdout="not valid json at all",
        stderr="fatal: analyzer crashed",
    )
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    result.stdout.fnmatch_lines(["*ERROR*"])
    assert "fatal: analyzer crashed" in result.stdout.str()
    assert result.ret != 0


def test_stderr_used_as_error_fallback(pytester):
    """When stdout is empty/invalid and stderr has content stderr appears in error."""
    inject_mock(
        pytester,
        returncode=2,
        stdout="",
        stderr="fatal: could not read config",
    )
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    result.stdout.fnmatch_lines(["*ERROR*"])
    assert "fatal: could not read config" in result.stdout.str()


def test_empty_output_shows_fallback_error(pytester):
    """Completely empty stdout + stderr shows the fallback error message."""
    inject_mock(pytester, returncode=0, stdout="", stderr="")
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    result.stdout.fnmatch_lines(["*ERROR*"])
    assert "cytoscnpy produced no output" in result.stdout.str()


def test_error_does_not_crash_session(pytester):
    """Even when the plugin errors the pytest session completes and tests run."""
    inject_mock(pytester, returncode=0, stdout="{{broken")
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    assert "passed" in result.stdout.str()


# ---------------------------------------------------------------------------
# G. Multi-file
# ---------------------------------------------------------------------------


def test_each_file_gets_its_own_item(pytester):
    """Multiple .py files each get an independent cytoscnpy item."""
    inject_mock(pytester)
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
    inject_mock(
        pytester,
        stdout=make_json(unused_functions=[def_item("dead_fn", file="dirty.py")]),
    )
    result = pytester.runpytest("--cytoscnpy", "-v")
    output = result.stdout.str()
    assert "dirty" in output and "cytoscnpy" in output
    assert "clean" in output and "cytoscnpy" in output
