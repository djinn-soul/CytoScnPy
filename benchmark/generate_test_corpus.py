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

from corpus_content import (
    FUNCTION_NAMES,
    generate_class,
    generate_file_content,
    generate_function,
    generate_imports,
)

__all__ = [
    "create_directory_structure",
    "generate_class",
    "generate_file_content",
    "generate_function",
    "generate_imports",
    "main",
]

PROGRESS_INTERVAL = 1000


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

        if files_created % PROGRESS_INTERVAL == 0:
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
