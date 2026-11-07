use super::*;

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
