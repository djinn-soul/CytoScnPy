import sys
from pathlib import Path

# Add the python/ directory to sys.path so we can import cytoscnpy
# This assumes tests are run from the project root or python/tests
project_root = Path(__file__).parent.parent
sys.path.insert(0, str(project_root))
