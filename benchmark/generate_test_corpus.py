#!/usr/bin/env python3
"""
Generate a realistic test corpus of 10,000 Python files for benchmarking.

Creates varied Python code patterns to stress-test all analyzer features:
- Functions, classes, methods
- Imports (unused, used, from imports)
- Variables, parameters
- Type hints
- Decorators
- Entry points (__main__ blocks)
- Docstrings
- Comments
"""

from __future__ import annotations

import argparse
import random
import shutil
from pathlib import Path

# Realistic function/class names from common libraries
FUNCTION_NAMES: list[str] = [
    "process_data",
    "validate_input",
    "calculate_metrics",
    "parse_response",
    "serialize_object",
    "deserialize_json",
    "fetch_records",
    "update_database",
    "send_email",
    "generate_report",
    "compress_file",
    "extract_archive",
    "authenticate_user",
    "authorize_request",
    "encrypt_data",
    "decrypt_payload",
    "log_message",
    "format_output",
    "transform_data",
    "aggregate_results",
]

CLASS_NAMES = [
    "DataProcessor",
    "UserManager",
    "RequestHandler",
    "ResponseBuilder",
    "DatabaseConnection",
    "CacheManager",
    "FileHandler",
    "EmailService",
    "AuthenticationProvider",
    "AuthorizationService",
    "EncryptionService",
    "Logger",
    "Formatter",
    "Transformer",
    "Aggregator",
    "Validator",
]

IMPORT_MODULES = [
    "os",
    "sys",
    "json",
    "typing",
    "pathlib",
    "datetime",
    "re",
    "collections",
    "itertools",
    "functools",
    "logging",
    "argparse",
    "subprocess",
    "shutil",
    "requests",
    "numpy",
    "pandas",
    "flask",
    "django",
    "pytest",
]


def generate_function(name: str, *, used: bool = True, complexity: int = 1) -> str:
    """Generate a Python function with varying complexity."""
    if not used:
        # Placeholder to avoid ARG001, in a real case we might change the body
        pass
    params = (
        ["data: dict", "config: dict", "options: list"] if random.random() > 0.5 else []
    )
    param_str = ", ".join(params[: random.randint(0, len(params))])

    # Add docstring
    doc = (
        f'    """Process {name.replace("_", " ")}."""\n'
        if random.random() > 0.3
        else ""
    )

    # Generate body with varying complexity
    body_lines: list[str] = []
    if complexity > 1:
        body_lines.append("    result = []")
        body_lines.append("    for item in data:")
        body_lines.append("        if item.get('active'):")
        body_lines.append("            result.append(item)")
        body_lines.append("    return result")
    else:
        body_lines.append("    return data")

    body = "\n".join(body_lines)

    return f"def {name}({param_str}):\n{doc}{body}\n\n"


def generate_class(name: str, num_methods: int = 3) -> str:
    """Generate a Python class with methods."""
    methods: list[str] = []

    # Constructor
    methods.append(
        "    def __init__(self, config: dict):\n"
        "        self.config = config\n"
        "        self.state = {}\n"
    )

    # Regular methods
    method_names = random.sample(FUNCTION_NAMES, min(num_methods, len(FUNCTION_NAMES)))
    methods.extend(
        f"    def {method_name}(self, data: dict) -> dict:\n"
        f'        """Process data using {method_name}."""\n'
        "        return self.config.get('result', {})\n"
        for method_name in method_names
    )

    return f"class {name}:\n" + "\n".join(methods) + "\n\n"


def generate_imports(
    num_imports: int, used_ratio: float = 0.6
) -> tuple[str, list[str]]:
    """Generate import statements and return used names."""
    imports: list[str] = []
    used_names: list[str] = []

    selected_modules = random.sample(
        IMPORT_MODULES, min(num_imports, len(IMPORT_MODULES))
    )

    for module in selected_modules:
        if random.random() > 0.5:
            # from X import Y pattern
            if module in ["typing", "collections"]:
                items = (
                    ["Dict", "List", "Optional"]
                    if module == "typing"
                    else ["defaultdict", "Counter"]
                )
                item = random.choice(items)
                imports.append(f"from {module} import {item}")
                if random.random() < used_ratio:
                    used_names.append(item)
            else:
                imports.append(f"import {module}")
                if random.random() < used_ratio:
                    used_names.append(module)
        else:
            # import X pattern
            imports.append(f"import {module}")
            if random.random() < used_ratio:
                used_names.append(module)

    return "\n".join(imports) + "\n\n", used_names


def _generate_module_content() -> str:
    """Generate content for a module."""
    imports, used = generate_imports(random.randint(3, 8))
    functions: list[str] = []
    for i in range(random.randint(5, 15)):
        func_name = random.choice(FUNCTION_NAMES) + f"_{i}"
        is_used = random.random() > 0.4
        complexity = random.randint(1, 3)
        functions.append(
            generate_function(func_name, used=is_used, complexity=complexity)
        )
    usage = ""
    if used and random.random() > 0.5:
        usage = f"\n# Use imported module\nresult = {used[0]}\n"
    return imports + "".join(functions) + usage


def _generate_class_content() -> str:
    """Generate content for a class-based module."""
    imports, _ = generate_imports(random.randint(2, 5))
    classes = [
        generate_class(
            random.choice(CLASS_NAMES) + (f"{i}" if i > 0 else ""), random.randint(3, 8)
        )
        for i in range(random.randint(2, 5))
    ]
    return imports + "".join(classes)


