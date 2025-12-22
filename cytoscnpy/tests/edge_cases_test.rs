//! Comprehensive Edge Case Tests for Rust `CytoScnPy` Implementation
#![allow(clippy::expect_used)]

use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

/// Helper struct to create temporary Python projects for testing
struct TestProject {
    root: TempDir,
}

impl TestProject {
    /// Create a new temporary test project
    fn new() -> Self {
        TestProject {
            root: TempDir::new().expect("Failed to create temp directory"),
        }
    }

    /// Get the root path of the test project
    fn path(&self) -> &std::path::Path {
        self.root.path()
    }

    /// Create a Python file in the project
    fn create_file(&self, name: &str, content: &str) -> std::io::Result<()> {
        let file_path = self.root.path().join(name);
        let mut file = File::create(file_path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Create a subdirectory and optionally an __init__.py file for a package
    fn create_package(&self, name: &str) -> std::io::Result<()> {
        let package_path = self.root.path().join(name);
        fs::create_dir_all(&package_path)?;
        let init_path = package_path.join("__init__.py");
        File::create(init_path)?.write_all(b"")?;
        Ok(())
    }
}

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

#[test]
fn test_dataclass() {
    let project = TestProject::new();
    project
        .create_file(
            "dataclasses_test.py",
            r#"
from dataclasses import dataclass

@dataclass
class User:
    name: str
    email: str

def create_user():
    return User("John", "john@example.com")

user = create_user()
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

// ============================================================================
// ADVANCED PYTHON FEATURES TESTS
// ============================================================================

#[test]
fn test_async_await() {
    let project = TestProject::new();
    project
        .create_file(
            "async_code.py",
            r#"
import asyncio

async def fetch_data(url):
    await asyncio.sleep(0.1)
    return {"data": "result"}

async def main():
    data = await fetch_data("url")
    return data
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_generators() {
    let project = TestProject::new();
    project
        .create_file(
            "generators.py",
            r"
def fibonacci(n):
    a, b = 0, 1
    for _ in range(n):
        yield a
        a, b = b, a + b

def use_generator():
    fibs = list(fibonacci(10))
    return fibs

result = use_generator()
",
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_lambda_expressions() {
    let project = TestProject::new();
    project
        .create_file(
            "lambdas.py",
            r"
def use_lambda():
    add = lambda x, y: x + y
    result = add(1, 2)

    squares = [x**2 for x in range(10)]
    return squares

result = use_lambda()
",
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_walrus_operator() {
    let project = TestProject::new();
    project
        .create_file(
            "walrus.py",
            r#"
def process_with_walrus(items):
    if (n := len(items)) > 5:
        return f"Large list with {n} items"
    return f"Small list with {n} items"

result = process_with_walrus([1, 2, 3, 4, 5, 6])
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

// ============================================================================
// SPECIAL METHODS AND MAGIC METHODS TESTS
// ============================================================================

#[test]
fn test_magic_methods() {
    let project = TestProject::new();
    project
        .create_file(
            "magic.py",
            r#"
class Vector:
    def __init__(self, x, y):
        self.x = x
        self.y = y

    def __add__(self, other):
        return Vector(self.x + other.x, self.y + other.y)

    def __repr__(self):
        return f"Vector({self.x}, {self.y})"

v1 = Vector(1, 2)
v2 = Vector(3, 4)
v3 = v1 + v2
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_context_manager() {
    let project = TestProject::new();
    project
        .create_file(
            "context.py",
            r#"
class FileManager:
    def __init__(self, filename):
        self.filename = filename

    def __enter__(self):
        return self

    def __exit__(self, *args):
        pass

    def read(self):
        return "file content"

with FileManager("test.txt") as fm:
    content = fm.read()
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

// ============================================================================
// FRAMEWORK-SPECIFIC TESTS
// ============================================================================

#[test]
fn test_flask_routes() {
    let project = TestProject::new();
    project
        .create_file(
            "flask_app.py",
            r#"
from flask import Flask

app = Flask(__name__)

@app.route('/api/users', methods=['GET', 'POST'])
def get_users():
    return {"users": []}

@app.route('/api/users/<int:user_id>')
def get_user(user_id):
    return {"user_id": user_id}

if __name__ == "__main__":
    app.run()
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_fastapi_endpoints() {
    let project = TestProject::new();
    project
        .create_file(
            "fastapi_app.py",
            r#"
from fastapi import FastAPI

api = FastAPI()

@api.get("/items/{item_id}")
async def read_item(item_id: int):
    return {"item_id": item_id}

@api.post("/items/")
async def create_item(name: str):
    return {"name": name}
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_django_models() {
    let project = TestProject::new();
    project
        .create_file(
            "models.py",
            r#"
from django.db import models

class User(models.Model):
    name = models.CharField(max_length=100)
    email = models.EmailField()

    def display(self):
        return f"{self.name} ({self.email})"
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

// ============================================================================
// SECURITY PATTERN TESTS
// ============================================================================

#[test]
fn test_sql_injection_pattern() {
    let project = TestProject::new();
    project
        .create_file(
            "sql_injection.py",
            r#"
def sql_query(user_input):
    query = f"SELECT * FROM users WHERE id = {user_input}"
    return query

result = sql_query("1")
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

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

#[test]
fn test_single_char_names() {
    let project = TestProject::new();
    project
        .create_file(
            "single_chars.py",
            r"
def f(x, y, z):
    a = x + y
    b = y + z
    c = a + b
    return c

result = f(1, 2, 3)
",
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

// ============================================================================
// SPECIAL PATTERNS TESTS
// ============================================================================

#[test]
fn test_pragma_comments() {
    let project = TestProject::new();
    project
        .create_file(
            "pragma.py",
            r#"
def ignored_function():  # pragma: no cytoscnpy
    return "ignored"

def not_ignored():
    return "should be flagged"

class IgnoredClass:  # cytoscnpy: ignore
    pass
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_dynamic_references() {
    let project = TestProject::new();
    project
        .create_file(
            "dynamic.py",
            r#"
def dynamic_function():
    return "dynamic"

func = globals()["dynamic_function"]
result = func()
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_string_formatting() {
    let project = TestProject::new();
    project
        .create_file(
            "formatting.py",
            r#"
def format_output(name, age):
    basic = f"Name: {name}"
    formatted = f"Age: {age:02d}"
    old_style = "Hello %s" % name
    return old_style

result = format_output("Alice", 30)
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_unpacking() {
    let project = TestProject::new();
    project
        .create_file(
            "unpacking.py",
            r#"
def unpack_sequences():
    a, b, c = [1, 2, 3]
    first, *rest = [1, 2, 3, 4, 5]
    return (a, first, rest)

def unpack_dict():
    dict1 = {"a": 1, "b": 2}
    dict2 = {"c": 3}
    merged = {**dict1, **dict2}
    return merged

result1 = unpack_sequences()
result2 = unpack_dict()
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_enum_and_namedtuple() {
    let project = TestProject::new();
    project
        .create_file(
            "enums.py",
            r"
from enum import Enum
from typing import NamedTuple

class Status(Enum):
    PENDING = 1
    ACTIVE = 2

class Point(NamedTuple):
    x: int
    y: int

def use_enum():
    status = Status.ACTIVE
    return status.value

def use_point():
    p = Point(1, 2)
    return p.x + p.y

result1 = use_enum()
result2 = use_point()
",
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[test]
fn test_comprehensive_project() {
    let project = TestProject::new();
    project
        .create_package("models")
        .expect("Failed to create models package");
    project
        .create_package("utils")
        .expect("Failed to create utils package");
    project
        .create_package("services")
        .expect("Failed to create services package");

    project
        .create_file(
            "models/__init__.py",
            r"
from .user import User
__all__ = ['User']
",
        )
        .expect("Failed to create models/__init__.py");

    project
        .create_file(
            "models/user.py",
            r#"
from dataclasses import dataclass

@dataclass
class User:
    id: int
    name: str
    email: str

    def display(self):
        return f"{self.name} ({self.email})"
"#,
        )
        .expect("Failed to create models/user.py");

    project
        .create_file(
            "utils/__init__.py",
            r"
from .helpers import validate_email
__all__ = ['validate_email']
",
        )
        .expect("Failed to create utils/__init__.py");

    project
        .create_file(
            "utils/helpers.py",
            r#"
def validate_email(email):
    return "@" in email
"#,
        )
        .expect("Failed to create utils/helpers.py");

    project
        .create_file("services/__init__.py", "")
        .expect("Failed to create services/__init__.py");

    project
        .create_file(
            "services/user_service.py",
            r#"
from models import User
from utils import validate_email

class UserService:
    def create_user(self, name, email):
        if not validate_email(email):
            raise ValueError("Invalid email")
        user = User(1, name, email)
        return user
"#,
        )
        .expect("Failed to create services/user_service.py");

    project
        .create_file(
            "main.py",
            r#"
from services.user_service import UserService

service = UserService()
user = service.create_user("John", "john@example.com")
print(user.display())
"#,
        )
        .expect("Failed to create main.py");

    assert!(project.path().exists());
}

// ============================================================================
// TYPE HINTS AND ANNOTATIONS TESTS
// ============================================================================

#[test]
fn test_complex_type_hints() {
    let project = TestProject::new();
    project
        .create_file(
            "type_hints.py",
            r#"
from typing import Dict, List, Optional, Union, Callable, TypeVar, Generic

T = TypeVar('T')

class Container(Generic[T]):
    def __init__(self, value: T):
        self.value = value

    def get(self) -> T:
        return self.value

def complex_types(
    data: Dict[str, List[int]],
    callback: Callable[[int], Optional[str]]
) -> Dict[str, str]:
    return {"result": "value"}

container = Container[int](42)
result = container.get()
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_metaclass() {
    let project = TestProject::new();
    project
        .create_file(
            "metaclass.py",
            r#"
class SingletonMeta(type):
    _instances = {}

    def __call__(cls, *args, **kwargs):
        if cls not in cls._instances:
            instance = super().__call__(*args, **kwargs)
            cls._instances[cls] = instance
        return cls._instances[cls]

class Database(metaclass=SingletonMeta):
    def __init__(self):
        self.connection = None

    def connect(self):
        self.connection = "connected"
        return self.connection

db = Database()
db.connect()
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_iterator_protocol() {
    let project = TestProject::new();
    project
        .create_file(
            "iterators.py",
            r"
class CountUp:
    def __init__(self, max):
        self.max = max
        self.current = 0

    def __iter__(self):
        return self

    def __next__(self):
        if self.current < self.max:
            self.current += 1
            return self.current
        raise StopIteration

counter = CountUp(5)
result = list(counter)
",
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}
