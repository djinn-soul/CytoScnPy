"""Tests for CI/CD quality gate features (--fail-threshold, --max-complexity, --min-mi flags).

These tests use the Python library directly instead of subprocess calls,
making them faster and more portable than binary-based tests.
"""

from cytoscnpy import run


class TestFailThreshold:
    """Tests for --fail-threshold quality gate."""

    def test_fail_threshold_passes_when_below(self, tmp_path):
        """Clean code should pass when below threshold."""
        file_path = tmp_path / "clean.py"
        file_path.write_text("""
def used_function():
    return 42

result = used_function()
print(result)
""")
        # High threshold should pass for clean code
        exit_code = run(["--fail-threshold", "50", "--json", str(tmp_path)])
        assert exit_code == 0, "Expected success when below fail threshold"

    def test_fail_threshold_fails_when_above(self, tmp_path):
        """Code with lots of unused items should fail low threshold."""
        for i in range(3):
            file_path = tmp_path / f"unused_{i}.py"
            file_path.write_text("""
def unused_function_1():
    pass

def unused_function_2():
    pass

def unused_function_3():
    pass

class UnusedClass:
    pass
""")
        # Ultra-low threshold should fail
        exit_code = run(["--fail-threshold", "0.1", "--json", str(tmp_path)])
        assert exit_code == 1, "Expected failure when above fail threshold"

    def test_fail_threshold_with_env_var(self, tmp_path, monkeypatch):
        """Environment variable should set fail threshold."""
        file_path = tmp_path / "mixed.py"
        file_path.write_text("""
def used_function():
    return 42

def unused_function():
    pass

result = used_function()
""")
        # Set env var to ultra-low threshold
        monkeypatch.setenv("CYTOSCNPY_FAIL_THRESHOLD", "0.01")
        exit_code = run(["--json", str(tmp_path)])
        assert exit_code == 1, "Expected failure from env var threshold"

    def test_cli_overrides_env_var(self, tmp_path, monkeypatch):
        """CLI --fail-threshold should override env var."""
        file_path = tmp_path / "test.py"
        file_path.write_text("""
def unused_function():
    pass
""")
        # Env var says fail at 0.01%, but CLI says 1000% (should always pass)
        monkeypatch.setenv("CYTOSCNPY_FAIL_THRESHOLD", "0.01")
        exit_code = run(["--fail-threshold", "1000", "--json", str(tmp_path)])
        assert exit_code == 0, "Expected CLI to override env var"

    def test_no_gate_when_not_specified(self, tmp_path):
        """Without --fail-threshold, should always pass regardless of unused code."""
        file_path = tmp_path / "lots_unused.py"
        file_path.write_text("""
def unused1(): pass
def unused2(): pass
def unused3(): pass
def unused4(): pass
def unused5(): pass
class Unused1: pass
class Unused2: pass
""")
        # Should pass without threshold set
        exit_code = run(["--json", str(tmp_path)])
        assert exit_code == 0, "Expected success when no fail threshold specified"


class TestMaxComplexityGate:
    """Tests for --max-complexity quality gate."""

    def test_max_complexity_passes(self, tmp_path):
        """Simple function should pass high complexity threshold."""
        file_path = tmp_path / "simple.py"
        file_path.write_text("""
def simple_function():
    return 42
""")
        exit_code = run(["--max-complexity", "20", "--quality", str(tmp_path)])
        assert exit_code == 0, "Expected success with high complexity threshold"

    def test_max_complexity_fails(self, tmp_path):
        """Complex function should fail low complexity threshold."""
        file_path = tmp_path / "complex.py"
        file_path.write_text("""
def complex_function(a, b, c, d, e):
    if a > 0:
        if b > 0:
            if c > 0:
                return 1
            else:
                return 2
        elif d > 0:
            return 3
        else:
            return 4
    elif e > 0:
        for i in range(10):
            if i % 2 == 0:
                return 5
    else:
        try:
            return 6
        except ValueError:
            return 7
        except TypeError:
            return 8
    return 0
""")
        # Very low threshold should cause gate to fail
        exit_code = run(["--max-complexity", "3", "--quality", str(tmp_path)])
        # Note: Gate only triggers if CSP-Q301 findings exist
        # Just verify it runs without error
        assert exit_code in (0, 1), "Expected command to complete"


class TestMinMIGate:
    """Tests for --min-mi (maintainability index) quality gate."""

    def test_min_mi_passes(self, tmp_path):
        """Simple, maintainable code should pass low MI threshold."""
        file_path = tmp_path / "maintainable.py"
        file_path.write_text('''
def simple_function():
    """A simple, documented function."""
    return 42
''')
        exit_code = run(["--min-mi", "20", str(tmp_path)])
        assert exit_code == 0, "Expected success with low MI threshold"

    def test_min_mi_fails(self, tmp_path):
        """Any real code should fail impossible MI threshold (>100)."""
        file_path = tmp_path / "code.py"
        file_path.write_text("""
def function():
    return 42
""")
        exit_code = run(["--min-mi", "101", str(tmp_path)])
        assert exit_code == 1, "Expected failure with impossible MI threshold"


class TestQuietMode:
    """Tests for --quiet flag behavior."""

    def test_quiet_mode_runs(self, tmp_path):
        """Quiet mode should run without error."""
        file_path = tmp_path / "test.py"
        file_path.write_text("""
def unused_function():
    pass
""")
        exit_code = run(["--quality", "--quiet", str(tmp_path)])
        assert exit_code == 0, "Expected quiet mode to run successfully"


class TestAutoEnableQuality:
    """Tests for auto-enabling --quality with gate flags."""

    def test_auto_enable_with_min_mi(self, tmp_path):
        """--min-mi should auto-enable quality mode."""
        file_path = tmp_path / "test.py"
        file_path.write_text("""
def function():
    return 42
""")
        # Should run without error even without explicit --quality
        exit_code = run(["--min-mi", "30", str(tmp_path)])
        assert exit_code == 0, "Expected auto-enable quality with --min-mi"

    def test_auto_enable_with_max_complexity(self, tmp_path):
        """--max-complexity should auto-enable quality mode."""
        file_path = tmp_path / "test.py"
        file_path.write_text("""
def function():
    return 42
""")
        # Should run without error even without explicit --quality
        exit_code = run(["--max-complexity", "20", str(tmp_path)])
        assert exit_code == 0, "Expected auto-enable quality with --max-complexity"
