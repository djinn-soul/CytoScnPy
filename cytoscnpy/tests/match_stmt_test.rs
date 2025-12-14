//! Tests for match statement analysis.

use cytoscnpy::analyzer::CytoScnPy;
use std::path::PathBuf;

#[test]
fn test_match_statement() {
    let code = r#"
def handle_point(point):
    match point:
        case (0, 0):
            print("Origin")
        case (0, y):
            print(f"Y={y}")
        case (x, 0):
            print(f"X={x}")
        case (x, y):
            print(f"X={x}, Y={y}")
        case _:
            print("Not a point")

def handle_class(point):
    match point:
        case Point(x=0, y=0):
            print("Origin")
        case Point(x=0, y=y):
            print(f"Y={y}")
        case Point(x=x, y=0):
            print(f"X={x}")
        case Point(x=x, y=y):
            print(f"X={x}, Y={y}")
        case _:
            print("Not a point")

def handle_sequence(seq):
    match seq:
        case [x, y, *rest]:
            print(f"x={x}, y={y}, rest={rest}")
        case _:
            print("Not a sequence")

def handle_mapping(mapping):
    match mapping:
        case {"name": name, "age": age}:
            print(f"Name: {name}, Age: {age}")
        case {"id": id, **rest}:
            print(f"ID: {id}, Rest: {rest}")
        case _:
            print("Not a mapping")

def handle_or(item):
    match item:
        case 0 | 1 | 2:
            print("Small number")
        case x:
            print(f"Other: {x}")

def unused_variable_in_match(item):
    match item:
        case [x, y, unused_y]:
            print(f"x={x}, y={y}")
"#;

    let analyzer = CytoScnPy::default().with_confidence(100).with_tests(false);
    let report = analyzer.analyze_code(code, PathBuf::from("match_example.py"));

    // Check that we found no errors for valid usages
    let unused_vars: Vec<_> = report
        .unused_variables
        .iter()
        .map(|v| v.simple_name.as_str())
        .collect();

    // 'other' is used in print
    assert!(!unused_vars.contains(&"other"));

    // 'y' in handle_point is used
    assert!(!unused_vars.contains(&"y"));

    // 'x' in handle_point is used
    assert!(!unused_vars.contains(&"x"));

    // 'x', 'y' in handle_class are used
    // Note: 'x' and 'y' appear multiple times in different scopes,
    // but our analyzer should track them as used in their respective scopes.

    // 'rest' in handle_sequence is used
    assert!(!unused_vars.contains(&"rest"));

    // 'name', 'age' in handle_mapping are used
    assert!(!unused_vars.contains(&"name"));
    assert!(!unused_vars.contains(&"age"));

    // 'id', 'rest' in handle_mapping are used
    assert!(!unused_vars.contains(&"id"));

    // 'x' in handle_or is used
    assert!(!unused_vars.contains(&"x"));

    // Check that we DO find the unused variable 'y' in unused_variable_in_match
    // The variable name will be qualified, e.g., unused_variable_in_match.y
    // But our simple check above uses simple_name.
    // Let's check specifically for the one we expect to be unused.

    println!(
        "Unused variables found: {:?}",
        report
            .unused_variables
            .iter()
            .map(|v| &v.full_name)
            .collect::<Vec<_>>()
    );

    let found_unused_y = report.unused_variables.iter().any(|v| {
        v.simple_name == "unused_y"
            && v.full_name == "match_example.unused_variable_in_match.unused_y"
    });
    assert!(
        found_unused_y,
        "Should detect unused variable 'unused_y' in match pattern"
    );
}


