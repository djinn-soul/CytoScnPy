use super::*;

#[test]
fn removing_all_imports_preserves_required_suites_and_semicolons() {
    for (source, expected) in [
        (
            "def run():\n    import os\n\nrun()\n",
            "def run():\n    pass",
        ),
        (
            "def run(): import os; import sys\nrun()\n",
            "def run(): pass",
        ),
        ("import os; print('hello')\n", "print('hello')"),
        ("print('hello'); import os; import sys\n", "print('hello')"),
        (
            "if condition:\n    import os\nelse:\n    import sys\n",
            "pass",
        ),
        (
            "try:\n    import os\nexcept Exception:\n    import sys\n",
            "pass",
        ),
        (
            "for item in items:\n    import os\nelse:\n    import sys\n",
            "pass",
        ),
        ("with resource:\n    import os\n", "pass"),
        ("match value:\n    case 1:\n        import os\n", "pass"),
        ("import os\nimport sys\n", ""),
    ] {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("app.py");
        std::fs::write(&file, source).unwrap();
        let mut analyzer = crate::analyzer::CytoScnPy::default();
        let scan = analyzer.analyze(&file);
        let items: Vec<_> = scan
            .unused_imports
            .iter()
            .map(|def| ("import", def))
            .collect();
        assert!(!items.is_empty(), "{source}");
        let options = DeadCodeFixOptions {
            analysis_root: dir.path().to_path_buf(),
            dry_run: true,
            json_output: true,
            ..Default::default()
        };
        let preview = apply_dead_code_fix_to_file(&mut Vec::new(), &file, &items, &options)
            .unwrap()
            .unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), source);
        let mut rewriter = crate::fix::ByteRangeRewriter::new(source);
        rewriter.add_edits(preview.planned_edits.unwrap().into_iter().map(|edit| {
            crate::fix::Edit::new(
                edit.start_byte,
                edit.end_byte,
                edit.replacement.as_deref().unwrap_or(""),
            )
        }));
        let preview_source = rewriter.apply_verified().unwrap();
        let options = DeadCodeFixOptions {
            dry_run: false,
            ..options
        };
        let applied = apply_dead_code_fix_to_file(&mut Vec::new(), &file, &items, &options)
            .unwrap()
            .unwrap();
        let fixed = std::fs::read_to_string(&file).unwrap();
        assert_eq!(fixed, preview_source);
        assert!(fixed.contains(expected), "{fixed}");
        assert!(!fixed.contains("import"), "{fixed}");
        assert_eq!(applied.items_removed, items.len());
    }
}

#[test]
fn failed_fixes_preserve_files_and_propagate_error_context_in_json_mode() {
    for dry_run in [true, false] {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("app.py");
        let source = "used = 1\nprint(used)\n";
        std::fs::write(&file, source).unwrap();
        // A stale/corrupt finding must never silently disappear or produce a success report.
        let def = create_definition_with_range("unused", "variable", file.clone(), 1, 0, 100);
        let mut results = AnalysisResult::default();
        results.unused_variables.push(def);
        let options = DeadCodeFixOptions {
            fix_variables: true,
            json_output: true,
            dry_run,
            analysis_root: dir.path().to_path_buf(),
            ..Default::default()
        };
        let mut output = Vec::new();
        let error = run_fix_deadcode(&results, &options, &mut output)
            .err()
            .unwrap();
        let details = format!("{error:#}");
        assert!(details.contains("app.py"), "{details}");
        assert!(output.is_empty());
        assert_eq!(std::fs::read_to_string(file).unwrap(), source);
    }
}

#[test]
fn nested_definitions_inside_branches_are_located_by_exact_offset() {
    let source = "if enabled:\n    def same():\n        return 1\n    class Same:\n        pass\nelse:\n    def same():\n        return 2\n";
    let module = ruff_python_parser::parse_module(source)
        .unwrap()
        .into_syntax();
    let start = source.rfind("def same").unwrap();
    let range = find_def_range(&module.body, "same", "function", Some(start)).unwrap();
    assert_eq!(&source[range.0..range.1], "def same():\n        return 2");
    let start = source.find("class Same").unwrap();
    let range = find_def_range(&module.body, "Same", "class", Some(start)).unwrap();
    assert_eq!(&source[range.0..range.1], "class Same:\n        pass");
}

#[test]
fn partial_import_edits_preserve_semicolons() {
    for source in [
        "import sys, os; print(sys.version)\n",
        "import os, sys; print(sys.version)\n",
        "print('hello'); import sys, os\nprint(sys.version)\n",
    ] {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("app.py");
        std::fs::write(&file, source).unwrap();
        let mut analyzer = crate::analyzer::CytoScnPy::default();
        let scan = analyzer.analyze(&file);
        let options = DeadCodeFixOptions {
            fix_imports: true,
            analysis_root: dir.path().to_path_buf(),
            ..Default::default()
        };
        run_fix_deadcode(&scan, &options, &mut Vec::new()).unwrap();
        let fixed = std::fs::read_to_string(file).unwrap();
        assert!(fixed.contains("import sys"), "{fixed}");
        assert!(fixed.contains("print(sys.version)"), "{fixed}");
        assert!(ruff_python_parser::parse_module(&fixed).is_ok());
    }
}
