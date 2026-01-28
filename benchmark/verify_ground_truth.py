import json
import os


def find_ground_truth_files(root_dir):
    """Find all ground_truth.json files in the directory."""
    gt_files = []
    for root, dirs, files in os.walk(root_dir):
        if "ground_truth.json" in files:
            gt_files.append(os.path.join(root, "ground_truth.json"))
    return gt_files


def verify_item(file_path, item, lines):
    """Verify a single truth item against the source file content."""
    line_start = item.get("line_start")
    name = item.get("name")
    item_type = item.get("type")

    if not line_start or not name:
        return f"MISSING FIELDS: {item}"

    if line_start > len(lines):
        return f"LINE OUT OF BOUNDS: line {line_start} > {len(lines)}"

    # Python lines are 1-based, list is 0-based
    line_content = lines[line_start - 1].strip()

    # Basic Heuristics
    if item_type == "function" or item_type == "method":
        search_name = name.split(".")[-1]
        if (
            f"def {search_name}" not in line_content
            and f"async def {search_name}" not in line_content
        ):
            return f"TYPE MISMATCH: Expected function/method '{name}' (looking for 'def {search_name}') at line {line_start}, found: {line_content}"
    elif item_type == "class":
        if f"class {name}" not in line_content:
            return f"TYPE MISMATCH: Expected class '{name}' at line {line_start}, found: {line_content}"
    elif item_type == "import":
        # Handle both single-line imports and multi-line import continuations
        # For multi-line: "from x import (\n    name1,\n    name2\n)" - name appears on its line
        # For single-line: "import os" or "from x import name"
        if name not in line_content and "import" not in line_content:
            return f"TYPE MISMATCH: Expected import '{name}' at line {line_start}, found: {line_content}"
    elif item_type == "variable":
        # loose check for variable assignment or usage
        if name not in line_content:
            # Try simple name (e.g. MultiKeywordClass.name -> name)
            simple_name = name.split(".")[-1]
            if simple_name not in line_content:
                return f"NAME NOT FOUND: Expected variable '{name}' at line {line_start}, found: {line_content}"

    return None


def verify_ground_truth(gt_path):
    """Verify entire ground truth file against source files."""
    issues = []
    base_dir = os.path.dirname(gt_path)

    try:
        with open(gt_path, "r", encoding="utf-8") as f:
            data = json.load(f)
    except Exception as e:
        return [f"JSON ERROR: {str(e)}"]

    files = data.get("files", {})
    if not files:
        # Check if it's the other format (list of items directly? rare provided schema)
        pass

    for filename, file_data in files.items():
        py_path = os.path.join(base_dir, filename)
        if not os.path.exists(py_path):
            issues.append(f"FILE MISSING: {filename} not found in {base_dir}")
            continue

        with open(py_path, "r", encoding="utf-8") as f:
            content = f.read()
            lines = content.splitlines()

        dead_items = file_data.get("dead_items", [])
        for item in dead_items:
            if item.get("suppressed"):
                continue
            issue = verify_item(py_path, item, lines)
            if issue:
                issues.append(f"{filename}: {issue}")

    return issues


def main():
    """Main verification logic."""
    root_dir = r"e:\Github\CytoScnPy\benchmark\examples"
    gt_files = find_ground_truth_files(root_dir)

    total_issues = 0
    for gt in gt_files:
        issues = verify_ground_truth(gt)
        if issues:
            print(f"Issues in {gt}:")
            for i in issues:
                print(f"  - {i}")
            print("-" * 40)
            total_issues += len(issues)

    print(f"\nTotal Issues Found: {total_issues}")


if __name__ == "__main__":
    main()
