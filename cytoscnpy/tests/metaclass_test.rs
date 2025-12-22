//! Tests for metaclass keyword argument detection.
//!
//! Bug #5: The `ClassDef` handler was not visiting `node.keywords`,
//! so classes used as metaclasses were incorrectly flagged as unused.
#![allow(clippy::unwrap_used)]
#![allow(clippy::str_to_string)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::needless_raw_string_hashes)]

use cytoscnpy::utils::LineIndex;
use cytoscnpy::visitor::CytoScnPyVisitor;
use ruff_python_parser::{parse, Mode};
use std::collections::HashSet;
use std::path::PathBuf;

/// Macro to visit code and get visitor with definitions and references
macro_rules! visit_code {
    ($code:expr, $visitor:ident) => {
        let line_index = LineIndex::new($code);
        let mut $visitor =
            CytoScnPyVisitor::new(PathBuf::from("test.py"), "test".to_string(), &line_index);
        let ast = parse($code, Mode::Module.into()).unwrap();
        if let ruff_python_ast::Mod::Module(module) = ast.into_syntax() {
            for stmt in module.body {
                $visitor.visit_stmt(&stmt);
            }
        }
    };
}

/// Helper to check if a class is referenced (i.e., used as metaclass)
fn is_class_referenced(code: &str, class_name: &str) -> bool {
    let line_index = LineIndex::new(code);
    let mut visitor =
        CytoScnPyVisitor::new(PathBuf::from("test.py"), "test".to_string(), &line_index);
    let ast = parse(code, Mode::Module.into()).unwrap();
    if let ruff_python_ast::Mod::Module(module) = ast.into_syntax() {
        for stmt in module.body {
            visitor.visit_stmt(&stmt);
        }
    }

    // Check if the class name appears in references
    visitor.references.contains_key(class_name)
        || visitor
            .references
            .contains_key(&format!("test.{}", class_name))
}

/// Helper to get all referenced names
fn get_references(code: &str) -> HashSet<String> {
    let line_index = LineIndex::new(code);
    let mut visitor =
        CytoScnPyVisitor::new(PathBuf::from("test.py"), "test".to_string(), &line_index);
    let ast = parse(code, Mode::Module.into()).unwrap();
    if let ruff_python_ast::Mod::Module(module) = ast.into_syntax() {
        for stmt in module.body {
            visitor.visit_stmt(&stmt);
        }
    }
    visitor.references.keys().cloned().collect()
}

// =============================================================================
// Basic Metaclass Tests
// =============================================================================

#[test]
fn test_metaclass_basic_usage_is_referenced() {
    let code = r#"
class Meta(type):
    pass

class MyClass(metaclass=Meta):
    pass
"#;
    assert!(
        is_class_referenced(code, "Meta"),
        "Meta should be referenced - it's used as metaclass"
    );
}

#[test]
fn test_metaclass_with_body_is_referenced() {
    let code = r#"
class Meta(type):
    def __new__(cls, name, bases, dct):
        x = super().__new__(cls, name, bases, dct)
        x.attr = 100
        return x

class MyClass(metaclass=Meta):
    pass
"#;
    assert!(
        is_class_referenced(code, "Meta"),
        "Meta with body should be referenced"
    );
}

#[test]
fn test_unused_metaclass_not_referenced() {
    let code = r#"
class UnusedMeta(type):
    pass

class SomeClass:
    pass
"#;
    assert!(
        !is_class_referenced(code, "UnusedMeta"),
        "UnusedMeta should NOT be referenced - never used as metaclass"
    );
}

// =============================================================================
// Singleton Pattern Tests
// =============================================================================

#[test]
fn test_singleton_metaclass_is_referenced() {
    let code = r#"
class SingletonMeta(type):
    _instances = {}
    
    def __call__(cls, *args, **kwargs):
        if cls not in cls._instances:
            cls._instances[cls] = super().__call__(*args, **kwargs)
        return cls._instances[cls]

class Singleton(metaclass=SingletonMeta):
    pass
"#;
    assert!(
        is_class_referenced(code, "SingletonMeta"),
        "SingletonMeta should be referenced"
    );
}

// =============================================================================
// Multiple Keywords Tests
// =============================================================================

#[test]
fn test_metaclass_with_base_class_and_keyword() {
    let code = r#"
class BaseMeta(type):
    pass

class Base:
    pass

class Child(Base, metaclass=BaseMeta):
    pass
"#;
    assert!(
        is_class_referenced(code, "BaseMeta"),
        "BaseMeta should be referenced - used as metaclass with inheritance"
    );
    assert!(
        is_class_referenced(code, "Base"),
        "Base should be referenced - used as base class"
    );
}

// =============================================================================
// Nested and Complex Patterns
// =============================================================================

#[test]
fn test_multiple_classes_same_metaclass() {
    let code = r#"
class SharedMeta(type):
    pass

class ClassA(metaclass=SharedMeta):
    pass

class ClassB(metaclass=SharedMeta):
    pass

class ClassC(metaclass=SharedMeta):
    pass
"#;
    let refs = get_references(code);
    // SharedMeta should be referenced multiple times
    assert!(
        refs.contains("SharedMeta") || refs.contains("test.SharedMeta"),
        "SharedMeta should be referenced - used by multiple classes"
    );
}

// =============================================================================
// Real-world Pattern: Django-style Metaclass
// =============================================================================

#[test]
fn test_django_style_model_metaclass() {
    let code = r#"
class ModelBase(type):
    def __new__(mcs, name, bases, namespace):
        cls = super().__new__(mcs, name, bases, namespace)
        return cls

class Model(metaclass=ModelBase):
    pass

class User(Model):
    name: str
"#;
    assert!(
        is_class_referenced(code, "ModelBase"),
        "ModelBase should be referenced - Django-style metaclass pattern"
    );
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_metaclass_only_keyword_no_bases() {
    let code = r#"
class JustMeta(type):
    pass

class NoBase(metaclass=JustMeta):
    pass
"#;
    assert!(
        is_class_referenced(code, "JustMeta"),
        "JustMeta should be referenced even without base classes"
    );
}

#[test]
fn test_visitor_tracks_metaclass_reference() {
    let code = r#"
class Meta(type):
    pass

class MyClass(metaclass=Meta):
    pass
"#;
    visit_code!(code, visitor);

    // Verify Meta is in references
    let ref_names: HashSet<String> = visitor.references.keys().cloned().collect();
    assert!(
        ref_names.contains("Meta") || ref_names.contains("test.Meta"),
        "Meta should appear in references. Found: {:?}",
        ref_names
    );
}

#[test]
fn test_both_base_and_metaclass_tracked() {
    let code = r#"
class ParentMeta(type):
    pass

class Parent:
    pass

class Child(Parent, metaclass=ParentMeta):
    pass
"#;
    visit_code!(code, visitor);

    let ref_names: HashSet<String> = visitor.references.keys().cloned().collect();

    // Both Parent (base class) and ParentMeta (metaclass) should be referenced
    assert!(
        ref_names.contains("Parent") || ref_names.contains("test.Parent"),
        "Parent should be in references"
    );
    assert!(
        ref_names.contains("ParentMeta") || ref_names.contains("test.ParentMeta"),
        "ParentMeta should be in references"
    );
}
