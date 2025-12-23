//! Tests for framework detection (Flask, Django, etc.).
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::str_to_string)]
#![allow(clippy::needless_raw_string_hashes)]

use cytoscnpy::framework::FrameworkAwareVisitor;
use cytoscnpy::utils::LineIndex;
use ruff_python_parser::{parse, Mode};

fn run_framework_visitor<'a>(source: &str, line_index: &'a LineIndex) -> FrameworkAwareVisitor<'a> {
    let tree = parse(source, Mode::Module.into()).expect("Failed to parse");
    let mut visitor = FrameworkAwareVisitor::new(line_index);

    if let ruff_python_ast::Mod::Module(module) = tree.into_syntax() {
        for stmt in &module.body {
            visitor.visit_stmt(stmt);
        }
    }
    visitor
}

#[test]
fn test_flask_detection() {
    let source = r#"
from flask import Flask, route

app = Flask(__name__)

@app.route("/")
def index():
    pass
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    assert!(visitor.is_framework_file, "Should detect Flask file");
    assert!(
        visitor.detected_frameworks.contains("flask"),
        "Should detect 'flask' framework"
    );
    assert!(
        !visitor.framework_decorated_lines.is_empty(),
        "Should find decorated lines"
    );
}

#[test]
fn test_django_detection() {
    let source = r"
from django.db import models
from django.contrib.auth.decorators import login_required

class MyModel(models.Model):
    pass

@login_required
def my_view(request):
    pass
";
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    assert!(visitor.is_framework_file, "Should detect Django file");
    assert!(
        visitor.detected_frameworks.contains("django"),
        "Should detect 'django' framework"
    );
    assert!(
        visitor.framework_decorated_lines.len() >= 2,
        "Should detect at least 2 framework constructs"
    );
}

#[test]
fn test_fastapi_detection() {
    let source = r#"
from fastapi import FastAPI

app = FastAPI()

@app.get("/items/{item_id}")
def read_item(item_id: int):
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
        !visitor.framework_decorated_lines.is_empty(),
        "Should find decorated lines"
    );
}

#[test]
fn test_framework_imports_only() {
    let source = r"
import flask
import django
";
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);
    assert!(visitor.is_framework_file);
    assert!(visitor.detected_frameworks.contains("flask"));
    assert!(visitor.detected_frameworks.contains("django"));
}

#[test]
fn test_no_framework() {
    let source = r"
import os
import sys

def normal_func():
    pass
";
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);
    assert!(!visitor.is_framework_file);
    assert!(visitor.detected_frameworks.is_empty());
    assert!(visitor.framework_decorated_lines.is_empty());
}

// ============================================================================
// Django-Specific Pattern Tests
// ============================================================================

#[test]
fn test_django_urlpatterns_detection() {
    let source = r#"
from django.urls import path
from . import views

def my_view(request):
    pass

def another_view(request):
    pass

urlpatterns = [
    path('home/', my_view),
    path('about/', another_view),
]
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    assert!(visitor.is_framework_file, "Should detect Django file");
    assert!(
        visitor.detected_frameworks.contains("django"),
        "Should detect 'django' framework"
    );
    assert!(
        visitor
            .framework_references
            .contains(&"my_view".to_string()),
        "Should extract my_view from urlpatterns"
    );
    assert!(
        visitor
            .framework_references
            .contains(&"another_view".to_string()),
        "Should extract another_view from urlpatterns"
    );
}

#[test]
fn test_django_urlpatterns_class_based_view() {
    let source = r#"
from django.urls import path
from django.views.generic import TemplateView

class HomeView:
    pass

urlpatterns = [
    path('home/', HomeView.as_view()),
]
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    assert!(visitor.is_framework_file, "Should detect Django file");
    assert!(
        visitor
            .framework_references
            .contains(&"HomeView".to_string()),
        "Should extract HomeView from class-based view pattern"
    );
}

#[test]
fn test_django_admin_register() {
    let source = r#"
from django.contrib import admin
from .models import MyModel, AnotherModel

class MyModel:
    pass

class AnotherModel:
    pass

admin.site.register(MyModel)
admin.site.register(AnotherModel)
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    assert!(visitor.is_framework_file, "Should detect Django file");
    assert!(
        visitor.detected_frameworks.contains("django"),
        "Should detect 'django' framework via admin.register"
    );
    assert!(
        visitor
            .framework_references
            .contains(&"MyModel".to_string()),
        "Should extract MyModel from admin.site.register"
    );
    assert!(
        visitor
            .framework_references
            .contains(&"AnotherModel".to_string()),
        "Should extract AnotherModel from admin.site.register"
    );
}

