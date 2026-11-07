use super::*;

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
