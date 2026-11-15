use super::*;

/// Applies every dead-code fix the analyzer finds in `source`.
fn fix_all(source: &str) -> String {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("app.py");
    std::fs::write(&file, source).unwrap();
    let mut analyzer = crate::analyzer::CytoScnPy::default();
    let scan = analyzer.analyze(&file);
    let items: Vec<_> = scan
        .unused_imports
        .iter()
        .map(|def| ("import", def))
        .chain(scan.unused_functions.iter().map(|def| ("function", def)))
        .chain(scan.unused_methods.iter().map(|def| ("method", def)))
        .chain(scan.unused_classes.iter().map(|def| ("class", def)))
        .collect();
    assert!(!items.is_empty(), "{source}");
    let options = DeadCodeFixOptions {
        analysis_root: dir.path().to_path_buf(),
        ..Default::default()
    };
    apply_dead_code_fix_to_file(&mut Vec::new(), &file, &items, &options)
        .unwrap()
        .unwrap();
    std::fs::read_to_string(&file).unwrap()
}

#[test]
fn removing_a_definition_keeps_the_surrounding_blank_line_spacing() {
    for (source, expected) in [
        // Between two definitions: the two-line gap stays two lines.
        (
            "def used(x):\n    return x\n\n\ndef unused():\n    return 1\n\n\ndef main():\n    print(used(3))\n\n\nmain()\n",
            "def used(x):\n    return x\n\n\ndef main():\n    print(used(3))\n\n\nmain()\n",
        ),
        // At the start of the file: no blank lines are left on top.
        (
            "import os\nimport json\n\n\ndef main():\n    print(1)\n\n\nmain()\n",
            "def main():\n    print(1)\n\n\nmain()\n",
        ),
        (
            "import os\n\n\ndef unused():\n    return 1\n\n\ndef main():\n    print(1)\n\n\nmain()\n",
            "def main():\n    print(1)\n\n\nmain()\n",
        ),
        // At the end of the file: no blank lines are left at the bottom.
        (
            "def main():\n    print(1)\n\n\nmain()\n\n\ndef unused():\n    return 1\n",
            "def main():\n    print(1)\n\n\nmain()\n",
        ),
        // The last method of a class: the larger gap after it is kept.
        (
            "class A:\n    def f(self):\n        return 1\n\n    def unused(self):\n        return 2\n\n\nprint(A().f())\n",
            "class A:\n    def f(self):\n        return 1\n\n\nprint(A().f())\n",
        ),
        // Every line removed: nothing is left.
        ("import os\nimport sys\n", ""),
    ] {
        assert_eq!(fix_all(source), expected, "{source}");
    }
}

#[test]
fn partial_line_deletions_keep_their_line() {
    // The import shares its line with code, so only the import goes.
    let fixed = fix_all("import os; print('hello')\n\n\nprint('bye')\n");
    assert_eq!(fixed, "print('hello')\n\n\nprint('bye')\n");
}
