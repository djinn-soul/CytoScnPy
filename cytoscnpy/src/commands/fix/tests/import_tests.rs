use super::*;

#[test]
fn import_fix_preserves_used_import_in_another_scope() {
    for (top_import, local_import) in [
        ("import os", "import os"),
        ("from os import name", "from os import name"),
        ("import os as system", "import os as system"),
    ] {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("app.py");
        let binding = if top_import.contains("system") {
            "system"
        } else if top_import.starts_with("from") {
            "name"
        } else {
            "os"
        };
        let source = format!("{top_import}\nprint({binding})\ndef work():\n    {local_import}\n    return 1\nprint(work())\n");
        std::fs::write(&file, &source).unwrap();
        let mut analyzer = crate::analyzer::CytoScnPy::default();
        let result = analyzer.analyze(&file);
        assert_eq!(result.unused_imports.len(), 1);
        assert_eq!(result.unused_imports[0].line, 4);
        let options = DeadCodeFixOptions {
            dry_run: false,
            fix_imports: true,
            analysis_root: dir.path().to_path_buf(),
            ..DeadCodeFixOptions::default()
        };
        let applied = apply_dead_code_fix_to_file(
            &mut Vec::new(),
            &file,
            &[("import", &result.unused_imports[0])],
            &options,
        )
        .unwrap();
        assert!(applied.is_some());
        let fixed = std::fs::read_to_string(&file).unwrap();
        assert!(fixed.starts_with(top_import));
        assert!(!fixed.contains(&format!("    {local_import}")));
        assert!(ruff_python_parser::parse_module(&fixed).is_ok());
    }
}

#[test]
fn import_fix_uses_exact_offset_inside_control_flow() {
    use crate::commands::fix::import_plan::plan_import_edits;
    let source = "import os\nif enabled:\n    import os\n";
    let parsed = ruff_python_parser::parse_module(source).unwrap();
    let offset = source.rfind("os").unwrap();
    let mut def = create_definition_with_range(
        "os",
        "import",
        PathBuf::from("app.py"),
        3,
        offset,
        offset + 2,
    );
    let planned = plan_import_edits(&[("import", &def)], parsed.syntax(), source);
    assert_eq!(planned.len(), 1);
    assert_eq!(
        &source[planned[0].start_byte..planned[0].end_byte],
        "import os"
    );
    assert!(planned[0].start_byte > source.find("if enabled").unwrap());
    def.start_byte += 1;
    assert!(plan_import_edits(&[("import", &def)], parsed.syntax(), source).is_empty());
}

#[test]
fn test_apply_dead_code_fix_removes_alias_from_import_as() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    let source = "import a as x, b, c as z\n";
    std::fs::write(&file_path, source).unwrap();

    let def = create_definition_with_range(
        "x",
        "import",
        file_path.clone(),
        1,
        source.find("a as x").unwrap(),
        13,
    );

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

    let def = create_definition_with_range(
        "b_alias",
        "import",
        file_path.clone(),
        3,
        source.find("b as b_alias").unwrap(),
        42,
    );

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

    let def = create_definition_with_range(
        "b",
        "import",
        file_path.clone(),
        3,
        source.find("    b").unwrap() + 4,
        29,
    );

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
