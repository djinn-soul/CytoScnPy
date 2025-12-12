//! Test suite for the analyzer module.

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_analyze_basic() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r#"
def used_function():
    return "used"

def unused_function():
    return "unused"

class UsedClass:
    def method(self):
        pass

class UnusedClass:
    def method(self):
        pass

result = used_function()
instance = UsedClass()
"#;
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path()).unwrap();

    // Verify unused functions
    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();
    assert!(unused_funcs.contains(&"unused_function".to_owned()));
    assert!(!unused_funcs.contains(&"used_function".to_owned()));

    // Verify unused classes
    let unused_classes: Vec<String> = result
        .unused_classes
        .iter()
        .map(|c| c.simple_name.clone())
        .collect();
    assert!(unused_classes.contains(&"UnusedClass".to_owned()));
    assert!(!unused_classes.contains(&"UsedClass".to_owned()));

    // Verify summary
    assert_eq!(result.analysis_summary.total_files, 1);
}

#[test]
fn test_analyze_empty_directory() {
    let dir = tempdir().unwrap();
    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path()).unwrap();

    assert_eq!(result.analysis_summary.total_files, 0);
    assert!(result.unused_functions.is_empty());
    assert!(result.unused_classes.is_empty());
}

#[test]
fn test_confidence_threshold_filtering() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    // _private is penalized, so its confidence should be lower
    let content = r"
def regular_unused():
    pass

def _private_unused():
    pass
";
    write!(file, "{content}").unwrap();

    // High threshold: _private_unused should be filtered out (low confidence)
    // regular_unused (100) should remain
    // _private_unused (100 - 80 = 20)

    // Set threshold to 30
    let mut cytoscnpy_high = CytoScnPy::default().with_confidence(30).with_tests(false);
    let result_high = cytoscnpy_high.analyze(dir.path()).unwrap();

    let funcs_high: Vec<String> = result_high
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(funcs_high.contains(&"regular_unused".to_owned()));
    assert!(!funcs_high.contains(&"_private_unused".to_owned()));

    // Low threshold: both should be present
    let mut cytoscnpy_low = CytoScnPy::default().with_confidence(10).with_tests(false);
    let result_low = cytoscnpy_low.analyze(dir.path()).unwrap();

    let funcs_low: Vec<String> = result_low
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(funcs_low.contains(&"regular_unused".to_owned()));
    assert!(funcs_low.contains(&"_private_unused".to_owned()));
}

#[test]
fn test_module_name_generation_implicit() {
    let dir = tempdir().unwrap();

    // Create src/package/submodule.py
    let package_path = dir.path().join("src").join("package");
    fs::create_dir_all(&package_path).unwrap();

    let file_path = package_path.join("submodule.py");
    let mut file = File::create(&file_path).unwrap();
    write!(file, "def regular_func(): pass").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(0).with_tests(false);
    let result = cytoscnpy.analyze(dir.path()).unwrap();

    // We can't check internal module name directly, but we can check if full_name reflects it?
    // In Rust impl, module name is just file_stem (e.g. "submodule"), not dotted path "src.package.submodule"
    // So the full name would be "submodule.regular_func" or "regular_func" if module name is ignored in some contexts.
    // Let's check what we get.

    if let Some(func) = result.unused_functions.first() {
        // Based on analyzer.rs: module name is now full dotted path "src.package.submodule"
        assert_eq!(func.full_name, "src.package.submodule.regular_func");
    } else {
        panic!("No unused function found");
    }
}

#[test]
fn test_heuristics_auto_called_methods() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r#"
class MyClass:
    def __init__(self):
        pass

    def __str__(self):
        return "string"

instance = MyClass()
"#;
    write!(file, "{content}").unwrap();

    // Dunder methods (__init__, __str__) get confidence=0 from the dunder/AUTO_CALLED penalty.
    // With threshold=1, they are filtered out (0 < 1), so they won't appear in unused list.
    // This is correct: these methods are implicitly called by Python, not truly unused.

    let mut cytoscnpy = CytoScnPy::default().with_confidence(1).with_tests(false);
    let result = cytoscnpy.analyze(dir.path()).unwrap();

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    // Auto-called methods should be ignored (filtered out) because they are implicitly used.
    assert!(!unused_funcs.contains(&"__init__".to_owned()));
    assert!(!unused_funcs.contains(&"__str__".to_owned()));
}

#[test]
fn test_mark_exports_in_init() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("__init__.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
def public_function():
    pass

def _private_function():
    pass
";
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(0).with_tests(false);
    let result = cytoscnpy.analyze(dir.path()).unwrap();

    // In Rust impl: "In __init__.py penalty ... confidence -= 20"
    // And "Private names ... confidence -= 30"

    let public_def = result
        .unused_functions
        .iter()
        .find(|f| f.simple_name == "public_function")
        .unwrap();
    assert!(public_def.in_init);
    // Base 100 - 15 = 85
    assert_eq!(public_def.confidence, 85);

    let private_def = result
        .unused_functions
        .iter()
        .find(|f| f.simple_name == "_private_function")
        .unwrap();
    // Base 100 - 80 (private) - 15 (init) = 5
    assert_eq!(private_def.confidence, 5);
}

#[test]
fn test_mark_refs_direct_reference() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
def my_func():
    pass

my_func()
";
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(0).with_tests(false);
    let result = cytoscnpy.analyze(dir.path()).unwrap();

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(!unused_funcs.contains(&"my_func".to_owned()));
}
