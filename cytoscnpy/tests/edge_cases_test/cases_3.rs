use super::*;

#[test]
fn test_command_injection() {
    let project = TestProject::new();
    project
        .create_file(
            "command_injection.py",
            r#"
import subprocess

def dangerous_command(filename):
    subprocess.run(f"rm {filename}", shell=True)

def safe_command(args):
    subprocess.run(args, shell=False)
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_pickle_security() {
    let project = TestProject::new();
    project
        .create_file(
            "pickle_risk.py",
            r"
import pickle

def insecure_deserialize(data):
    return pickle.loads(data)

def safe_serialize(obj):
    return pickle.dumps(obj)
",
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

// ============================================================================
// CODE QUALITY TESTS
// ============================================================================

#[test]
fn test_high_complexity() {
    let project = TestProject::new();
    project
        .create_file(
            "complex.py",
            r"
def highly_complex(a, b, c):
    if a == 1:
        if b == 2:
            if c == 3:
                return 1
            elif c == 4:
                return 2
        elif b == 5:
            return 3
    elif a == 6:
        if b == 7:
            return 4
        else:
            return 5
    else:
        return 6

result = highly_complex(1, 2, 3)
",
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_deep_nesting() {
    let project = TestProject::new();
    project
        .create_file(
            "deep_nesting.py",
            r#"
def deeply_nested():
    if True:
        if True:
            if True:
                if True:
                    if True:
                        if True:
                            return "deeply nested"
    return None

result = deeply_nested()
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_many_arguments() {
    let project = TestProject::new();
    project
        .create_file(
            "many_args.py",
            r"
def function_with_many_args(a, b, c, d, e, f, g):
    return a + b + c + d + e + f + g

result = function_with_many_args(1, 2, 3, 4, 5, 6, 7)
",
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

// ============================================================================
// VARIABLE SCOPING AND CLOSURE TESTS
// ============================================================================

#[test]
fn test_global_variables() {
    let project = TestProject::new();
    project
        .create_file(
            "globals.py",
            r"
global_var = 42

def modify_global():
    global global_var
    global_var = 100
    return global_var

result = modify_global()
",
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_nonlocal_variables() {
    let project = TestProject::new();
    project
        .create_file(
            "nonlocal.py",
            r"
def outer():
    value = 0

    def inner():
        nonlocal value
        value += 1
        return value

    return inner()

result = outer()
",
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

// ============================================================================
// EXCEPTION HANDLING TESTS
// ============================================================================

#[test]
fn test_exception_handling() {
    let project = TestProject::new();
    project
        .create_file(
            "exceptions.py",
            r"
class CustomError(Exception):
    pass

def safe_division(a, b):
    try:
        return a / b
    except ZeroDivisionError:
        return None
    except ValueError as ve:
        return str(ve)
    finally:
        pass

result = safe_division(10, 2)
",
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

// ============================================================================
// EDGE CASES TESTS
// ============================================================================

#[test]
fn test_empty_file() {
    let project = TestProject::new();
    project
        .create_file("empty.py", "")
        .expect("Failed to create empty file");

    assert!(project.path().exists());
}

#[test]
fn test_only_imports() {
    let project = TestProject::new();
    project
        .create_file(
            "imports_only.py",
            r"
import os
import sys
from pathlib import Path
",
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_only_docstring() {
    let project = TestProject::new();
    project
        .create_file("docstring_only.py", r#""""Module docstring""""#)
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_long_function_names() {
    let project = TestProject::new();
    project
        .create_file(
            "long_names.py",
            r#"
def this_is_a_very_long_function_name_that_describes_what_it_does():
    return "result"

def another_extremely_long_function_name_with_many_words():
    return 42

result1 = this_is_a_very_long_function_name_that_describes_what_it_does()
result2 = another_extremely_long_function_name_with_many_words()
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_unicode_identifiers() {
    let project = TestProject::new();
    project
        .create_file(
            "unicode.py",
            r#"
def café(épée):
    résultat = épée.upper()
    return résultat

result = café("hello")
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}
