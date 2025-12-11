//! Tests for method usage in various contexts (self, cls, etc.).

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_method_context() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r#"
class MyClass:
    def helper(self):
        return "helper"

    def main(self):
        self.helper()
"#;
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path()).unwrap();

    let unused_methods: Vec<String> = result
        .unused_methods
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    // 'helper' should be used because 'self.helper()' is called inside 'main'
    assert!(
        !unused_methods.contains(&"helper".to_owned()),
        "MyClass.helper should be used via self.helper()"
    );
}

#[test]
fn test_nested_class_method_call() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
class Outer:
    class Inner:
        def inner_helper(self):
            pass
        
        def inner_main(self):
            self.inner_helper()
";
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path()).unwrap();

    let unused_methods: Vec<String> = result
        .unused_methods
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(
        !unused_methods.contains(&"inner_helper".to_owned()),
        "Inner.inner_helper should be used"
    );
}

#[test]
fn test_inheritance_method_call() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
class Parent:
    def parent_method(self):
        pass

class Child(Parent):
    def child_method(self):
        self.parent_method()
";
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path()).unwrap();

    let unused_methods: Vec<String> = result
        .unused_methods
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(
        !unused_methods.contains(&"parent_method".to_owned()),
        "Parent.parent_method should be used via self.parent_method in Child"
    );
}

#[test]
fn test_static_and_class_methods() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
class MyClass:
    @staticmethod
    def static_func():
        pass

    @classmethod
    def class_func(cls):
        cls.static_func()

    def instance_func(self):
        self.class_func()
";
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path()).unwrap();

    let unused_methods: Vec<String> = result
        .unused_methods
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(
        !unused_methods.contains(&"static_func".to_owned()),
        "static_func should be used via cls.static_func"
    );
    assert!(
        !unused_methods.contains(&"class_func".to_owned()),
        "class_func should be used via self.class_func"
    );
}
