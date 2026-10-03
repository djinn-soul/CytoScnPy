use super::*;
use std::path::PathBuf;

#[test]
fn test_no_duplicate_filenames() {
    let files = vec![
        PathBuf::from("src/foo.py"),
        PathBuf::from("src/bar.py"),
        PathBuf::from("src/baz.py"),
    ];
    let (count, worst, duplicates) = filenames::find_duplicate_filenames(&files);
    assert_eq!(count, 0);
    assert!(worst.is_none());
    assert!(duplicates.is_empty());
}

#[test]
fn test_duplicate_filenames_counted() {
    let files = vec![
        PathBuf::from("pkg_a/utils.py"),
        PathBuf::from("pkg_b/utils.py"),
        PathBuf::from("pkg_c/utils.py"),
        PathBuf::from("pkg_a/models.py"),
        PathBuf::from("pkg_b/models.py"),
        PathBuf::from("pkg_a/unique.py"),
    ];
    let (count, worst, duplicates) = filenames::find_duplicate_filenames(&files);
    assert_eq!(count, 2);
    assert_eq!(worst, Some(("utils.py".to_owned(), 3)));
    assert_eq!(duplicates.len(), 2);
    assert_eq!(duplicates[0].filename, "utils.py");
    assert_eq!(duplicates[0].count, 3);
    assert_eq!(duplicates[1].filename, "models.py");
    assert_eq!(duplicates[1].count, 2);
}

#[test]
fn test_init_py_excluded_from_duplicates() {
    let files = vec![
        PathBuf::from("pkg_a/__init__.py"),
        PathBuf::from("pkg_b/__init__.py"),
        PathBuf::from("pkg_c/__init__.py"),
    ];
    let (count, worst, duplicates) = filenames::find_duplicate_filenames(&files);
    assert_eq!(count, 0);
    assert!(worst.is_none());
    assert!(duplicates.is_empty());
}

fn func(name: &str, file: &str, line: usize) -> FunctionDefinition {
    FunctionDefinition {
        name: name.to_owned(),
        file: PathBuf::from(file),
        line,
    }
}

#[test]
fn test_function_collisions_threshold() {
    let funcs = vec![
        func("parse", "a.py", 10),
        func("parse", "b.py", 20),
        func("parse", "c.py", 30),
        func("process", "a.py", 50),
        func("process", "b.py", 60),
    ];

    let (count, worst, collisions) = functions::find_function_collisions(&funcs);
    assert_eq!(count, 1, "Only 'parse' appears in 3+ distinct files");
    assert_eq!(worst, Some(("parse".to_owned(), 3)));
    assert_eq!(collisions.len(), 1);
    assert_eq!(collisions[0].function_name, "parse");
    assert_eq!(collisions[0].file_count, 3);
}

#[test]
fn test_multiple_definitions_in_same_file_count_once_for_file_count() {
    let funcs = vec![
        func("calc", "a.py", 10),
        func("calc", "a.py", 25),
        func("calc", "b.py", 30),
    ];

    let (count, _, _) = functions::find_function_collisions(&funcs);
    assert_eq!(count, 0, "calc is only in 2 distinct files (a.py and b.py)");
}

#[test]
fn test_structural_dunder_methods_excluded() {
    let funcs = vec![
        func("__init__", "a.py", 1),
        func("__init__", "b.py", 1),
        func("__init__", "c.py", 1),
        func("setUp", "a.py", 5),
        func("setUp", "b.py", 5),
        func("setUp", "c.py", 5),
    ];

    let (count, worst, collisions) = functions::find_function_collisions(&funcs);
    assert_eq!(count, 0);
    assert!(worst.is_none());
    assert!(collisions.is_empty());
}

#[test]
fn test_generic_names_matching() {
    assert!(generic::is_generic_name("utils"));
    assert!(generic::is_generic_name("helpers"));
    assert!(generic::is_generic_name("common"));
    assert!(generic::is_generic_name("HANDLER"));
    assert!(!generic::is_generic_name("payment_utils"));
    assert!(!generic::is_generic_name("user_service"));
    assert!(!generic::is_generic_name("api_handler"));
}

#[test]
fn test_find_generic_names() {
    let files = vec![
        PathBuf::from("src/utils.py"),
        PathBuf::from("src/payment_utils.py"),
    ];
    let funcs = vec![
        FunctionDefinition {
            name: "handler".to_owned(),
            file: PathBuf::from("src/app.py"),
            line: 12,
        },
        FunctionDefinition {
            name: "calculate_tax".to_owned(),
            file: PathBuf::from("src/app.py"),
            line: 25,
        },
    ];

    let (count, entries) = generic::find_generic_names(&files, &funcs);
    assert_eq!(count, 2);
    assert_eq!(entries[0].category, GenericCategory::Filename);
    assert_eq!(entries[0].identifier, "utils");
    assert_eq!(entries[1].category, GenericCategory::Function);
    assert_eq!(entries[1].identifier, "handler");
}

#[test]
fn test_ast_function_extraction() {
    let code = r"
def top_level(x):
    return x + 1

class MyClass:
    def method_one(self):
        pass

    async def async_method(self):
        def nested_fn():
            return 42
        return nested_fn()
";
    let funcs = functions::extract_functions_from_source(code, &PathBuf::from("dummy.py"));
    let names: Vec<&str> = funcs.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"top_level"));
    assert!(names.contains(&"method_one"));
    assert!(names.contains(&"async_method"));
    assert!(names.contains(&"nested_fn"));
    assert_eq!(funcs.len(), 4);
}

#[test]
fn test_searchability_serialization() {
    let result = SearchabilityResult {
        stats: SearchabilityStats {
            total_files: 2,
            total_functions: 3,
            duplicate_filenames: 1,
            worst_duplicate_filename: Some(("utils.py".to_owned(), 2)),
            function_name_collisions: 0,
            worst_function_collision: None,
            generic_name_count: 1,
        },
        duplicate_files: vec![DuplicateFileEntry {
            filename: "utils.py".to_owned(),
            count: 2,
            paths: vec![PathBuf::from("a/utils.py"), PathBuf::from("b/utils.py")],
        }],
        function_collisions: vec![],
        generic_names: vec![GenericNameEntry {
            category: GenericCategory::Filename,
            identifier: "utils".to_owned(),
            file: PathBuf::from("a/utils.py"),
            line: None,
        }],
    };

    let json = serde_json::to_string(&result).expect("serialization must succeed");
    assert!(json.contains("\"duplicate_filenames\":1"));
    assert!(json.contains("utils.py"));
}

#[test]
fn test_conftest_py_and_duplicate_paths_deduped() {
    let files = vec![
        PathBuf::from("tests/unit/conftest.py"),
        PathBuf::from("tests/e2e/conftest.py"),
        PathBuf::from("pkg/service.py"),
        PathBuf::from("pkg/service.py"),
    ];
    let (count, worst, duplicates) = filenames::find_duplicate_filenames(&files);
    assert_eq!(count, 0);
    assert!(worst.is_none());
    assert!(duplicates.is_empty());
}

#[test]
fn test_inner_and_wrapper_excluded() {
    let funcs = vec![
        func("inner", "a.py", 5),
        func("inner", "b.py", 5),
        func("inner", "c.py", 5),
        func("wrapper", "a.py", 15),
        func("wrapper", "b.py", 15),
        func("wrapper", "c.py", 15),
    ];

    let (count, worst, collisions) = functions::find_function_collisions(&funcs);
    assert_eq!(count, 0);
    assert!(worst.is_none());
    assert!(collisions.is_empty());
}
