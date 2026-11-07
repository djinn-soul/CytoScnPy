use super::*;

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
