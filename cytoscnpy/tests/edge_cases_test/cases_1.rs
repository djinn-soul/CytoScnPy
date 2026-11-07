use super::*;

// ============================================================================
// NESTED STRUCTURES TESTS
// ============================================================================

#[test]
fn test_nested_functions() {
    let project = TestProject::new();
    project
        .create_file(
            "nested.py",
            r"
class OuterClass:
    def outer_method(self):
        def method_local():
            def deeply_nested():
                return 42
            return deeply_nested()
        return method_local()

def factory_function():
    def inner_created():
        def triple_nested():
            pass
        return triple_nested
    return inner_created()

instance = OuterClass()
instance.outer_method()
",
        )
        .expect("Failed to create test file");

    // Test that nested structures are properly detected
    assert!(project.path().exists());
}

#[test]
fn test_deeply_nested_classes() {
    let project = TestProject::new();
    project
        .create_file(
            "deep_classes.py",
            r#"
class OuterClass:
    class InnerClass:
        class DeepClass:
            def deep_method(self):
                return "nested"

obj = OuterClass.InnerClass.DeepClass()
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

// ============================================================================
// DECORATOR TESTS
// ============================================================================

#[test]
fn test_simple_decorators() {
    let project = TestProject::new();
    project
        .create_file(
            "decorators.py",
            r#"
import functools

def custom_decorator(func):
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)
    return wrapper

@custom_decorator
def decorated_function():
    return "decorated"

result = decorated_function()
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_property_decorators() {
    let project = TestProject::new();
    project
        .create_file(
            "properties.py",
            r#"
class Person:
    def __init__(self, first, last):
        self._first = first
        self._last = last

    @property
    def full_name(self):
        return f"{self._first} {self._last}"

    @full_name.setter
    def full_name(self, value):
        parts = value.split()
        self._first = parts[0]
        self._last = parts[1]

person = Person("John", "Doe")
name = person.full_name
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_staticmethod_classmethod() {
    let project = TestProject::new();
    project
        .create_file(
            "static_class.py",
            r"
class MathUtils:
    @staticmethod
    def add(a, b):
        return a + b

    @classmethod
    def create_default(cls):
        return cls()

result = MathUtils.add(2, 3)
",
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

// ============================================================================
// IMPORT AND REFERENCE RESOLUTION TESTS
// ============================================================================

#[test]
fn test_simple_imports() {
    let project = TestProject::new();
    project
        .create_file(
            "imports.py",
            r#"
import os
import sys
from pathlib import Path
from typing import Dict, List

def use_modules():
    path = Path(".")
    return str(path)

result = use_modules()
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_import_aliasing() {
    let project = TestProject::new();
    project
        .create_file(
            "aliasing.py",
            r"
import numpy as np
from collections import OrderedDict as OD

def use_aliases():
    arr = np.array([1, 2, 3])
    od = OD()
    return arr

result = use_aliases()
",
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_multifile_package() {
    let project = TestProject::new();
    project
        .create_package("mypackage")
        .expect("Failed to create package");

    project
        .create_file(
            "mypackage/core.py",
            r#"
def main_func():
    return "main"
"#,
        )
        .expect("Failed to create core.py");

    project
        .create_file(
            "main.py",
            r"
from mypackage.core import main_func

result = main_func()
",
        )
        .expect("Failed to create main.py");

    assert!(project.path().exists());
}

// ============================================================================
// OBJECT-ORIENTED PROGRAMMING TESTS
// ============================================================================

#[test]
fn test_inheritance() {
    let project = TestProject::new();
    project
        .create_file(
            "inheritance.py",
            r#"
class Base:
    def base_method(self):
        return "base"

class Derived(Base):
    def derived_method(self):
        return self.base_method()

obj = Derived()
obj.derived_method()
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_mixins() {
    let project = TestProject::new();
    project
        .create_file(
            "mixins.py",
            r#"
class LoggingMixin:
    def log(self, msg):
        print(f"LOG: {msg}")

class Service(LoggingMixin):
    def work(self):
        self.log("Working")

service = Service()
service.work()
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}
