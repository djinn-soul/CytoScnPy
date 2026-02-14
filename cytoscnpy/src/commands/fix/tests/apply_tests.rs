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
        json_output: false,
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
        json_output: false,
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
fn test_run_fix_deadcode_dry_run_json_output() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def unused_function():\n    pass\n").unwrap();

    let def = create_definition("unused_function", "function", file_path, 1);
    let mut results = AnalysisResult::default();
    results.unused_functions.push(def);

    let options = DeadCodeFixOptions {
        min_confidence: 80,
        dry_run: true,
        json_output: true,
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

    let payload: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
    assert_eq!(payload["schema_version"], "2");
    assert_eq!(payload["kind"], "dead_code_fix_plan");
    assert_eq!(payload["planned_items"], 1);
    assert!(payload["plans"][0]["planned_edits"][0]["stable_id"].is_string());

    let temp_path = dir.path().to_string_lossy().to_string();
    let escaped_temp_path = temp_path.replace('\\', "\\\\");
    let normalized = String::from_utf8(buffer)
        .unwrap()
        .replace(&temp_path, "[TMP]")
        .replace(&escaped_temp_path, "[TMP]");
    insta::assert_snapshot!("dead_code_fix_plan_json", normalized);
}

#[test]
fn test_run_fix_deadcode_apply_json_output() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def unused_function():\n    pass\n").unwrap();

    let def = create_definition("unused_function", "function", file_path.clone(), 1);
    let mut results = AnalysisResult::default();
    results.unused_functions.push(def);

    let options = DeadCodeFixOptions {
        min_confidence: 80,
        dry_run: false,
        json_output: true,
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

    let payload: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
    assert_eq!(payload["schema_version"], "2");
    assert_eq!(payload["kind"], "dead_code_fix_report");
    assert_eq!(payload["applied_files"], 1);
    assert_eq!(payload["items_removed"], 1);
    assert!(payload["results"][0]["file"].is_string());

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.trim().is_empty());
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
