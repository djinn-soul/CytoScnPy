use super::*;

#[test]
fn test_apply_dead_code_fix_removes_decorators_and_keeps_valid_python() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let source = "
@decorator
def unused_function():
    pass

def used_function():
    pass
";
    std::fs::write(&file_path, source).unwrap();

    let def = create_definition("unused_function", "function", file_path.clone(), 3);
    let options = DeadCodeFixOptions {
        dry_run: false,
        fix_variables: false,
        analysis_root: dir.path().to_path_buf(),
        ..DeadCodeFixOptions::default()
    };

    let mut buffer = Vec::new();
    let res = apply_dead_code_fix_to_file(&mut buffer, &file_path, &[("function", &def)], &options)
        .unwrap();
    assert!(res.is_some());

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(!content.contains("@decorator"));
    assert!(content.contains("def used_function"));
    assert!(ruff_python_parser::parse_module(&content).is_ok());
}

#[test]
fn test_apply_dead_code_fix_removes_import_alias_from_multi_import() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let source = "import a, b, c\nfrom mod import x, y, z\n";
    std::fs::write(&file_path, source).unwrap();

    let def_import = create_definition("b", "import", file_path.clone(), 1);
    let def_from = create_definition("y", "import", file_path.clone(), 2);

    let options = DeadCodeFixOptions {
        dry_run: false,
        fix_imports: true,
        fix_variables: false,
        analysis_root: dir.path().to_path_buf(),
        ..DeadCodeFixOptions::default()
    };

    let mut buffer = Vec::new();
    let res = apply_dead_code_fix_to_file(
        &mut buffer,
        &file_path,
        &[("import", &def_import), ("import", &def_from)],
        &options,
    )
    .unwrap();
    assert!(res.is_some());

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("import a, c"));
    assert!(content.contains("from mod import x, z"));
}

#[test]
fn test_apply_dead_code_fix_replaces_unused_for_tuple_name() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let source = "for a, b, c in items:\n    print(a, c)\n";
    std::fs::write(&file_path, source).unwrap();

    let start = source.find('b').unwrap();
    let def = create_definition_with_range("b", "variable", file_path.clone(), 1, start, start + 1);

    let options = DeadCodeFixOptions {
        dry_run: false,
        fix_imports: false,
        fix_variables: true,
        analysis_root: dir.path().to_path_buf(),
        ..DeadCodeFixOptions::default()
    };

    let mut buffer = Vec::new();
    let res = apply_dead_code_fix_to_file(&mut buffer, &file_path, &[("variable", &def)], &options)
        .unwrap();
    assert!(res.is_some());

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("for a, _, c in items"));
    assert!(ruff_python_parser::parse_module(&content).is_ok());
}

#[test]
fn test_apply_dead_code_fix_removes_unused_method() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let source = "
class Service:
    def unused(self):
        return 1

    def used(self):
        return 2
";
    std::fs::write(&file_path, source).unwrap();

    let start = source.find("def unused").unwrap();
    let end = source[start..].find('\n').unwrap() + start;
    let def = create_definition_with_range("unused", "method", file_path.clone(), 3, start, end);
    let options = DeadCodeFixOptions {
        dry_run: false,
        analysis_root: dir.path().to_path_buf(),
        ..DeadCodeFixOptions::default()
    };

    let mut buffer = Vec::new();
    let res = apply_dead_code_fix_to_file(&mut buffer, &file_path, &[("method", &def)], &options)
        .unwrap();
    assert!(res.is_some());

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(!content.contains("def unused"));
    assert!(content.contains("def used"));
    assert!(ruff_python_parser::parse_module(&content).is_ok());
}

#[test]
fn test_apply_dead_code_fix_replaces_only_method_with_pass() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let source = "
class Service:
    def unused(self):
        return 1
