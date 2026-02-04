import sys
from pathlib import Path
from unittest.mock import patch

import pytest

# Add benchmark directory to sys.path
BENCHMARK_DIR = Path(__file__).resolve().parent.parent
sys.path.append(str(BENCHMARK_DIR))

try:
    import generate_test_corpus
except ImportError:
    pytest.fail("Could not import generate_test_corpus from benchmark directory")


def test_generate_function():
    """Test function generation."""
    code = generate_test_corpus.generate_function("my_func", used=True, complexity=1)
    assert "def my_func(" in code
    assert "return data" in code or "return" in code

    code_complex = generate_test_corpus.generate_function(
        "complex_func", used=True, complexity=2
    )
    assert "def complex_func(" in code_complex
    assert "for item in data:" in code_complex


def test_generate_class():
    """Test class generation."""
    code = generate_test_corpus.generate_class("MyClass", num_methods=2)
    assert "class MyClass:" in code
    assert "def __init__(self, config: dict):" in code
    # We requested 2 methods + init = 3 defs or just check for method presence
    assert code.count("def ") >= 3


def test_generate_imports():
    """Test import generation."""
    code, used_names = generate_test_corpus.generate_imports(
        num_imports=5, used_ratio=1.0
    )
    assert len(code.splitlines()) >= 5
    # Check that we got some used names
    assert len(used_names) > 0

    # Check content
    if "import os" in code:
        pass  # Good

    # Check return structure
    assert isinstance(code, str)
    assert isinstance(used_names, list)


def test_generate_file_content():
    """Test content generation for different file types."""
    # Module
    content = generate_test_corpus.generate_file_content("module")
    assert "def " in content or "import " in content

    # Class
    content = generate_test_corpus.generate_file_content("class")
    assert "class " in content

    # Script
    content = generate_test_corpus.generate_file_content("script")
    assert 'if __name__ == "__main__":' in content

    # Test
    content = generate_test_corpus.generate_file_content("test")
    assert "test_" in content
    assert "import pytest" in content or "import unittest" in content


def test_create_directory_structure(tmp_path):
    """Test creation of directory structure and files."""
    output_dir = tmp_path / "corpus"
    num_files = 10

    generated_count = generate_test_corpus.create_directory_structure(
        output_dir, num_files
    )

    assert generated_count >= num_files

    # Verify file existence
    files = list(output_dir.rglob("*.py"))
    # Note: create_directory_structure also creates __init__.py files, so strict count match might vary
    # but it should be at least num_files + package inits
    assert len(files) >= num_files

    # Verify we have some packages
    assert (output_dir / "core").exists()
    assert (output_dir / "utils").exists()

    # Verify content of a generated file
    # Pick a random non-init file
    py_files = [f for f in files if f.name != "__init__.py"]
    if py_files:
        content = py_files[0].read_text()
        assert len(content) > 0


def test_main(tmp_path):
    """Test main execution flow."""
    output = tmp_path / "out"

    with patch(
        "sys.argv", ["script", "--files", "5", "--output", str(output), "--clean"]
    ):
        with patch("generate_test_corpus.create_directory_structure") as mock_create:
            mock_create.return_value = 5

            with patch("builtins.print"):
                generate_test_corpus.main()

            mock_create.assert_called_once()
            args = mock_create.call_args
            # check output path
            assert str(args[0][0]) == str(output)
            # check num files
            assert args[0][1] == 5

    # Verify directory creation happens (the script calls mkdir before create_directory_structure)
    assert output.exists()


def test_main_clean_existing(tmp_path):
    """Test main with --clean on existing directory."""
    output = tmp_path / "out_clean"
    output.mkdir()
    (output / "old.txt").write_text("old")

    with patch(
        "sys.argv", ["script", "--files", "1", "--output", str(output), "--clean"]
    ):
        # Mock create so we don't spend time generating
        with patch("generate_test_corpus.create_directory_structure", return_value=1):
            with patch("builtins.print"):
                generate_test_corpus.main()

    # Old file should be gone (directory recreated)
    assert not (output / "old.txt").exists()
    assert output.exists()


def test_generate_mixed_content():
    """Test generating mixed content explicitly."""
    content = generate_test_corpus.generate_file_content("mixed")
    assert "class " in content
    assert "def " in content
