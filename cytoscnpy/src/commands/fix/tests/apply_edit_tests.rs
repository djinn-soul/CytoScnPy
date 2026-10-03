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

    let mut def = create_definition("unused_function", "function", file_path.clone(), 3);
    def.start_byte = source.find("unused_function").unwrap();
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
fn test_apply_dead_code_fix_removes_nested_definition_with_used_top_level_namesake() {
    for (item_type, source, signature) in [
        (
            "function",
            "def shared():\n    return 1\n\ndef outer():\n    def shared():\n        return 2\n    return 3\n\nprint(shared(), outer())\n",
            "def shared",
        ),
        (
            "class",
            "class Shared:\n    pass\n\ndef outer():\n    class Shared:\n        pass\n    return 3\n\nprint(Shared(), outer())\n",
            "class Shared",
        ),
    ] {
        let name = signature.split_whitespace().nth(1).unwrap();
        let source_start = source.rfind(signature).unwrap();
        let name_start = source_start + signature.find(name).unwrap();
        for start in [source_start, name_start] {
            let dir = TempDir::new().unwrap();
            let file_path = dir.path().join("test.py");
            std::fs::write(&file_path, source).unwrap();

            let end = source[start..].find('\n').unwrap() + start;
            let def = create_definition_with_range(name, item_type, file_path.clone(), 5, start, end);
            let options = DeadCodeFixOptions {
                dry_run: false,
                analysis_root: dir.path().to_path_buf(),
                ..DeadCodeFixOptions::default()
            };

            let mut buffer = Vec::new();
            let res = apply_dead_code_fix_to_file(
                &mut buffer,
                &file_path,
                &[(item_type, &def)],
                &options,
            )
            .unwrap();

            assert!(res.is_some(), "{item_type} at byte {start}");
            let content = std::fs::read_to_string(&file_path).unwrap();
            assert_eq!(content.matches(signature).count(), 1);
            assert!(content.contains("return 3"));
            assert!(content.contains("print("));
            assert!(ruff_python_parser::parse_module(&content).is_ok());
        }
    }
}

#[test]
fn test_apply_dead_code_fix_removes_import_alias_from_multi_import() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let source = "import a, b, c\nfrom mod import x, y, z\n";
    std::fs::write(&file_path, source).unwrap();

    let def_import = create_definition_with_range(
        "b",
        "import",
        file_path.clone(),
        1,
        source.find("b, c").unwrap(),
        source.find("b, c").unwrap() + 1,
    );
    let def_from = create_definition_with_range(
        "y",
        "import",
        file_path.clone(),
        2,
        source.find("y, z").unwrap(),
        source.find("y, z").unwrap() + 1,
    );

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
    let error =
        apply_dead_code_fix_to_file(&mut buffer, &file_path, &[("function", &def)], &options)
            .err()
            .unwrap();
    assert!(format!("{error:#}").contains("Failed to parse fix source"));
    assert_eq!(
        std::fs::read_to_string(&file_path).unwrap(),
        "invalid python code (((( ("
    );
}
