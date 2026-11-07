"""Extensive tests for the CytoScnPy pytest plugin.

Uses pytester (in-process mode) throughout. The plugin is exercised end-to-end
by running a nested pytest session in a temporary directory.

The plugin mirrors pytest-vulture: one pytest Item per .py file, so findings
appear as native PASSED/FAILED results rather than a custom summary section.

Test groups:
  A. Activation  -- opt-in mechanisms (flag, ini formats, no accidental activation)
  B. Item format -- reportinfo label, per-file PASSED/FAILED, name cytoscnpy
  C. Findings    -- unused code, security, quality show as failures with messages

Groups D-G (paths, exit codes, error cases, multi-file) are in
test_pytest_plugin_scan_results.py; shared fakes are in pytest_plugin_support.py.
"""

from pytest_plugin_support import def_item, finding_item, inject_mock, make_json

pytest_plugins = ["pytester"]


# ---------------------------------------------------------------------------
# A. Activation
# ---------------------------------------------------------------------------


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
    inject_mock(pytester)
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    result.stdout.fnmatch_lines(["*cytoscnpy*"])


def test_enabled_via_pytest_ini(pytester):
    """cytoscnpy = true in pytest.ini activates the plugin."""
    inject_mock(pytester)
    pytester.makeini("[pytest]\ncytoscnpy = true\n")
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest()
    result.stdout.fnmatch_lines(["*cytoscnpy*"])


def test_enabled_via_pyproject_ini_options(pytester):
    """cytoscnpy = true under [tool.pytest.ini_options] activates the plugin."""
    inject_mock(pytester)
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
    inject_mock(pytester)
    pytester.makeini("[pytest]\ncytoscnpy = false\n")
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy")
    result.stdout.fnmatch_lines(["*cytoscnpy*"])


# ---------------------------------------------------------------------------
# B. Item format
# ---------------------------------------------------------------------------


def test_item_name_is_cytoscnpy(pytester):
    """Each file item is named cytoscnpy."""
    inject_mock(pytester)
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy", "-v")
    result.stdout.fnmatch_lines(["*::cytoscnpy*"])


def test_reportinfo_label(pytester):
    """Item reportinfo uses the [cytoscnpy] prefix."""
    inject_mock(pytester)
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy", "-v")
    result.stdout.fnmatch_lines(["*[cytoscnpy]*"])


def test_clean_file_passes(pytester):
    """A file with no findings produces a PASSED cytoscnpy item."""
    inject_mock(pytester, stdout=make_json())
    pytester.makepyfile("def test_ok(): pass\n")
    result = pytester.runpytest("--cytoscnpy", "-v")
    result.stdout.fnmatch_lines(["*::cytoscnpy PASSED*"])


def test_file_with_findings_fails(pytester):
    """A file with findings produces a FAILED cytoscnpy item."""
    pytester.makepyfile(test_src="def test_ok(): pass\n")
    inject_mock(
        pytester,
        stdout=make_json(
            unused_functions=[def_item("dead_fn", file="test_src.py", line=1)]
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
    inject_mock(
        pytester,
        stdout=make_json(
            unused_functions=[def_item("my_dead_fn", file="test_src.py", line=3)]
        ),
    )
    result = pytester.runpytest("--cytoscnpy")
    assert "my_dead_fn" in result.stdout.str()


def test_line_number_in_failure_output(pytester):
    """Line number appears in the failure output."""
    pytester.makepyfile(test_src="def test_ok(): pass\n")
    inject_mock(
        pytester,
        stdout=make_json(
            unused_functions=[def_item("fn", file="test_src.py", line=42)]
        ),
    )
    result = pytester.runpytest("--cytoscnpy")
    assert "42" in result.stdout.str()


def test_quality_finding_in_failure_output(pytester):
    """Quality issue message appears in the failure output."""
    pytester.makepyfile(test_src="def test_ok(): pass\n")
    inject_mock(
        pytester,
        stdout=make_json(
            quality=[finding_item("function too complex", file="test_src.py")]
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
    inject_mock(pytester, stdout=make_json(secrets=[secret]))
    result = pytester.runpytest("--cytoscnpy")
    assert "Possible API key" in result.stdout.str()


def test_kind_label_in_failure_output(pytester):
    """The finding kind label appears in failure output."""
    pytester.makepyfile(test_src="def test_ok(): pass\n")
    inject_mock(
        pytester,
        stdout=make_json(unused_functions=[def_item("fn", file="test_src.py")]),
    )
    result = pytester.runpytest("--cytoscnpy")
    assert "unused function" in result.stdout.str()
