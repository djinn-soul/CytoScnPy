use super::*;

// ============================================================================
// FastAPI-Specific Pattern Tests
// ============================================================================

#[test]
fn test_fastapi_depends_detection() {
    let source = r#"
from fastapi import FastAPI, Depends

def get_db():
    pass

def get_current_user():
    pass

app = FastAPI()

@app.get("/items")
def get_items(db = Depends(get_db), user = Depends(get_current_user)):
    pass
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    assert!(visitor.is_framework_file, "Should detect FastAPI file");
    assert!(
        visitor.detected_frameworks.contains("fastapi"),
        "Should detect 'fastapi' framework"
    );
    assert!(
        visitor.framework_references.contains(&"get_db".to_string()),
        "Should extract get_db from Depends()"
    );
    assert!(
        visitor
            .framework_references
            .contains(&"get_current_user".to_string()),
        "Should extract get_current_user from Depends()"
    );
}

#[test]
fn test_fastapi_async_depends() {
    let source = r#"
from fastapi import Depends

def verify_token():
    pass

async def get_items(token = Depends(verify_token)):
    pass
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    assert!(
        visitor
            .framework_references
            .contains(&"verify_token".to_string()),
        "Should extract dependency from async function"
    );
}

// ============================================================================
// Pydantic-Specific Pattern Tests
// ============================================================================

#[test]
fn test_pydantic_basemodel_detection() {
    let source = r#"
from pydantic import BaseModel

class UserModel(BaseModel):
    name: str
    email: str
    age: int
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    assert!(visitor.is_framework_file, "Should detect Pydantic file");
    assert!(
        visitor.detected_frameworks.contains("pydantic"),
        "Should detect 'pydantic' framework"
    );
    assert!(
        visitor.framework_references.contains(&"name".to_string()),
        "Should mark 'name' field as used"
    );
    assert!(
        visitor.framework_references.contains(&"email".to_string()),
        "Should mark 'email' field as used"
    );
    assert!(
        visitor.framework_references.contains(&"age".to_string()),
        "Should mark 'age' field as used"
    );
}

#[test]
fn test_pydantic_with_default_values() {
    let source = r#"
from pydantic import BaseModel
from typing import Optional

class Config(BaseModel):
    host: str = "localhost"
    port: int = 8080
    debug: Optional[bool] = None
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    assert!(visitor.is_framework_file, "Should detect Pydantic file");
    assert_eq!(
        visitor.framework_references.len(),
        3,
        "Should find all 3 Pydantic fields"
    );
}

#[test]
fn test_init_default() {
    // Test that FrameworkAwareVisitor initializes with correct default values
    let source = "";
    let line_index = LineIndex::new(source);
    let visitor = FrameworkAwareVisitor::new(&line_index);

    assert!(
        !visitor.is_framework_file,
        "Should default to non-framework file"
    );
    assert!(
        visitor.framework_decorated_lines.is_empty(),
        "Should have empty decorated lines set"
    );
    assert!(
        visitor.detected_frameworks.is_empty(),
        "Should have empty detected frameworks set"
    );
    assert!(
        visitor.framework_references.is_empty(),
        "Should have empty framework references"
    );
}

#[test]
fn test_multiple_decorators() {
    // Test handling multiple decorators on a single function
    let source = r#"
from flask import Flask
from flask_login import login_required
app = Flask(__name__)

@app.route('/users')
@login_required
@cache.cached(timeout=60)
def get_users():
    return []
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    assert!(visitor.is_framework_file, "Should detect framework file");
    // The function is on line 6 (after decorators)
    assert!(
        !visitor.framework_decorated_lines.is_empty(),
        "Should have decorated lines"
    );
}

#[test]
fn test_complex_decorator_patterns() {
    // Test complex route patterns with URL parameters and methods
    let source = r#"
from flask import Flask
from fastapi import APIRouter
app = Flask(__name__)
router = APIRouter()

@app.route('/api/v1/users/<int:user_id>', methods=['GET', 'POST'])
def user_endpoint(user_id):
    return {}

@router.get('/items/{item_id}')
async def get_item(item_id: int):
    return {}
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    assert!(visitor.is_framework_file, "Should detect framework file");
    assert!(
        visitor.framework_decorated_lines.len() >= 2,
        "Should detect both decorated endpoints"
    );
}

#[test]
fn test_decorated_endpoint_confidence_is_one() {
    // Framework endpoints with decorators should return confidence = 100 (1.0 * 100)
    let source = r#"
from flask import Flask, route

app = Flask(__name__)

@app.route("/")
def get_users():
    pass
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    // Verify this is a framework file with decorated lines
    assert!(visitor.is_framework_file, "Should be a framework file");
    assert!(
        !visitor.framework_decorated_lines.is_empty(),
        "Should have decorated lines"
    );

    // Find the decorated line (should be the function definition line)
    let decorated_line = *visitor.framework_decorated_lines.iter().next().unwrap();

    let result = detect_framework_usage(decorated_line, "get_users", "function", Some(&visitor));
    assert_eq!(
        result,
        Some(100),
        "Decorated endpoint should have confidence 100"
    );
}

#[test]
fn test_undecorated_function_in_framework_file_returns_none() {
    // Helper functions without decorators in framework files should return None
    let source = r#"
from flask import Flask

def helper_function():
    pass

@app.route('/')
def index():
    pass
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    // helper_function is on line 4, not decorated
    let result = detect_framework_usage(4, "helper_function", "function", Some(&visitor));
    assert_eq!(result, None, "Undecorated helper should return None");
}

#[test]
fn test_private_function_in_framework_file_returns_none() {
    // Private functions (starting with _) should be ignored
    let source = r#"
from flask import Flask

def _private_function():
    pass
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    let result = detect_framework_usage(4, "_private_function", "function", Some(&visitor));
    assert_eq!(result, None, "Private function should return None");
}

#[test]
fn test_non_framework_file_returns_none() {
    // Functions in non-framework files should return None
    let source = r"
import os
import sys

def regular_function():
    pass
";
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    assert!(!visitor.is_framework_file, "Should not be a framework file");
    let result = detect_framework_usage(5, "regular_function", "function", Some(&visitor));
    assert_eq!(
        result, None,
        "Non-framework file function should return None"
    );
}

#[test]
fn test_no_visitor_returns_none() {
    // When no visitor is provided, should return None
    let result = detect_framework_usage(10, "some_function", "function", None);
    assert_eq!(result, None, "No visitor should return None");
}