def _generate_script_content() -> str:
    """Generate content for a script with __main__ block."""
    imports, _ = generate_imports(random.randint(2, 4), used_ratio=0.9)
    functions = [
        generate_function(
            random.choice(FUNCTION_NAMES) + f"_{i}", used=True, complexity=2
        )
        for i in range(random.randint(2, 5))
    ]
    main_block = "\nif __name__ == \"__main__\":\n    config = {'debug': True}\n"
    if functions:
        func_name = functions[0].split("def ")[1].split("(")[0]
        main_block += f"    result = {func_name}({{}}, config, [])\n"
        main_block += "    print(result)\n"
    return imports + "".join(functions) + main_block


def _generate_test_content() -> str:
    """Generate content for a test file."""
    imports = (
        "import pytest\nimport unittest\nfrom unittest.mock import Mock, patch\n\n"
    )
    tests = [
        f"def test_{random.choice(FUNCTION_NAMES)}_{i}():\n"
        f'    """Test {random.choice(FUNCTION_NAMES)}_{i}."""\n'
        "    assert True\n\n"
        for i in range(random.randint(5, 15))
    ]
    return imports + "".join(tests)


def _generate_mixed_content() -> str:
    """Generate content for mixed module."""
    imports, _ = generate_imports(random.randint(3, 6))
    content = [generate_class(random.choice(CLASS_NAMES), num_methods=4)]
    content.extend(
        generate_function(
            random.choice(FUNCTION_NAMES) + f"_{i}", complexity=random.randint(1, 2)
        )
        for i in range(random.randint(3, 6))
    )
    return imports + "".join(content)


def generate_file_content(file_type: str) -> str:
    """Generate content for a Python file based on type."""
    dispatch = {
        "module": _generate_module_content,
        "class": _generate_class_content,
        "script": _generate_script_content,
        "test": _generate_test_content,
    }
    return dispatch.get(file_type, _generate_mixed_content)()


def create_directory_structure(base_path: Path, num_files: int):
    """Create a realistic package structure with nested modules."""

    # Create packages
    packages = [
        "core",
        "utils",
        "models",
        "services",
        "api",
        "tests",
        "handlers",
        "middleware",
        "validators",
        "serializers",
        "auth",
        "database",
        "cache",
        "tasks",
        "workers",
    ]

    # Distribution of file types
    file_types = {
        "module": 0.35,  # 35% - regular modules with functions
        "class": 0.25,  # 25% - class-based modules
        "script": 0.15,  # 15% - executable scripts
        "test": 0.15,  # 15% - test files
        "mixed": 0.10,  # 10% - mixed content
    }

    files_created = 0
    package_dirs: list[Path] = []

    # Create package directories
    for package in packages:
        pkg_dir = base_path / package
        pkg_dir.mkdir(parents=True, exist_ok=True)
        package_dirs.append(pkg_dir)

        # Create __init__.py
        (pkg_dir / "__init__.py").write_text('"""Package initialization."""\n')
        files_created += 1

        # Create subpackages
        for subpkg in random.sample(
            ["internal", "external", "helpers", "tests"], k=random.randint(1, 2)
        ):
            sub_dir = pkg_dir / subpkg
            sub_dir.mkdir(exist_ok=True)
            (sub_dir / "__init__.py").write_text("")
            package_dirs.append(sub_dir)
            files_created += 1

    # Generate files
    while files_created < num_files:
        # Choose random package
        target_dir = random.choice(package_dirs)

        # Choose file type based on distribution
        rand = random.random()
        cumulative = 0
        file_type = "module"
        for ftype, prob in file_types.items():
            cumulative += prob
            if rand < cumulative:
                file_type = ftype
                break

        # Generate filename
        if file_type == "test":
            filename = f"test_{random.choice(FUNCTION_NAMES)}_{files_created}.py"
        else:
            filename = f"{random.choice(FUNCTION_NAMES)}_{files_created}.py"

        file_path = target_dir / filename

        # Generate content
        content = generate_file_content(file_type)
        file_path.write_text(content)

        files_created += 1

        if files_created % 1000 == 0:
            print(f"Generated {files_created}/{num_files} files...")

    print(f"✓ Created {files_created} Python files in {base_path}")
    return files_created


def main():
    parser = argparse.ArgumentParser(
        description="Generate test corpus for benchmarking"
    )
    parser.add_argument(
        "--files",
        type=int,
        default=10000,
        help="Number of files to generate (default: 10000)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("benchmark_corpus"),
        help="Output directory (default: benchmark_corpus)",
    )
    parser.add_argument(
        "--clean",
        action="store_true",
        help="Remove existing output directory before generating",
    )

    args = parser.parse_args()

    if args.clean and args.output.exists():
        print(f"Removing existing directory: {args.output}")
        shutil.rmtree(args.output)

    args.output.mkdir(parents=True, exist_ok=True)

    print(f"Generating {args.files} Python files in {args.output}...")
    print("This will create realistic code patterns including:")
    print("  - Functions and classes")
    print("  - Used and unused imports")
    print("  - Test files")
    print("  - Scripts with __main__ blocks")
    print("  - Nested package structures")
    print()

    files_created = create_directory_structure(args.output, args.files)

    print("\n✓ Benchmark corpus created successfully!")
    print(f"  Total files: {files_created}")
    print(f"  Location: {args.output.absolute()}")
    print("\nRun benchmark with:")
    print(f"  hyperfine 'cytoscnpy analyze {args.output}'")


if __name__ == "__main__":
    main()
