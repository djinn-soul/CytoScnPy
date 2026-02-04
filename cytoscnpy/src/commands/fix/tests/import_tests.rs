use super::*;

#[test]
fn test_apply_dead_code_fix_removes_alias_from_import_as() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let source = "import a as x, b, c as z\n";
    std::fs::write(&file_path, source).unwrap();

    let def = create_definition("x", "import", file_path.clone(), 1);

    let options = DeadCodeFixOptions {
        dry_run: false,
        fix_imports: true,
        fix_variables: false,
        analysis_root: dir.path().to_path_buf(),
        ..DeadCodeFixOptions::default()
    };

    let mut buffer = Vec::new();
    let res = apply_dead_code_fix_to_file(&mut buffer, &file_path, &[("import", &def)], &options)
        .unwrap();
    assert!(res.is_some());

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("import b, c as z"));
}

#[test]
fn test_apply_dead_code_fix_removes_alias_from_parenthesized_from_import() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let source = "from mod import (\n    a,\n    b as b_alias,\n    c,\n)\n";
    std::fs::write(&file_path, source).unwrap();

    let def = create_definition("b_alias", "import", file_path.clone(), 3);

    let options = DeadCodeFixOptions {
        dry_run: false,
        fix_imports: true,
        fix_variables: false,
        analysis_root: dir.path().to_path_buf(),
        ..DeadCodeFixOptions::default()
    };

    let mut buffer = Vec::new();
    let res = apply_dead_code_fix_to_file(&mut buffer, &file_path, &[("import", &def)], &options)
        .unwrap();
    assert!(res.is_some());

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("from mod import (\n    a,\n    c,\n)\n"));
}

#[test]
fn test_apply_dead_code_fix_removes_last_parenthesized_import_without_trailing_comma() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let source = "from mod import (\n    a,\n    b\n)\n";
    std::fs::write(&file_path, source).unwrap();

    let def = create_definition("b", "import", file_path.clone(), 3);

    let options = DeadCodeFixOptions {
        dry_run: false,
        fix_imports: true,
        fix_variables: false,
        analysis_root: dir.path().to_path_buf(),
        ..DeadCodeFixOptions::default()
    };

    let mut buffer = Vec::new();
    let res = apply_dead_code_fix_to_file(&mut buffer, &file_path, &[("import", &def)], &options)
        .unwrap();
    assert!(res.is_some());

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("from mod import (\n    a\n)\n"));
}
