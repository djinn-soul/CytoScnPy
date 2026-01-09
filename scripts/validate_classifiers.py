try:
    import tomllib  # ty: ignore[unresolved-import]
except ImportError:
    import tomli as tomllib  # ty: ignore[unresolved-import]
import sys
from trove_classifiers import classifiers  # ty: ignore[unresolved-import]


def validate_classifiers():
    """Validate PyPI classifiers against trove classifiers."""
    with open("pyproject.toml", "rb") as f:
        data = tomllib.load(f)

    project_classifiers = data.get("project", {}).get("classifiers", [])

    invalid_classifiers = [c for c in project_classifiers if c not in classifiers]

    if invalid_classifiers:
        print(
            "Error: Invalid PyPI classifiers found in pyproject.toml:", file=sys.stderr
        )
        for ic in invalid_classifiers:
            print(f"  - {ic}", file=sys.stderr)
        sys.exit(1)
    else:
        print("Success: All classifiers are valid.")
        sys.exit(0)


if __name__ == "__main__":
    validate_classifiers()
