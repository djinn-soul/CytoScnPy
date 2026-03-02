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
