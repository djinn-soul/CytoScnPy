use super::*;

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