";
    std::fs::write(&file_path, source).unwrap();

    let start = source.find("def unused").unwrap();
    let end = source[start..].find('\n').unwrap() + start;
    let def = create_definition_with_range("unused", "method", file_path.clone(), 2, start, end);
    let options = DeadCodeFixOptions {
        dry_run: false,
        analysis_root: dir.path().to_path_buf(),
        ..DeadCodeFixOptions::default()
    };

    let mut buffer = Vec::new();
    let res = apply_dead_code_fix_to_file(&mut buffer, &file_path, &[("method", &def)], &options)
        .unwrap();
    assert!(res.is_some());

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(!content.contains("def unused"));
    assert!(content.contains("class Service:\n    pass"));
    assert!(ruff_python_parser::parse_module(&content).is_ok());
}

#[test]
fn test_apply_dead_code_fix_removes_nested_method() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let source = "
class Outer:
    class Inner:
        def stale(self):
            return 1

        def keep(self):
            return 2
";
    std::fs::write(&file_path, source).unwrap();

    let start = source.find("def stale").unwrap();
    let end = source[start..].find('\n').unwrap() + start;
    let def = create_definition_with_range("stale", "method", file_path.clone(), 4, start, end);
    let options = DeadCodeFixOptions {
        dry_run: false,
        analysis_root: dir.path().to_path_buf(),
        ..DeadCodeFixOptions::default()
    };

    let mut buffer = Vec::new();
    let res = apply_dead_code_fix_to_file(&mut buffer, &file_path, &[("method", &def)], &options)
        .unwrap();
    assert!(res.is_some());

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(!content.contains("def stale"));
    assert!(content.contains("def keep"));
    assert!(ruff_python_parser::parse_module(&content).is_ok());
}

#[test]
fn test_apply_dead_code_fix_removes_decorated_methods() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let source = "
class Decorated:
    @staticmethod
    def stale_static():
        return 1

    @property
    def stale_prop(self):
        return 2

    @classmethod
    def stale_class(cls):
        return 3

    def keep(self):
        return 4
";
    std::fs::write(&file_path, source).unwrap();

    let start_static = source.find("@staticmethod").unwrap();
    let end_static = source[start_static..].find("def stale_static").unwrap() + start_static;
    let def_static = create_definition_with_range(
        "stale_static",
        "method",
        file_path.clone(),
        4,
        start_static,
        end_static,
    );
    let start_prop = source.find("@property").unwrap();
    let end_prop = source[start_prop..].find("def stale_prop").unwrap() + start_prop;
    let def_prop = create_definition_with_range(
        "stale_prop",
        "method",
        file_path.clone(),
        8,
        start_prop,
        end_prop,
    );
    let start_class = source.find("@classmethod").unwrap();
    let end_class = source[start_class..].find("def stale_class").unwrap() + start_class;
    let def_class = create_definition_with_range(
        "stale_class",
        "method",
        file_path.clone(),
        12,
        start_class,
        end_class,
    );
    let options = DeadCodeFixOptions {
        dry_run: false,
        analysis_root: dir.path().to_path_buf(),
        ..DeadCodeFixOptions::default()
    };

    let mut buffer = Vec::new();
    let res = apply_dead_code_fix_to_file(
        &mut buffer,
        &file_path,
        &[
            ("method", &def_static),
            ("method", &def_prop),
            ("method", &def_class),
        ],
        &options,
    )
    .unwrap();
    assert!(res.is_some());

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(!content.contains("stale_static"));
    assert!(!content.contains("stale_prop"));
    assert!(!content.contains("stale_class"));
    assert!(content.contains("def keep"));
    assert!(ruff_python_parser::parse_module(&content).is_ok());
}

#[test]
fn test_apply_fix_parse_error() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "invalid python code (((( (").unwrap();

    let def = create_definition("f", "function", file_path.clone(), 1);
    let options = DeadCodeFixOptions {
        analysis_root: dir.path().to_path_buf(),
        fix_variables: false,
        ..DeadCodeFixOptions::default()
    };

    let mut buffer = Vec::new();
    let res = apply_dead_code_fix_to_file(&mut buffer, &file_path, &[("function", &def)], &options)
        .unwrap();
    assert!(res.is_none());
    assert!(String::from_utf8(buffer).unwrap().contains("Parse error:"));
}
