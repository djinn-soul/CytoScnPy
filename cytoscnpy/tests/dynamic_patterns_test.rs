//! Tests for dynamic pattern detection (eval, exec, globals).

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_globals_access() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r#"
def unused_but_dynamic():
    pass

# Dynamic access should mark it as used
g = globals()
g["unused_but_dynamic"]()
"#;
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(100).with_tests(false);
    let result = cytoscnpy.analyze(dir.path()).unwrap();

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(
        !unused_funcs.contains(&"unused_but_dynamic".to_owned()),
        "Globals access failed to mark function as used"
    );
}

#[test]
fn test_hasattr_usage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r#"
class MyClass:
    def unused_method(self):
        pass

obj = MyClass()
# hasattr check should mark 'unused_method' as referenced
if hasattr(obj, "unused_method"):
    print("exists")
"#;
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(100).with_tests(false);
    let result = cytoscnpy.analyze(dir.path()).unwrap();

    let unused_methods: Vec<String> = result
        .unused_methods
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(
        !unused_methods.contains(&"unused_method".to_owned()),
        "Hasattr failed to mark method as used"
    );
}

#[test]
fn test_eval_exec_dynamic_marking() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r#"
def hidden_func():
    pass

# eval makes the module dynamic, potentially using anything
eval("hidden_func()")
"#;
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(100).with_tests(false);
    let result = cytoscnpy.analyze(dir.path()).unwrap();

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(
        !unused_funcs.contains(&"hidden_func".to_owned()),
        "Eval failed to mark function as used"
    );
}
