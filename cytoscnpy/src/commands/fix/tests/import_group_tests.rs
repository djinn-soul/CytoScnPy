use super::*;

fn fix_imports(source: &str, dry_run: bool) -> (String, Vec<FixResult>, serde_json::Value) {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("app.py");
    std::fs::write(&file, source).unwrap();
    let mut analyzer = crate::analyzer::CytoScnPy::default();
    let analysis = analyzer.analyze(&file);
    let options = DeadCodeFixOptions {
        dry_run,
        json_output: true,
        fix_imports: true,
        analysis_root: dir.path().to_path_buf(),
        ..DeadCodeFixOptions::default()
    };
    let mut output = Vec::new();
    let results = run_fix_deadcode(&analysis, &options, &mut output).unwrap();
    let fixed = std::fs::read_to_string(&file).unwrap();
    if !dry_run {
        assert!(ruff_python_parser::parse_module(&fixed).is_ok());
        assert!(analyzer.analyze(&file).unused_imports.is_empty());
    }
    (fixed, results, serde_json::from_slice(&output).unwrap())
}

#[test]
fn grouped_import_removals_remove_every_selected_alias() {
    for (source, kept) in [
        ("import os, sys\nprint('hello')\n", ""),
        ("from os import name, path\nprint('hello')\n", ""),
        ("import os, sys, math\nprint(math.pi)\n", "import math"),
        ("import os, sys, math\nprint(os.name)\n", "import os"),
        ("import os, sys, math\nprint(sys.version)\n", "import sys"),
        (
            "from os import (\n    name,\n    path,\n    sep,\n)\nprint(sep)\n",
            "sep,",
        ),
        (
            "from os import (name, path, sep)\nprint(name)\n",
            "from os import (name)",
        ),
        (
            "from os import (name, path, sep,)\nprint(name)\n",
            "from os import (name, )",
        ),
    ] {
        let (fixed, results, json) = fix_imports(source, false);
        assert!(fixed.contains(kept), "{fixed:?} must retain {kept:?}");
        assert_eq!(results[0].items_removed, 2);
        assert_eq!(results[0].removed_names.len(), 2);
        assert_eq!(json["items_removed"], 2);
    }
}

#[test]
fn dry_run_reports_all_definitions_with_one_edit_for_a_group() {
    let source = "import os, sys\nprint('hello')\n";
    let (unchanged, results, json) = fix_imports(source, true);
    assert_eq!(unchanged, source);
    assert_eq!(results[0].items_removed, 2);
    assert_eq!(results[0].removed_names, ["os", "sys"]);
    assert_eq!(results[0].planned_edits.as_ref().unwrap().len(), 1);
    assert_eq!(json["planned_items"], 2);
    let edit = &results[0].planned_edits.as_ref().unwrap()[0];
    let mut rewriter = crate::fix::ByteRangeRewriter::new(source);
    rewriter.add_edit(crate::fix::Edit::delete(edit.start_byte, edit.end_byte));
    let preview = rewriter.apply_verified().unwrap();
    let (applied, _, _) = fix_imports(source, false);
    assert_eq!(preview, applied);
}