#[test]
fn test_django_signal_connect() {
    let source = r#"
from django.db.models.signals import pre_save, post_save
from django.dispatch import receiver

def my_pre_save_handler(sender, **kwargs):
    pass

def my_post_save_handler(sender, **kwargs):
    pass

pre_save.connect(my_pre_save_handler)
post_save.connect(my_post_save_handler)
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    assert!(visitor.is_framework_file, "Should detect Django file");
    assert!(
        visitor
            .framework_references
            .contains(&"my_pre_save_handler".to_string()),
        "Should extract pre_save receiver"
    );
    assert!(
        visitor
            .framework_references
            .contains(&"my_post_save_handler".to_string()),
        "Should extract post_save receiver"
    );
}

#[test]
fn test_django_all_patterns_combined() {
    let source = r#"
from django.urls import path
from django.contrib import admin
from django.db.models.signals import post_save

def view_func(request):
    pass

class MyModel:
    pass

def signal_handler(sender, **kwargs):
    pass

urlpatterns = [
    path('api/', view_func),
]

admin.site.register(MyModel)
post_save.connect(signal_handler)
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    assert!(visitor.is_framework_file, "Should detect Django file");
    assert_eq!(
        visitor.framework_references.len(),
        3,
        "Should find all 3 Django references: view_func, MyModel, signal_handler"
    );
    assert!(visitor
        .framework_references
        .contains(&"view_func".to_string()));
    assert!(visitor
        .framework_references
        .contains(&"MyModel".to_string()));
    assert!(visitor
        .framework_references
        .contains(&"signal_handler".to_string()));
}

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

// ============================================================================
// TestDetectFrameworkUsage (6 tests)
// ============================================================================

use cytoscnpy::framework::detect_framework_usage;

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

#[test]
fn test_non_function_in_framework_file_returns_none() {
    // Variables and other non-function definitions should return None
    let source = r#"
from flask import Flask

app = Flask(__name__)
"#;
    let line_index = LineIndex::new(source);
    let visitor = run_framework_visitor(source, &line_index);

    let result = detect_framework_usage(4, "app", "variable", Some(&visitor));
    assert_eq!(result, None, "Variable should return None");

    let result = detect_framework_usage(4, "MyClass", "class", Some(&visitor));
    assert_eq!(result, None, "Class should return None");
}

// ============================================================================
// TestFrameworkPatterns (3 tests)
// ============================================================================

use cytoscnpy::framework::{get_framework_imports, FRAMEWORK_DECORATORS, FRAMEWORK_FUNCTIONS};

#[test]
fn test_framework_decorators_list() {
    // Validate that common decorators are in the list
    assert!(
        FRAMEWORK_DECORATORS.contains(&"@*.route"),
        "Should contain @*.route"
    );
    assert!(
        FRAMEWORK_DECORATORS.contains(&"@*.get"),
        "Should contain @*.get"
    );
    assert!(
        FRAMEWORK_DECORATORS.contains(&"@login_required"),
        "Should contain @login_required"
    );
    assert!(
        FRAMEWORK_DECORATORS.contains(&"@permission_required"),
        "Should contain @permission_required"
    );
}

#[test]
fn test_framework_functions_list() {
    // Validate that common framework functions are in the list
    assert!(FRAMEWORK_FUNCTIONS.contains(&"get"), "Should contain 'get'");
    assert!(
        FRAMEWORK_FUNCTIONS.contains(&"post"),
        "Should contain 'post'"
    );
    assert!(
        FRAMEWORK_FUNCTIONS.contains(&"*_queryset"),
        "Should contain '*_queryset'"
    );
    assert!(
        FRAMEWORK_FUNCTIONS.contains(&"get_context_data"),
        "Should contain 'get_context_data'"
    );
}

#[test]
fn test_framework_imports_set() {
    // Validate that common framework imports are in the set
    let imports = get_framework_imports();
    assert!(imports.contains("flask"), "Should contain 'flask'");
    assert!(imports.contains("django"), "Should contain 'django'");
    assert!(imports.contains("fastapi"), "Should contain 'fastapi'");
    assert!(imports.contains("pydantic"), "Should contain 'pydantic'");
}
