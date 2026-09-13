use super::*;

#[test]
fn test_find_def_range_function() {
    let source = "
def used(): pass

def unused():
    pass
";
    let parsed = ruff_python_parser::parse_module(source).unwrap();
    let body = parsed.into_syntax().body;

    let range = find_def_range(&body, "unused", "function", source.find("def unused"));
    assert!(range.is_some());
    let (start, _end) = range.unwrap();
    assert!(start > 15);
}

#[test]
fn test_find_def_range_function_includes_decorators() {
    let source = "
@decorator
def unused():
    pass
";
    let parsed = ruff_python_parser::parse_module(source).unwrap();
    let body = parsed.into_syntax().body;

    let range = find_def_range(&body, "unused", "function", source.find("unused")).unwrap();
    assert_eq!(range.0, source.find('@').unwrap());
}

#[test]
fn test_find_def_range_class() {
    let source = "
class Used: pass

class Unused:
    pass
";
    let parsed = ruff_python_parser::parse_module(source).unwrap();
    let body = parsed.into_syntax().body;

    let range = find_def_range(&body, "Unused", "class", source.find("class Unused"));
    assert!(range.is_some());
}

#[test]
fn test_find_def_range_class_includes_decorators() {
    let source = "
@decorator
class Unused:
    pass
";
    let parsed = ruff_python_parser::parse_module(source).unwrap();
    let body = parsed.into_syntax().body;

    let range = find_def_range(&body, "Unused", "class", source.find("Unused")).unwrap();
    assert_eq!(range.0, source.find('@').unwrap());
}

#[test]
fn test_find_def_range_import() {
    let source = "
import used
import unused
";
    let parsed = ruff_python_parser::parse_module(source).unwrap();
    let body = parsed.into_syntax().body;

    let range = find_def_range(&body, "unused", "import", None);
    assert!(range.is_some());
}

#[test]
fn test_find_def_range_import_from_multi() {
    let source = "from mod import a, b, c";
    let parsed = ruff_python_parser::parse_module(source).unwrap();
    let body = parsed.into_syntax().body;

    let range = find_def_range(&body, "a", "import", None);
    assert!(range.is_none());
}

#[test]
fn test_find_def_range_method() {
    let source = "
class Service:
    def used(self): pass

    def unused(self):
        pass
";
    let parsed = ruff_python_parser::parse_module(source).unwrap();
    let body = parsed.into_syntax().body;

    let range = find_def_range(&body, "unused", "method", None);
    assert!(range.is_some());
}

#[test]
fn test_find_def_range_method_includes_decorators() {
    let source = "
class Service:
    @classmethod
    def unused(cls):
        pass
";
    let parsed = ruff_python_parser::parse_module(source).unwrap();
    let body = parsed.into_syntax().body;

    let range = find_def_range(&body, "unused", "method", None).unwrap();
    assert_eq!(range.0, source.find('@').unwrap());
}

#[test]
fn test_find_def_range_uses_source_position_for_duplicate_names() {
    let source = "
def duplicate():
    return 1

def duplicate():
    return 2
";
    let parsed = ruff_python_parser::parse_module(source).unwrap();
    let body = parsed.into_syntax().body;
    let second_name = source.rfind("duplicate").unwrap();

    let range = find_def_range(&body, "duplicate", "function", Some(second_name)).unwrap();

    assert_eq!(&source[range.0..range.1], "def duplicate():\n    return 2");
}

#[test]
fn test_find_def_range_nested_same_name() {
    let source = "def shared():\n    def shared():\n        return 2\n    return 1\n";
    let parsed = ruff_python_parser::parse_module(source).unwrap();
    let body = parsed.into_syntax().body;
    let outer_name = source.find("shared").unwrap();
    let inner_name = source.rfind("shared").unwrap();

    let inner_range = find_def_range(&body, "shared", "function", Some(inner_name));
    assert!(
        inner_range.is_none(),
        "nested definition should not match outer function"
    );

    let outer_range = find_def_range(&body, "shared", "function", Some(outer_name)).unwrap();
    assert_eq!(
        &source[outer_range.0..outer_range.1],
        "def shared():\n    def shared():\n        return 2\n    return 1"
    );
}
