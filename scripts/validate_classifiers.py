import tomllib
import sys
from trove_classifiers import classifiers

def validate_classifiers():
    with open("pyproject.toml", "rb") as f:
        data = tomllib.load(f)
    
    project_classifiers = data.get("project", {}).get("classifiers", [])
    
    invalid_classifiers = []
    for c in project_classifiers:
        if c not in classifiers:
            invalid_classifiers.append(c)
            
    if invalid_classifiers:
        print("Error: Invalid PyPI classifiers found in pyproject.toml:")
        for ic in invalid_classifiers:
            print(f"  - {ic}")
        sys.exit(1)
    else:
        print("Success: All classifiers are valid.")
        sys.exit(0)

if __name__ == "__main__":
    validate_classifiers()
