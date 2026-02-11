use super::*;

#[test]
fn test_run_fix_deadcode_dry_run() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let source = "
def unused_function():
    pass
";
    std::fs::write(&file_path, source).unwrap();

    let def = create_definition("unused_function", "function", file_path.clone(), 2);

    let mut results = AnalysisResult::default();
    results.unused_functions.push(def);

    let options = DeadCodeFixOptions {
        min_confidence: 60,
        dry_run: true,
        fix_functions: true,
        fix_methods: false,
        fix_classes: false,
        fix_imports: false,
        fix_variables: false,
        verbose: true,
        with_cst: false,
        analysis_root: dir.path().to_path_buf(),
    };

    let mut buffer = Vec::new();
    let fix_results = run_fix_deadcode(&results, &options, &mut buffer).unwrap();

    assert!(fix_results.is_empty());

    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("[DRY-RUN]"));
    assert!(output.contains("Would remove function 'unused_function'"));

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, source);
}

#[test]
fn test_run_fix_deadcode_apply() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let source = "
def unused_function():
    pass
";
    std::fs::write(&file_path, source).unwrap();

    let def = create_definition("unused_function", "function", file_path.clone(), 2);

    let mut results = AnalysisResult::default();
    results.unused_functions.push(def);

    let options = DeadCodeFixOptions {
        min_confidence: 60,
        dry_run: false,
        fix_functions: true,
        fix_methods: false,
        fix_classes: false,
        fix_imports: false,
        fix_variables: false,
        verbose: false,
        with_cst: false,
        analysis_root: dir.path().to_path_buf(),
    };

    let mut buffer = Vec::new();
    let fix_results = run_fix_deadcode(&results, &options, &mut buffer).unwrap();

    assert_eq!(fix_results.len(), 1);
    assert_eq!(fix_results[0].items_removed, 1);

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.trim().is_empty());
}

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

    let def = create_definition("unused", "method", file_path.clone(), 3);
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

    let def = create_definition("unused", "method", file_path.clone(), 2);
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
fn test_collect_items_to_fix() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let def = create_definition("test", "function", file_path.clone(), 1);

    let mut results = AnalysisResult::default();
    results.unused_functions.push(def);
    results
        .unused_classes
        .push(create_definition("Class", "class", file_path.clone(), 10));
    results
        .unused_methods
        .push(create_definition("method", "method", file_path.clone(), 15));
    results
        .unused_imports
        .push(create_definition("imp", "import", file_path, 20));

    let options = DeadCodeFixOptions {
        min_confidence: 60,
        fix_functions: true,
        fix_methods: true,
        fix_classes: true,
        fix_imports: true,
        fix_variables: false,
        ..DeadCodeFixOptions::default()
    };

    let collected = crate::commands::fix::plan::collect_items_to_fix(&results, &options);
    assert_eq!(collected.values().next().unwrap().len(), 4);
}

#[test]
fn test_collect_items_to_fix_respects_min_confidence_for_methods() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let mut method_low = create_definition("method_low", "method", file_path.clone(), 10);
    method_low.confidence = 79;
    let mut method_high = create_definition("method_high", "method", file_path.clone(), 20);
    method_high.confidence = 80;

    let mut results = AnalysisResult::default();
    results.unused_methods.push(method_low);
    results.unused_methods.push(method_high);

    let options = DeadCodeFixOptions {
        min_confidence: 80,
        fix_methods: true,
        ..DeadCodeFixOptions::default()
    };

    let collected = crate::commands::fix::plan::collect_items_to_fix(&results, &options);
    let items = collected.values().next().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].1.simple_name, "method_high");
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
