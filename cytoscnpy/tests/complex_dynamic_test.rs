//! Tests for complex dynamic Python patterns.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_complex_cross_file_dynamic_chain() {
    // Scenario:
    // lib.py: Defines 'hidden_gem' (unused statically).
    // middle.py: Imports 'hidden_gem'. Uses 'globals()["hidden_gem"]()'.
    // main.py: Calls 'middle.runner()'.
    //
    // Expected Chain:
    // 1. main.py uses middle.runner.
    // 2. middle.py has `globals()`, so it's marked dynamic.
    // 3. In a dynamic file, ALL definitions (including imports) are marked used.
    // 4. So `from lib import hidden_gem` is used.
    // 5. This reference propagates to `lib.py`.
    // 6. `lib.hidden_gem` is marked used.

    let dir = tempdir().unwrap();
    let src_path = dir.path().join("src");
    fs::create_dir_all(&src_path).unwrap();

    // 1. lib.py
    let lib_path = src_path.join("lib.py");
    let mut lib_file = File::create(&lib_path).unwrap();
    write!(lib_file, "def hidden_gem(): pass").unwrap();

    // 2. middle.py
    let middle_path = src_path.join("middle.py");
    let mut middle_file = File::create(&middle_path).unwrap();
    write!(
        middle_file,
        r#"
from src.lib import hidden_gem

def runner():
    # Dynamic usage of the imported name
    g = globals()
    g["hidden_gem"]()
"#
    )
    .unwrap();

    // 3. main.py
    let main_path = dir.path().join("main.py");
    let mut main_file = File::create(&main_path).unwrap();
    write!(
        main_file,
        r#"
from src import middle

if __name__ == "__main__":
    middle.runner()
"#
    )
    .unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(100).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    // Check lib.hidden_gem
    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.full_name.clone())
        .collect();

    // Debug output if it fails
    if unused_funcs.iter().any(|f| f.contains("hidden_gem")) {
        println!("Unused functions: {unused_funcs:?}");
    }

    assert!(
        !unused_funcs.contains(&"src.lib.hidden_gem".to_owned()),
        "lib.hidden_gem should be used via dynamic chain"
    );
}

#[test]
fn test_hasattr_cross_file() {
    // Scenario:
    // models.py: Defines 'User' class with 'save' method.
    // processor.py: Imports 'User'. Checks 'hasattr(user, "save")'.
    //
    // Expected:
    // 'User.save' should be marked as used.

    let dir = tempdir().unwrap();

    // models.py
    let models_path = dir.path().join("models.py");
    let mut models_file = File::create(&models_path).unwrap();
    write!(
        models_file,
        r"
class User:
    def save(self):
        pass
    
    def delete(self):
        pass
"
    )
    .unwrap();

    // processor.py
    let proc_path = dir.path().join("processor.py");
    let mut proc_file = File::create(&proc_path).unwrap();
    write!(
        proc_file,
        r#"
from models import User

def process():
    u = User()
    if hasattr(u, "save"):
        u.save()
"#
    )
    .unwrap();

    // main.py (entry point)
    let main_path = dir.path().join("main.py");
    let mut main_file = File::create(&main_path).unwrap();
    write!(main_file, "import processor; processor.process()").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(100).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_method_names: Vec<String> = result
        .unused_methods
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(
        !unused_method_names.contains(&"save".to_owned()),
        "User.save should be used via hasattr"
    );
    assert!(
        unused_method_names.contains(&"delete".to_owned()),
        "User.delete should be unused"
    );
}

#[test]
fn test_eval_local_scope_usage() {
    // Scenario:
    // A function defines local variables that are only used inside `eval`.
    // Without dynamic tracking, these would be reported as unused variables.

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("script.py");
    let mut file = File::create(&file_path).unwrap();
    write!(
        file,
        r#"
def calculate(expression):
    # 'x' and 'y' are defined but not statically used
    x = 10
    y = 20
    # eval makes the file dynamic, so all locals should be considered used
    return eval(expression)

calculate("x + y")
"#
    )
    .unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(100).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_vars: Vec<String> = result
        .unused_variables
        .iter()
        .map(|v| v.simple_name.clone())
        .collect();

    assert!(
        !unused_vars.contains(&"x".to_owned()),
        "Local 'x' should be used due to eval"
    );
    assert!(
        !unused_vars.contains(&"y".to_owned()),
        "Local 'y' should be used due to eval"
    );
}
