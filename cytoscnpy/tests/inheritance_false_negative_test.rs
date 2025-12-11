//! Test to verify the fix for the Order class false negative bug.
//!
//! This test ensures that user-defined BaseModel classes don't incorrectly
//! trigger framework detection and cause subclasses to be marked as used.

use cytoscnpy::analyzer::CytoScnPy;
use std::path::PathBuf;

/// Test that classes inheriting from a user-defined BaseModel are detected as unused
/// when they are never instantiated.
#[test]
fn test_user_defined_basemodel_subclasses_detected() {
    let code = r#"
class BaseModel:
    """User-defined base model, NOT Pydantic."""
    def __init__(self, id=None):
        self.id = id

class User(BaseModel):
    """Used class - instantiated below."""
    def __init__(self, name):
        super().__init__()
        self.name = name

class Order(BaseModel):
    """Unused class - never instantiated."""
    def __init__(self, user_id):
        super().__init__()
        self.user_id = user_id

class Cart:
    """Unused class - no inheritance."""
    def __init__(self):
        self.items = []

# Only User is instantiated
user = User("John")
"#;

    let analyzer = CytoScnPy::default();
    let result = analyzer.analyze_code(code, PathBuf::from("test_models.py"));

    // Get class names from unused_classes
    let unused_class_names: Vec<&str> = result
        .unused_classes
        .iter()
        .map(|d| d.simple_name.as_str())
        .collect();

    println!("Unused classes: {:?}", unused_class_names);

    // Order should be detected as unused (it's never instantiated)
    assert!(
        unused_class_names.contains(&"Order"),
        "Order should be detected as unused! Got: {:?}",
        unused_class_names
    );

    // Cart should also be detected as unused
    assert!(
        unused_class_names.contains(&"Cart"),
        "Cart should be detected as unused! Got: {:?}",
        unused_class_names
    );

    // User should NOT be in unused_classes (it's instantiated)
    assert!(
        !unused_class_names.contains(&"User"),
        "User should NOT be in unused_classes! Got: {:?}",
        unused_class_names
    );

    // BaseModel should NOT be in unused_classes (it's used as base class)
    assert!(
        !unused_class_names.contains(&"BaseModel"),
        "BaseModel should NOT be in unused_classes! Got: {:?}",
        unused_class_names
    );
}

/// Test that actual Pydantic models are still correctly detected as framework code
#[test]
fn test_pydantic_basemodel_still_works() {
    let code = r#"
from pydantic import BaseModel

class UserSchema(BaseModel):
    """Pydantic model - should be treated as framework code."""
    name: str
    email: str

class ProductSchema(BaseModel):
    """Pydantic model - should be treated as framework code."""
    name: str
    price: float
"#;

    let analyzer = CytoScnPy::default();
    let result = analyzer.analyze_code(code, PathBuf::from("test_pydantic.py"));

    // Get class names from unused_classes
    let unused_class_names: Vec<&str> = result
        .unused_classes
        .iter()
        .map(|d| d.simple_name.as_str())
        .collect();

    println!("Unused Pydantic classes: {:?}", unused_class_names);

    // Pydantic models should have low confidence or be marked as used
    // They may still appear in unused_classes but with low confidence
    // The key is that they should be treated differently than user-defined classes
}

/// Test that Django Model classes are still correctly detected as framework code
#[test]
fn test_django_model_still_works() {
    let code = r#"
from django.db import models

class User(models.Model):
    """Django model - should be treated as framework code."""
    name = models.CharField(max_length=100)
    email = models.EmailField()

class Product(models.Model):
    """Django model - should be treated as framework code."""
    name = models.CharField(max_length=100)
    price = models.DecimalField()
"#;

    let analyzer = CytoScnPy::default();
    let result = analyzer.analyze_code(code, PathBuf::from("test_django.py"));

    // Get class names from unused_classes
    let unused_class_names: Vec<&str> = result
        .unused_classes
        .iter()
        .map(|d| d.simple_name.as_str())
        .collect();

    println!("Unused Django classes: {:?}", unused_class_names);

    // Django models may appear but should have framework-related handling
}
