import shutil
import sys
from pathlib import Path

# Add the python/ directory to sys.path so we can import cytoscnpy
# This assumes tests are run from the project root or python/tests
# __file__ is e:\Github\CytoScnPy\python\tests\conftest.py
# .parent is e:\Github\CytoScnPy\python\tests
# .parent.parent is e:\Github\CytoScnPy\python
# .parent.parent.parent is e:\Github\CytoScnPy
project_root = Path(__file__).parent.parent.parent
sys.path.insert(0, str(Path(__file__).parent.parent))


def pytest_sessionfinish(session, exitstatus):
    """Clean up the local .pytest_tmp directory after tests finish."""
    tmp_dir = project_root / ".pytest_tmp"
    if tmp_dir.exists():
        try:
            shutil.rmtree(tmp_dir, ignore_errors=True)
        except Exception:  # noqa: S110
            pass
