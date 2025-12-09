# CytoScnPy Test Suite

This directory contains tests for the Rust implementation of CytoScnPy.

## Test Structure

- `edge_cases_test.rs` - **Comprehensive edge case tests (~44 test functions covering 100+ scenarios)**
- `integration_test.rs` - End-to-end tests running the binary
- `visitor_test.rs` - Unit tests for AST visitor
- `framework_test.rs` - Tests for framework detection (Flask, Django, FastAPI)
- `test_utils_test.rs` - Tests for test file detection
- `security_test.rs` - Tests for secrets and dangerous code detection
- `quality_test.rs` - Tests for code quality checks

## Edge Case Tests Coverage

The `edge_cases_test.rs` module provides comprehensive test coverage for:

### Nested Structures

- Nested functions and closures
- Deeply nested classes
- Factory patterns

### Decorators & Properties

- Custom decorators with wrapper functions
- Property decorators and setters
- Static methods and class methods

### Imports & References

- Simple imports and aliasing
- Multi-file packages
- Relative imports

### Object-Oriented Programming

- Single inheritance
- Mixins
- Dataclasses
- Metaclasses
- Iterator protocol

### Advanced Python Features

- Async/await patterns
- Generator functions
- Lambda expressions
- Walrus operator (`:=`)
- F-strings and string formatting
- Unpacking and extended unpacking
- Enums and NamedTuples

### Magic & Special Methods

- Operator overloading (`__add__`, `__repr__`, etc.)
- Context managers (`__enter__`, `__exit__`)
- Iterator protocol (`__iter__`, `__next__`)

### Framework Detection

- Flask routes with multiple HTTP methods
- FastAPI endpoints with async handlers
- Django models with ORM

### Security Patterns

- SQL injection vulnerabilities
- Command injection with `shell=True`
- Insecure pickle deserialization

### Code Quality Metrics

- High cyclomatic complexity
- Deep nesting
- Functions with many arguments
- Very long functions

### Variable Scoping

- Global variables and `global` keyword
- Nonlocal variables and closures

### Exception Handling

- Try/except/finally blocks
- Custom exception classes
- Multiple exception handling

### Edge Cases

- Empty files
- Files with only imports/docstrings
- Very long function names
- Unicode identifiers
- Single character variable names
- Dynamic references with `globals()`

### Type Hints & Annotations

- Complex generic types
- TypeVar and Generic classes
- Union and Optional types
- Callable types

### Integration Tests

- Multi-file packages with circular imports
- Real-world project structure (models, utils, services)
- Package initialization with `__all__`

## Running Tests

```bash
# Run all tests
cargo test

# Run edge case tests specifically
cargo test --test edge_cases_test

# Run specific test
cargo test --test edge_cases_test test_nested_functions

# Run with output
cargo test -- --nocapture

# Run in release mode (faster)
cargo test --release

# Run specific test file
cargo test --test visitor_test
```

### Platform-Specific Commands

#### Windows

```powershell
cd cytoscnpy
cargo test --test edge_cases_test

# Or with verbose output
cargo test --test edge_cases_test -- --nocapture
```

#### Linux/macOS

```bash
cd cytoscnpy
cargo test --test edge_cases_test

# Or with verbose output
cargo test --test edge_cases_test -- --nocapture
```

## Current Status

✅ **All tests passing**: Full test suite (150+ tests across 30+ test files)

**Test Categories:**

- **Edge case tests**: 44 tests covering complex scenarios
- **Integration tests**: 16 tests validating end-to-end functionality
- **Unit tests**: Tests for visitor, complexity, halstead, metrics, etc.
- **Security tests**: 20 tests for secrets and dangerous code detection
- **Quality tests**: 8 tests for code quality checks
- **Framework tests**: 5 tests for Flask, Django, FastAPI detection

Note: Most other tests create test files dynamically using the `tempfile` crate.

## Adding New Tests

### Option 1: Add to existing edge_cases_test.rs

```rust
#[test]
fn test_my_scenario() {
    let project = TestProject::new();
    project
        .create_file("test.py", r#"
// Your Python code here
"#)
        .expect("Failed to create test file");

    assert!(project.path().exists());
}
```

### Option 2: Create a new test file

1. Create a new file in `tests/` directory: `my_feature_test.rs`
2. Use the `TestProject` helper from `edge_cases_test.rs` as a template
3. Write test functions with `#[test]` attribute
4. Import required modules: `use std::fs::File;`

## Test Data

Test fixtures are created dynamically using the `tempfile` crate, allowing isolated testing without external files. This mirrors the Python test suite structure.

## Test Examples

### Testing a Simple Feature

```rust
#[test]
fn test_my_feature() {
    let project = TestProject::new();
    project
        .create_file("feature.py", r#"
def my_function():
    return "result"

result = my_function()
"#)
        .expect("Failed to create test file");

    assert!(project.path().exists());
}
```

### Testing a Package Structure

```rust
#[test]
fn test_package_structure() {
    let project = TestProject::new();
    project.create_package("mymodule").expect("Failed to create package");

    project
        .create_file("mymodule/core.py", "def core_func(): pass")
        .expect("Failed to create core.py");

    project
        .create_file("main.py", "from mymodule.core import core_func")
        .expect("Failed to create main.py");

    assert!(project.path().exists());
}
```

## FAQ

**Q: How do I run just one test?**
A: Use `cargo test --test edge_cases_test test_name`

**Q: How do I see test output?**
A: Add `-- --nocapture` to the command: `cargo test --test edge_cases_test -- --nocapture`

**Q: Why do some tests fail?**
A: Check that the Rust analyzer binary (`cytoscnpy`) is properly compiled. You may need to run `cargo build` first.

**Q: Can I add tests without building the binary?**
A: Yes! The `TestProject` helper creates test files but doesn't run the analyzer. To test analyzer output, you'd need the compiled binary.

**Q: How do I debug a failing test?**
A: Run with: `cargo test --test edge_cases_test test_name -- --nocapture --test-threads=1`
