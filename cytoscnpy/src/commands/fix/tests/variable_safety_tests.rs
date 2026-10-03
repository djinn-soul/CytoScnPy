use super::*;

#[test]
fn discard_renames_preserve_existing_bindings_and_references() {
    for source in [
        "def run():\n    _ = 'keep'\n    unused = 'discard'\n    return _\n\nprint(run())\n",
        "_ = 'keep'\ndef run():\n    unused = 'discard'\n    return _\n\nprint(run())\n",
        "def run(_):\n    unused = 'discard'\n    return _\n\nprint(run('keep'))\n",
        "def run():\n    unused = 'discard'\n    return f'{_}'\n",
        "_ = 'keep'\n_unused = 1\n_unused_1 = 2\ndef run():\n    unused = 'discard'\n    return _, _unused, _unused_1\n",
        "_ = 'keep'\n_ｕｎｕｓｅｄ = 1\ndef run():\n    unused = 'discard'\n    return _, _unused\n",
    ] {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("app.py");
        std::fs::write(&file, source).unwrap();
        let start = source.find("unused = 'discard'").unwrap();
        let def = create_definition_with_range("unused", "variable", file.clone(), 3, start, start + 6);
        let options = DeadCodeFixOptions {
            analysis_root: dir.path().to_path_buf(),
            ..Default::default()
        };
        let result = apply_dead_code_fix_to_file(&mut Vec::new(), &file, &[("variable", &def)], &options)
            .unwrap().unwrap();
        assert_eq!(result.items_removed, 1);
        let fixed = std::fs::read_to_string(&file).unwrap();
        let replacement = fixed.lines().find(|line| line.contains("= 'discard'")).unwrap()
            .trim().split(" = ").next().unwrap();
        assert_ne!(replacement, "_", "{fixed}");
        // Every other statement, including the return expression, stays identical.
        assert_eq!(fixed.replace(replacement, "unused"), source.replace(replacement, "unused"));
        if source.contains("_ｕｎｕｓｅｄ") {
            assert_ne!(replacement, "_unused", "normalized Unicode binding must be reserved");
        }
    }
}

#[test]
fn discard_renames_in_one_pattern_have_distinct_bindings() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("app.py");
    let source = "match value:\n    case [left, right]:\n        print('matched')\n";
    std::fs::write(&file, source).unwrap();
    let left = source.find("left").unwrap();
    let right = source.find("right").unwrap();
    let defs = [
        create_definition_with_range("left", "variable", file.clone(), 2, left, left + 4),
        create_definition_with_range("right", "variable", file.clone(), 2, right, right + 5),
    ];
    let options = DeadCodeFixOptions {
        analysis_root: dir.path().to_path_buf(),
        ..Default::default()
    };
    apply_dead_code_fix_to_file(
        &mut Vec::new(),
        &file,
        &[("variable", &defs[0]), ("variable", &defs[1])],
        &options,
    )
    .unwrap();
    let fixed = std::fs::read_to_string(file).unwrap();
    assert!(ruff_python_parser::parse_module(&fixed).is_ok());
}
